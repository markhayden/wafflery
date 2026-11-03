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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bindings: BTreeMap<String, Vec<InputBinding>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lineage: BTreeMap<String, Vec<Option<BindingSource>>>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputBinding {
    Image {
        asset: String,
        #[serde(default)]
        frame: ImageFrame,
    },
    Artifact {
        reference: Value,
    },
    Location {
        value: Value,
    },
}
impl Submission {
    /// Every owned image occurrence, including repeated assets and mixed operation lists.
    pub fn bound_images(&self) -> Vec<(&str, usize, &str)> {
        self.images
            .iter()
            .flat_map(|(slot, images)| {
                images
                    .iter()
                    .enumerate()
                    .map(move |(index, id)| (slot.as_str(), index, id.as_str()))
            })
            .chain(self.bindings.iter().flat_map(|(slot, items)| {
                items
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, item)| match item {
                        InputBinding::Image { asset, .. } => {
                            Some((slot.as_str(), index, asset.as_str()))
                        }
                        _ => None,
                    })
            }))
            .collect()
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BindingSource {
    pub attempt: String,
    pub port: String,
    pub occurrence: usize,
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
    #[serde(default)]
    pub observations: Value,
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
        self.create_with_observations(submission, arguments, declaration, inputs, json!({}))
            .await
    }
    pub(crate) async fn create_with_observations(
        &self,
        submission: Submission,
        arguments: Value,
        declaration: Value,
        inputs: Value,
        observations: Value,
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
            observations,
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
                for (_, _, asset) in attempt.request.bound_images() {
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
                attempt.notices.push("Cancellation requested; final outcome unconfirmed. Wafflery will look for an associated run; missing evidence does not prove the tool stopped.".into());
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
            let records = { let mut query=tx.prepare("SELECT id,record FROM attempts WHERE json_extract(record,'$.execution') IN ('submitted','running','cancelling','unconfirmed') OR json_extract(record,'$.preservation.contract')='pending' OR json_extract(record,'$.preservation.native')='preserving'")?; query.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()? };
            for (id,record) in records {
                let mut attempt: Attempt = serde_json::from_str(&record)?;
                if matches!(attempt.execution.as_str(),"submitted"|"running"|"cancelling"|"unconfirmed") { attempt.execution="interrupted".into(); attempt.notices.push("Host session ended. The tool's final outcome is unknown; this attempt was not retried.".into()); }
                if attempt.preservation["contract"] == "pending" { attempt.preservation["contract"] = json!("incomplete"); }
                if attempt.preservation["native"]=="preserving"{attempt.preservation["native"]=json!("failed");if let Some(values)=attempt.observations["native_artifacts"].as_array_mut(){for value in values{if value["state"]=="preserving"{value["state"]=json!("failed");value["reason"]=json!("Host session ended during native preservation. Retry preservation explicitly; the original action will not be replayed.");}}}}

                tx.execute("UPDATE attempts SET record=?1 WHERE id=?2",params![serde_json::to_string(&attempt)?,id])?;
            }
            tx.commit()?;Ok(())
        }).await
    }
}
