use super::{Error, Result, db::Database};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Submission {
    pub id: String,
    pub tool: String,
    pub action: String,
    pub revision: String,
    pub params: Value,
    pub images: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub frames: BTreeMap<String, Vec<ImageFrame>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImageFrame {
    pub projection: Projection,
    pub topology: Topology,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    Flat,
    Equirectangular,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Topology {
    Bounded,
    Wrap,
}
impl Default for ImageFrame {
    fn default() -> Self {
        Self {
            projection: Projection::Flat,
            topology: Topology::Bounded,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Attempt {
    pub id: String,
    pub sequence: i64,
    pub created_ms: u64,
    pub request: Submission,
    pub arguments: Value,
    pub declaration: Value,
    pub inputs: Value,
    pub wire: Option<Value>,
    pub execution: String,
    pub result: Option<Value>,
    #[serde(default)]
    pub raw_response: Option<Value>,
    pub preservation: Value,
    pub outputs: Vec<Value>,
    pub journal: Option<Value>,
    pub notices: Vec<String>,
}
#[derive(Clone)]
pub struct AttemptStore {
    pub database: Database,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttemptSummary {
    pub id: String,
    pub sequence: i64,
    pub created_ms: u64,
    pub tool: String,
    pub action: String,
    pub execution: String,
}
impl AttemptStore {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
    pub async fn find(&self, submission: &Submission) -> Result<Option<Attempt>> {
        let id = submission.id.clone();
        let body = serde_json::to_string(submission)?;
        self.database
            .call(move |db| {
                let existing: Option<(String, String)> = db
                    .query_row(
                        "SELECT submission,record FROM attempts WHERE id=?1",
                        [id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .optional()?;
                existing
                    .map(|(saved, record)| {
                        if body != saved {
                            Err(Error::Conflict(
                                "This submission ID already has different inputs",
                            ))
                        } else {
                            Ok(serde_json::from_str(&record)?)
                        }
                    })
                    .transpose()
            })
            .await
    }
    pub async fn create(
        &self,
        submission: Submission,
        arguments: Value,
        declaration: Value,
        inputs: Value,
    ) -> Result<Attempt> {
        let mut attempt = Attempt {
            id: submission.id.clone(),
            sequence: 0,
            created_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            request: submission,
            arguments,
            declaration,
            inputs,
            wire: None,
            execution: "submitted".into(),
            result: None,
            raw_response: None,
            preservation: json!({"contract":"pending","native":"unsupported","report":"unsupported"}),
            outputs: vec![],
            journal: None,
            notices: vec![],
        };
        self.database
            .call(move |db| {
                let tx = db.transaction()?;
                tx.execute(
                    "INSERT INTO attempts(id,submission,record) VALUES(?1,?2,?3)",
                    params![
                        attempt.id,
                        serde_json::to_string(&attempt.request)?,
                        serde_json::to_string(&attempt)?
                    ],
                )?;
                attempt.sequence = tx.last_insert_rowid();
                for asset in attempt.request.images.values().flatten() {
                    tx.execute(
                        "INSERT OR IGNORE INTO attempt_assets(attempt_id,asset_id) VALUES(?1,?2)",
                        params![attempt.id, asset],
                    )?;
                }
                tx.execute(
                    "UPDATE attempts SET record=?1 WHERE id=?2",
                    params![serde_json::to_string(&attempt)?, attempt.id],
                )?;
                tx.commit()?;
                Ok(attempt)
            })
            .await
    }
    pub async fn get(&self, id: &str) -> Result<Attempt> {
        let id = id.to_owned();
        self.database
            .call(move |db| {
                let record: Option<String> = db
                    .query_row("SELECT record FROM attempts WHERE id=?1", [id], |row| {
                        row.get(0)
                    })
                    .optional()?;
                Ok(serde_json::from_str(
                    &record.ok_or(Error::Missing("Saved attempt not found"))?,
                )?)
            })
            .await
    }
    pub async fn update<F>(&self, id: &str, change: F) -> Result<Attempt>
    where
        F: FnOnce(&mut Attempt) -> Result<()> + Send + 'static,
    {
        let id = id.to_owned();
        self.database.call(move |db| {
            let tx = db.transaction()?;
            let record: String = tx.query_row("SELECT record FROM attempts WHERE id=?1",[&id],|row|row.get(0))?;
            let mut attempt: Attempt = serde_json::from_str(&record)?; change(&mut attempt)?;
            tx.execute("UPDATE attempts SET record=?1 WHERE id=?2",params![serde_json::to_string(&attempt)?,id])?;
            for output in &attempt.outputs { if let Some(asset) = output["asset"]["id"].as_str() { tx.execute("INSERT OR IGNORE INTO attempt_assets(attempt_id,asset_id) VALUES(?1,?2)",params![attempt.id,asset])?; } }
            tx.commit()?; Ok(attempt)
        }).await
    }
    /// Called by the transport before polling its actual write future.
    pub async fn record_wire(&self, id: &str, wire: Value) -> Result<()> {
        self.update(id, move |attempt| {
            if attempt.wire.is_some() || attempt.execution != "submitted" {
                return Err(Error::Conflict(
                    "Attempt was already dispatched or interrupted",
                ));
            }
            attempt.wire = Some(wire);
            attempt.execution = "running".into();
            Ok(())
        })
        .await?;
        Ok(())
    }
    pub async fn request_cancel(&self, id: &str) -> Result<Attempt> {
        self.update(id,|attempt| {
            if attempt.result.is_some() || !matches!(attempt.execution.as_str(),"submitted"|"running") {return Ok(());}
            if attempt.wire.is_none() {
                attempt.execution="rejected".into();
                attempt.notices.push("Cancelled before dispatch; no tool call was sent.".into());
            } else {
                attempt.execution="unconfirmed".into();
                attempt.notices.push("Cancellation requested; final outcome unconfirmed. This contract does not provide a structured run reference before the result.".into());
            }
            attempt.preservation["contract"]=json!("incomplete");Ok(())
        }).await
    }
    pub async fn bind_result(&self, id: &str, result: Value) -> Result<Attempt> {
        let id = id.to_owned();
        self.database.call(move |db| {
            let tx=db.transaction()?;
            let record:String=tx.query_row("SELECT record FROM attempts WHERE id=?1",[&id],|r|r.get(0))?;
            let mut attempt:Attempt=serde_json::from_str(&record)?;
            let reused:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM attempts WHERE id != ?1 AND json_extract(record,'$.request.tool') = ?2 AND json_extract(record,'$.declaration.session') = ?3 AND json_extract(record,'$.result.run.resource') = ?4)",params![id,attempt.request.tool,attempt.declaration["session"].as_str(),result["run"]["resource"].as_str()],|r|r.get(0))?;
            if reused || attempt.result.is_some() {return Err(Error::Conflict("Tool run identity is already bound to another result"));}
            attempt.execution=result["status"].as_str().unwrap_or("interrupted").into();attempt.result=Some(result);
            tx.execute("UPDATE attempts SET record=?1 WHERE id=?2",params![serde_json::to_string(&attempt)?,id])?;
            tx.commit()?;Ok(attempt)
        }).await
    }
    pub async fn history(
        &self,
        before: Option<i64>,
        limit: usize,
        tool: Option<String>,
        action: Option<String>,
    ) -> Result<Vec<AttemptSummary>> {
        let limit = limit.clamp(1, 100) as i64;
        self.database
            .call(move |db| {
                let mut query = db.prepare(
                    "SELECT id,seq,json_extract(record,'$.created_ms'),json_extract(record,'$.request.tool'),json_extract(record,'$.request.action'),json_extract(record,'$.execution') FROM attempts WHERE seq < ?1 AND (?3 IS NULL OR json_extract(record,'$.request.tool')=?3) AND (?4 IS NULL OR json_extract(record,'$.request.action')=?4) ORDER BY seq DESC LIMIT ?2",
                )?;
                let rows = query.query_map(params![before.unwrap_or(i64::MAX), limit, tool, action], |row| {
                    Ok(AttemptSummary { id: row.get(0)?, sequence: row.get(1)?, created_ms: row.get::<_, i64>(2)?.max(0) as u64, tool: row.get(3)?, action: row.get(4)?, execution: row.get(5)? })
                })?;
                rows.map(|row| Ok(row?)).collect()
            })
            .await
    }
    pub async fn recover(&self) -> Result<()> {
        self.database.call(|db| {
            let tx = db.transaction()?;
            let records = { let mut query=tx.prepare("SELECT id,record FROM attempts")?; query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()? };
            for (id,record) in records {
                let mut attempt: Attempt = serde_json::from_str(&record)?;
                if matches!(attempt.execution.as_str(),"submitted"|"running"|"cancelling"|"unconfirmed") { attempt.execution="interrupted".into(); attempt.notices.push("Host session ended. The tool's final outcome is unknown; this attempt was not retried.".into()); }
                if attempt.preservation["contract"] == "pending" { attempt.preservation["contract"] = json!("incomplete"); }
                tx.execute("UPDATE attempts SET record=?1 WHERE id=?2",params![serde_json::to_string(&attempt)?,id])?;
            }
            tx.commit()?;Ok(())
        }).await
    }
}
