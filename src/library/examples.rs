use super::{
    Error, Result,
    attempts::{Attempt, Submission},
    db::Database,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
pub struct Example {
    pub id: String,
    pub sequence: i64,
    pub name: String,
    pub notes: String,
    pub request: Submission,
    pub inputs: Value,
    pub declaration: Value,
}
#[derive(Serialize)]
pub struct ExampleSummary {
    pub id: String,
    pub sequence: i64,
    pub name: String,
    pub tool: String,
    pub action: String,
}
#[derive(Clone)]
pub struct ExampleStore(pub Database);
impl ExampleStore {
    pub async fn save(&self, attempt_id: String, name: String, notes: String) -> Result<Example> {
        let name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 120 || notes.chars().count() > 4000 {
            return Err(Error::Conflict(
                "Use a name of 1–120 characters and notes up to 4,000 characters",
            ));
        }
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(std::io::Error::other)?;
        let id: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        self.0
            .call(move |db| {
                let tx = db.transaction()?;
                let record: Option<String> = tx
                    .query_row(
                        "SELECT record FROM attempts WHERE id=?1",
                        [&attempt_id],
                        |r| r.get(0),
                    )
                    .optional()?;
                let attempt: Attempt = serde_json::from_str(
                    &record.ok_or(Error::Missing("Saved attempt not found"))?,
                )?;
                let mut example = Example {
                    id,
                    sequence: 0,
                    name,
                    notes,
                    request: attempt.request,
                    inputs: attempt.inputs,
                    declaration: attempt.declaration,
                };
                tx.execute(
                    "INSERT INTO examples(id,record) VALUES(?1,?2)",
                    params![example.id, serde_json::to_string(&example)?],
                )?;
                example.sequence = tx.last_insert_rowid();
                for (_, _, asset) in example.request.bound_images() {
                    tx.execute(
                        "INSERT OR IGNORE INTO example_assets(example_id,asset_id) VALUES(?1,?2)",
                        params![example.id, asset],
                    )?;
                }
                tx.execute("INSERT INTO native_links(owner_kind,owner_id,direction,occurrence,sha256,media_type,artifact) SELECT 'example',?1,direction,occurrence,sha256,media_type,artifact FROM native_links WHERE owner_kind='attempt' AND owner_id=?2 AND direction='in'",params![example.id,attempt_id])?;
                tx.execute(
                    "UPDATE examples SET record=?1 WHERE id=?2",
                    params![serde_json::to_string(&example)?, example.id],
                )?;
                tx.commit()?;
                Ok(example)
            })
            .await
    }
    pub async fn get(&self, id: String) -> Result<Example> {
        self.0
            .call(move |db| {
                let record: Option<String> = db
                    .query_row("SELECT record FROM examples WHERE id=?1", [id], |r| {
                        r.get(0)
                    })
                    .optional()?;
                Ok(serde_json::from_str(
                    &record.ok_or(Error::Missing("Example not found"))?,
                )?)
            })
            .await
    }
    pub async fn list(&self, before: Option<i64>) -> Result<Vec<ExampleSummary>> {
        self.0.call(move |db| {
            let mut query=db.prepare("SELECT id,seq,json_extract(record,'$.name'),json_extract(record,'$.request.tool'),json_extract(record,'$.request.action') FROM examples WHERE seq<?1 ORDER BY seq DESC LIMIT 20")?;
            let rows=query.query_map([before.unwrap_or(i64::MAX)],|r|Ok(ExampleSummary{id:r.get(0)?,sequence:r.get(1)?,name:r.get(2)?,tool:r.get(3)?,action:r.get(4)?}))?;
            rows.map(|row|Ok(row?)).collect()
        }).await
    }
}
