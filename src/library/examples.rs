use super::{
    Error, Result,
    attempts::{Attempt, Submission},
    db::Database,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Serialize, Deserialize)]
pub struct Example {
    pub id: String,
    pub sequence: i64,
    pub name: String,
    pub notes: String,
    pub request: Submission,
    pub inputs: Value,
    pub declaration: Value,
    pub input_artifacts: Vec<Value>,
    pub workspace_dependent: bool,
}
#[derive(Serialize)]
pub struct ExampleSummary {
    pub workspace_dependent: bool,
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
        self.save_with_options(attempt_id, name, notes, false).await
    }
    /// Metadata transaction only; the coordinator verifies files under the storage lease first.
    pub async fn save_with_options(
        &self,
        attempt_id: String,
        name: String,
        notes: String,
        allow_workspace_dependent: bool,
    ) -> Result<Example> {
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
                let (input_artifacts,workspace_dependent)=native_inputs(&tx,&attempt,&attempt_id)?;
                if workspace_dependent && !allow_workspace_dependent {
                    return Err(Error::Conflict("Some native inputs are not preserved. Confirm saving a workspace-dependent example explicitly."));
                }
                let mut example = Example {
                    id,
                    sequence: 0,
                    name,
                    notes,
                    request: attempt.request,
                    inputs: attempt.inputs,
                    declaration: attempt.declaration,
                    input_artifacts,
                    workspace_dependent,
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
            let mut query=db.prepare("SELECT id,seq,json_extract(record,'$.name'),json_extract(record,'$.request.tool'),json_extract(record,'$.request.action'),json_extract(record,'$.workspace_dependent') FROM examples WHERE seq<?1 ORDER BY seq DESC LIMIT 20")?;
            let rows=query.query_map([before.unwrap_or(i64::MAX)],|r|Ok(ExampleSummary{id:r.get(0)?,sequence:r.get(1)?,name:r.get(2)?,tool:r.get(3)?,action:r.get(4)?,workspace_dependent:r.get(5)?}))?;
            rows.map(|row|Ok(row?)).collect()
        }).await
    }
}

fn native_inputs(
    db: &rusqlite::Connection,
    attempt: &Attempt,
    id: &str,
) -> Result<(Vec<Value>, bool)> {
    let mut snapshot = attempt
        .result
        .as_ref()
        .and_then(|result| result["artifacts"]["in"].as_array())
        .cloned()
        .unwrap_or_default();
    let mut dependent = false;
    for (port, bindings) in &attempt.request.bindings {
        let occurrences = snapshot
            .iter()
            .enumerate()
            .filter_map(|(index, item)| (item["port"] == *port).then_some(index))
            .collect::<Vec<_>>();
        for (index, binding) in bindings.iter().enumerate() {
            let super::attempts::InputBinding::Artifact { reference } = binding else {
                continue;
            };
            let linked = if let Some(occurrence) = occurrences.get(index) {
                // Owned bytes alone cannot make a foreign reference importable into this tool.
                let same_reference = snapshot[*occurrence]["artifact"] == *reference;
                let same_tool =
                    reference["tool"] == attempt.declaration["document"]["tool"]["name"];
                same_reference
                    && same_tool
                    && db.query_row(
                        "SELECT EXISTS(SELECT 1 FROM native_links WHERE owner_kind='attempt'
                     AND owner_id=?1 AND direction='in' AND occurrence=?2 AND artifact=?3)",
                        params![id, *occurrence as i64, serde_json::to_string(reference)?],
                        |row| row.get::<_, bool>(0),
                    )?
            } else {
                false
            };
            if let Some(occurrence) = occurrences.get(index) {
                snapshot[*occurrence]["native_state"] = json!(if linked {
                    "preserved"
                } else {
                    "workspace-dependent"
                });
            }
            dependent |= !linked;
        }
    }
    Ok((snapshot, dependent))
}
