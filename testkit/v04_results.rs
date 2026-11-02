//! Synthetic operation outcomes. The wire layer owns run allocation, journals and cancellation.
use super::{v04_bindings::Prepared, v04_fixtures as fixtures, v04_store::Store};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

pub struct RunInfo {
    pub tool: String,
    pub root: PathBuf,
    pub reference: Value,
    pub started: String,
}
pub enum Outcome {
    Completed,
    Cancelled,
    Error { code: &'static str, reason: String },
}
fn digest(value: &Value) -> String {
    let hash: String = Sha256::digest(serde_json::to_vec(value).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!("sha256:{hash}")
}
fn base(info: &RunInfo, action: &str, invocation: Value, input_identity: String) -> Value {
    json!({"contract":"0.4.0","tool":{"name":info.tool,"version":"0.1.0"},"action":action,
        "run":info.reference,"status":"completed","error":null,"started":info.started,"finished":info.started,"duration_ms":0,
        "input_identity":input_identity,"invocation":invocation,
        "spend":{"usd":0,"unresolved_usd":0,"basis":"measured"},"cache":"off","notes":[],
        "images":{"in":[],"out":[]},"locations":{"used":[],"found":[]},"artifacts":{"in":[],"out":[]},"body":{"illustrative":true}})
}
pub fn settle(run: &mut Value, outcome: Outcome, duration: Duration) {
    let (status, error) = match outcome {
        Outcome::Completed => ("completed", Value::Null),
        Outcome::Cancelled => ("cancelled", Value::Null),
        Outcome::Error { code, reason } => (
            "error",
            json!({"code":code,"retryable":false,"reason":reason}),
        ),
    };
    run["status"] = json!(status);
    run["error"] = error;
    finish(run, duration);
}
/// Update the elapsed time without changing a terminal outcome or its evidence.
pub fn finish(run: &mut Value, duration: Duration) {
    let started = time::OffsetDateTime::parse(
        run["started"].as_str().unwrap(),
        &time::format_description::well_known::Rfc3339,
    )
    .expect("run allocation supplies an RFC3339 timestamp");
    let elapsed = time::Duration::try_from(duration).expect("synthetic run duration fits");
    run["finished"] = json!(
        (started + elapsed)
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    );
    run["duration_ms"] = json!(u64::try_from(duration.as_millis()).unwrap_or(u64::MAX));
}

pub fn invalid(info: &RunInfo, action: &str, arguments: &Value, reason: &str) -> Value {
    let mut arguments = arguments.clone();
    if let Some(arguments) = arguments.as_object_mut() {
        arguments.remove("return_images");
        arguments.remove("image_blocks");
    }
    let mut run = base(
        info,
        action,
        Value::Null,
        digest(&json!({"action":action,"arguments":arguments})),
    );
    settle(&mut run, Outcome::Completed, Duration::ZERO);
    run["status"] = json!("invalid");
    run["error"] = json!({"code":"input","retryable":false,"reason":reason});
    run
}
fn store_failure(error: Value) -> Outcome {
    Outcome::Error {
        code: if error["code"] == -32602 {
            "internal"
        } else {
            "io"
        },
        reason: error["message"]
            .as_str()
            .unwrap_or("Artifact storage failed")
            .into(),
    }
}
fn io_failure(error: impl std::fmt::Display) -> Outcome {
    Outcome::Error {
        code: "io",
        reason: error.to_string(),
    }
}
fn preview(store: &Store, root: &Path, document: &Value, delivery: &str) -> Result<Value, Outcome> {
    let mut preview = document["preview"].clone();
    if preview.is_null() || delivery == "resource" {
        return Ok(preview);
    }
    let bytes = store
        .preview(document["artifact"]["identity"].as_str().unwrap())
        .map_err(store_failure)?;
    preview.as_object_mut().unwrap().remove("resource");
    if delivery == "inline" {
        preview["inline"] = json!({"media_type":"image/png","data":STANDARD.encode(bytes)});
    } else {
        let directory = root.join("previews");
        std::fs::create_dir_all(&directory).map_err(io_failure)?;
        let destination = directory.join(format!(
            "{}.png",
            preview["image"]["file_sha256"].as_str().unwrap()
        ));
        let mut staged = tempfile::NamedTempFile::new_in(&directory).map_err(io_failure)?;
        staged.write_all(&bytes).map_err(io_failure)?;
        staged.persist(&destination).map_err(io_failure)?;
        preview["path"] = json!(destination);
    }
    Ok(preview)
}
fn outputs(tool: &str, prepared: &Prepared) -> Vec<(String, Value)> {
    match prepared.action["name"].as_str().unwrap() {
        "locate_marker" => vec![("marker".into(), fixtures::artifact(tool, "location"))],
        "stamp_marker" | "make_token" => vec![(
            "token".into(),
            fixtures::artifact(tool, &format!("{tool}:Token")),
        )],
        "soften_token" => vec![(
            "token".into(),
            fixtures::artifact_variant(
                tool,
                &format!("{tool}:Token"),
                prepared.arguments["amount"].as_f64().unwrap(),
            ),
        )],
        "collect_tokens" => prepared
            .inputs
            .iter()
            .filter(|i| i.port == "tokens")
            .map(|i| ("tokens".into(), i.document.clone()))
            .collect(),
        "summarize" => vec![(
            "memo".into(),
            fixtures::artifact(tool, &format!("{tool}:Memo")),
        )],
        _ => unreachable!("all declared synthetic actions have an executor"),
    }
}
fn append(run: &mut Value, profile: &str, field: &str, value: Value) {
    run[profile][field].as_array_mut().unwrap().push(value);
}
/// Construct resolved evidence without touching inputs or producing artifacts.
pub fn resolved(prepared: &Prepared, info: &RunInfo) -> Value {
    let invocation = json!({"action":prepared.action["name"],"arguments":prepared.arguments,
        "effective":{"model":null,"config_identity":digest(&json!({"defaults":"synthetic-v04"}))},"requirements":[]});
    base(
        info,
        prepared.action["name"].as_str().unwrap(),
        invocation.clone(),
        digest(&invocation),
    )
}
pub fn execute(
    store: &Store,
    prepared: &Prepared,
    info: &RunInfo,
    mut cancelled: impl FnMut() -> bool,
) -> Value {
    let mut run = resolved(prepared, info);
    let operation = prepared.action["kind"] == "operation";
    let execution = || -> Result<Outcome, Outcome> {
        for input in &prepared.inputs {
            if cancelled() {
                return Ok(Outcome::Cancelled);
            }
            store.remember(&input.document).map_err(store_failure)?;
            if input.document["artifact"]["type"] == "image" {
                append(
                    &mut run,
                    "images",
                    "in",
                    json!({"slot":input.port,"role":"source","image":input.document["record"]}),
                );
            }
            if input.document["artifact"]["type"] == "location" {
                append(
                    &mut run,
                    "locations",
                    "used",
                    input.document["record"].clone(),
                );
            }
            if operation {
                append(
                    &mut run,
                    "artifacts",
                    "in",
                    json!({"port":input.port,"artifact":input.document["artifact"],"preview":null}),
                );
            }
        }
        for (port, document) in outputs(&info.tool, prepared) {
            if cancelled() {
                return Ok(Outcome::Cancelled);
            }
            store.remember(&document).map_err(store_failure)?;
            let delivery = preview(store, &info.root, &document, &prepared.return_images);
            // The native output is already complete. A failed delivery must retain it with
            // its resource preview instead of dropping the artifact from the failed result.
            let preview = delivery
                .as_ref()
                .ok()
                .cloned()
                .unwrap_or_else(|| document["preview"].clone());
            if !preview.is_null() {
                append(&mut run, "images", "out", preview.clone());
            }
            if document["artifact"]["type"] == "location" {
                append(&mut run, "locations", "found", document["record"].clone());
            }
            append(
                &mut run,
                "artifacts",
                "out",
                json!({"port":port,"artifact":document["artifact"],"preview":preview}),
            );
            delivery?;
        }
        Ok(if cancelled() {
            Outcome::Cancelled
        } else {
            Outcome::Completed
        })
    }();
    settle(
        &mut run,
        execution.unwrap_or_else(|failure| failure),
        Duration::ZERO,
    );
    run
}
