//! Owned synthetic run records and per-session correlation claims; no transport or workers.
use super::v04_results::RunInfo;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug)]
pub struct Claim {
    id: String,
    owner: Arc<()>,
}
struct HeldClaim {
    owner: Arc<()>,
    reference: Option<Value>,
}
pub struct Runs {
    root: PathBuf,
    tool: String,
    session: String,
    claims: Mutex<HashMap<String, HeldClaim>>,
}
fn error(code: i64, message: impl std::fmt::Display) -> Value {
    json!({"code":code,"message":message.to_string()})
}
fn storage(error: impl std::fmt::Display) -> Value {
    self::error(-32603, error)
}
fn atomic(path: &Path, value: &Value) -> Result<(), Value> {
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(storage)?;
    serde_json::to_writer(&mut file, value).map_err(storage)?;
    file.persist_noclobber(path).map_err(storage)?;
    Ok(())
}
fn document(path: &Path) -> Result<Value, Value> {
    serde_json::from_slice(&fs::read(path).map_err(storage)?).map_err(storage)
}
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn timestamp(started: &str) -> Result<i128, Value> {
    OffsetDateTime::parse(started, &Rfc3339)
        .map(|time| time.unix_timestamp_nanos())
        .map_err(storage)
}
fn now() -> String {
    OffsetDateTime::now_utc().format(&Rfc3339).unwrap()
}
impl Runs {
    pub fn new(root: &Path, tool: &str, session: &str) -> Self {
        Self {
            root: root.into(),
            tool: tool.into(),
            session: session.into(),
            claims: Mutex::new(HashMap::new()),
        }
    }
    pub fn claim(&self, id: Option<&str>) -> Result<Option<Claim>, Value> {
        let Some(id) = id else { return Ok(None) };
        if id.is_empty()
            || id.len() > 128
            || !id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
        {
            return Err(error(-32602, "Invalid client_run_id"));
        }
        let mut claims = self.claims.lock().unwrap();
        if let Some(held) = claims.get(id) {
            return Err(
                json!({"code":-32602,"message":"client_run_id already claimed","data":{"reason":"client_run_id_reused","run":held.reference}}),
            );
        }
        let owner = Arc::new(());
        claims.insert(
            id.into(),
            HeldClaim {
                owner: owner.clone(),
                reference: None,
            },
        );
        Ok(Some(Claim {
            id: id.into(),
            owner,
        }))
    }
    /// Only a refused call with no run releases its claim. Queued cancellation does not call this.
    pub fn release(&self, claim: Option<&Claim>) {
        let mut claims = self.claims.lock().unwrap();
        if let Some(claim) = claim
            && claims.get(&claim.id).is_some_and(|held| {
                Arc::ptr_eq(&held.owner, &claim.owner) && held.reference.is_none()
            })
        {
            claims.remove(&claim.id);
        }
    }
    pub fn allocate(&self, action: &str, claim: Option<&Claim>) -> Result<RunInfo, Value> {
        let mut claims = self.claims.lock().unwrap();
        if let Some(claim) = claim
            && !claims.get(&claim.id).is_some_and(|held| {
                Arc::ptr_eq(&held.owner, &claim.owner) && held.reference.is_none()
            })
        {
            return Err(storage(
                "Allocation requires ownership of an unallocated claim",
            ));
        }
        let client = claim.map(|claim| claim.id.as_str());
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(storage)?;
        let id: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let directory = self.root.join("runs").join(&id);
        fs::create_dir_all(directory.parent().unwrap()).map_err(storage)?;
        fs::create_dir(&directory).map_err(storage)?;
        let started = now();
        let reference = json!({"id":id,"resource":format!("{}://run/{id}/run.json",self.tool),"session_id":self.session,"client_run_id":client});
        let mut entry = reference.clone();
        entry["action"] = json!(action);
        entry["status"] = json!("running");
        entry["started"] = json!(started);
        let write = || -> Result<(), Value> {
            let mut journal = fs::File::create(directory.join("journal")).map_err(storage)?;
            writeln!(journal, "{}", json!({"event":"allocated","at":started})).map_err(storage)?;
            // Publishing the entry makes the run and its already-readable journal discoverable.
            atomic(&directory.join("entry.json"), &entry)
        };
        if let Err(error) = write() {
            let _ = fs::remove_dir_all(&directory);
            return Err(error);
        }
        if let Some(client) = client {
            claims.get_mut(client).unwrap().reference = Some(reference.clone());
        }
        let mut reference = reference;
        reference["dir"] = json!(directory);
        Ok(RunInfo {
            tool: self.tool.clone(),
            root: self.root.clone(),
            reference,
            started,
        })
    }
    fn directory(&self, id: &str) -> Result<PathBuf, Value> {
        if !valid_id(id) {
            return Err(error(-32002, "Unknown run resource"));
        }
        Ok(self.root.join("runs").join(id))
    }
    pub fn read(&self, id: &str, part: &str) -> Result<String, Value> {
        if !["run.json", "journal"].contains(&part) {
            return Err(error(-32002, "Unknown run resource"));
        }
        let directory = self.directory(id)?;
        if !directory.join("entry.json").is_file() {
            return Err(error(-32002, "Run resource is not available"));
        }
        let text = fs::read_to_string(directory.join(part))
            .map_err(|_| error(-32002, "Run resource is not available"))?;
        if part == "run.json" {
            let mut result: Value = serde_json::from_str(&text).map_err(storage)?;
            // The four identity fields stay unchanged; only the current local locator is rebased.
            result["run"]["dir"] = json!(directory);
            Ok(result.to_string())
        } else {
            Ok(text)
        }
    }

    fn append(directory: &Path, event: &str) -> Result<(), Value> {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(directory.join("journal"))
            .map_err(storage)?;
        writeln!(file, "{}", json!({"event":event,"at":now()})).map_err(storage)
    }
    fn allocation(&self, info: &RunInfo) -> Result<(PathBuf, Value), Value> {
        let directory = self.directory(info.reference["id"].as_str().unwrap_or(""))?;
        let entry = document(&directory.join("entry.json"))?;
        let mut reference = entry.clone();
        for field in ["action", "status", "started"] {
            reference.as_object_mut().unwrap().remove(field);
        }
        reference["dir"] = json!(directory);
        if info.reference != reference
            || info.started != entry["started"]
            || info.root != self.root
            || info.tool != self.tool
        {
            return Err(storage("Run allocation does not belong to this record"));
        }
        Ok((directory, entry))
    }
    pub fn journal(&self, info: &RunInfo, event: &str) -> Result<(), Value> {
        let _guard = self.claims.lock().unwrap();
        let (directory, _) = self.allocation(info)?;
        if directory.join("run.json").exists() {
            return Err(storage("Run already finished"));
        }
        Self::append(&directory, event)
    }
    /// Returns the exact saved result; an idempotent repeat must pass that returned value.
    pub fn finish(&self, info: &RunInfo, result: &Value) -> Result<Value, Value> {
        let _guard = self.claims.lock().unwrap();
        let (directory, entry) = self.allocation(info)?;
        if result["run"] != info.reference
            || result["action"] != entry["action"]
            || result["started"] != entry["started"]
            || !result["notes"].is_array()
            || !["completed", "error", "cancelled", "invalid"]
                .contains(&result["status"].as_str().unwrap_or(""))
        {
            return Err(storage("Result does not match the allocated run"));
        }
        let destination = directory.join("run.json");
        if destination.exists() {
            return if document(&destination)? == *result {
                Ok(result.clone())
            } else {
                Err(storage("Completed run is immutable"))
            };
        }
        // This journal event records a publication attempt. A failed write may leave it behind
        // and a retry may repeat it; only run.json establishes terminal status. No crash recovery.
        let mut saved = result.clone();
        if let Err(error) = Self::append(&directory, result["status"].as_str().unwrap()) {
            let reason = format!(
                "Run journal could not be updated: {}",
                error["message"].as_str().unwrap()
            );
            saved["notes"].as_array_mut().unwrap().push(json!(reason));
            if saved["status"] == "completed" {
                saved["status"] = json!("error");
                saved["error"] = json!({"code":"io","retryable":false,"reason":reason});
            }
        }
        // A broken journal cannot suppress writable final evidence. If the final store itself
        // fails, the caller reports a server storage failure and retains the allocated claim.
        atomic(&destination, &saved)?;
        Ok(saved)
    }
    pub fn list(&self, cursor: Option<&str>) -> Result<Value, Value> {
        let position = cursor
            .map(|cursor| {
                let invalid = || error(-32602, "Invalid runs cursor");
                if cursor.is_empty()
                    || cursor.len() > 256
                    || !cursor
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._~-".contains(&b))
                {
                    return Err(invalid());
                }
                let bytes = URL_SAFE_NO_PAD.decode(cursor).map_err(|_| invalid())?;
                let (started, id): (String, String) =
                    serde_json::from_slice(&bytes).map_err(|_| invalid())?;
                if !valid_id(&id) {
                    return Err(invalid());
                }
                Ok((timestamp(&started).map_err(|_| invalid())?, id))
            })
            .transpose()?;
        let entries = match fs::read_dir(self.root.join("runs")) {
            Ok(entries) => Some(entries),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(storage(e)),
        };
        let mut ordered = vec![];
        for item in entries.into_iter().flatten() {
            let item = item.map_err(storage)?;
            let id = item.file_name().to_string_lossy().into_owned();
            if !valid_id(&id) || !item.path().join("entry.json").exists() {
                continue;
            }
            let mut entry = document(&item.path().join("entry.json"))?;
            let key = (
                timestamp(
                    entry["started"]
                        .as_str()
                        .ok_or_else(|| storage("Invalid stored start time"))?,
                )?,
                id,
            );
            if position.as_ref().is_some_and(|position| &key >= position) {
                continue;
            }
            if item.path().join("run.json").exists() {
                entry["status"] = document(&item.path().join("run.json"))?["status"].clone();
            }
            ordered.push((key, entry));
        }
        ordered.sort_by(|a, b| b.0.cmp(&a.0));
        let next = if ordered.len() > 2 {
            let last = &ordered[1].1;
            json!(
                URL_SAFE_NO_PAD
                    .encode(serde_json::to_vec(&json!([last["started"], last["id"]])).unwrap())
            )
        } else {
            Value::Null
        };
        let entries: Vec<Value> = ordered
            .into_iter()
            .take(2)
            .map(|(_, entry)| entry)
            .collect();
        Ok(
            json!({"contract":"0.6.0","tool":{"name":self.tool,"version":"0.1.0"},"runs":entries,"next_cursor":next}),
        )
    }
}
