//! The `installables` scenario: two scripted models behind a `model` choice (table-manners 0.6 draft,
//! 3.1 per-choice readiness, 3.5 installables, 5.3 install and uninstall). State lives in files
//! under `<workspace>/installables/`; an install waits for `<workspace>/release-install` (or a
//! cancel) so tests decide when it ends. Nothing is downloaded.
use serde_json::{Value, json};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub const IDS: [&str; 2] = ["small", "large"];

fn dir(root: &Path) -> PathBuf {
    let dir = root.join("installables");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn state(root: &Path, id: &str) -> String {
    std::fs::read_to_string(dir(root).join(id))
        .map(|text| text.trim().to_owned())
        .unwrap_or_else(|_| if id == "small" { "installed" } else { "not_installed" }.into())
}
fn set(root: &Path, id: &str, value: &str) {
    std::fs::write(dir(root).join(id), value).unwrap();
}
fn outcome(root: &Path, id: &str, action: &str, state: &str, error: Value) {
    let at = "2026-11-09T12:00:00Z";
    std::fs::write(
        dir(root).join(format!("{id}.outcome")),
        json!({"action":action,"state":state,"error":error,"at":at}).to_string(),
    )
    .unwrap();
}
fn installable(root: &Path, id: &str) -> Value {
    let state = state(root, id);
    let mut item = json!({
        "id": id,
        "title": if id == "small" { "Small model" } else { "Large model" },
        "kind": "model",
        "download_bytes": if id == "small" { 120_000_000u64 } else { 3_400_000_000u64 },
        "installed_bytes": if id == "small" { 120_000_000u64 } else { 3_400_000_000u64 },
        "licence": if id == "small" { "Apache-2.0" } else { "Example Model License" },
        "state": state,
        "requirements": [{"kind":"file","name":"free disk","state":"present"}],
        "notes": if id == "small" { "Small and fast." } else { "Best on whole objects; large download." },
        "last_outcome": std::fs::read_to_string(dir(root).join(format!("{id}.outcome")))
            .ok().and_then(|text| serde_json::from_str::<Value>(&text).ok()).unwrap_or(Value::Null),
        "x-sample-lab": {"lab": {"remove": if id == "small" { 0.548 } else { 0.777 }, "best_for": if id == "small" { "small and fast" } else { "whole objects" }}}
    });
    if state == "installing" {
        item["progress"] = json!(0.25);
    }
    item
}
/// Adds the installables, the `model` choice and its per-choice requirements to the first action.
pub fn describe(root: &Path, mut document: Value) -> Value {
    document["installables"] = Value::Array(IDS.iter().map(|id| installable(root, id)).collect());
    let action = &mut document["actions"][0];
    action["params_schema"]["properties"]["model"] = json!({
        "type":"string","title":"Model","enum":IDS,"default":"small",
        "x-requirements":{"small":["model:small"],"large":["model:large"]}
    });
    let requirements: Vec<Value> = IDS.iter().map(|id| json!({
        "kind":"file","name":format!("model:{id}"),
        "state": if state(root, id) == "installed" { "present" } else { "missing" },
        "for_choice":{"param":"model","value":id},"installable":id,
        "setup":format!("Get the {id} model in the tool's setup.")
    })).collect();
    let default_ready = state(root, "small") == "installed";
    action["readiness"] = json!({
        "state": if default_ready { "ready" } else { "needs_setup" },
        "requirements": requirements
    });
    document
}
pub fn tools() -> [Value; 2] {
    let schema = json!({"type":"object","additionalProperties":false,"required":["id"],"properties":{"id":{"type":"string"}}});
    [
        json!({"name":"install","description":"Reserved install control","inputSchema":schema}),
        json!({"name":"uninstall","description":"Reserved uninstall control","inputSchema":schema}),
    ]
}
fn result(root: &Path, id: &str, error: Value) -> Value {
    let state = state(root, id);
    json!({
        "id": id, "state": if state == "installing" { "not_installed".into() } else { state.clone() },
        "installation": if state == "installed" { json!(format!("sha256:{id}")) } else { Value::Null },
        "error": error
    })
}
fn structured(value: Value) -> Value {
    json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false})
}
/// Replies on another thread so describe and uninstall stay answerable while an install waits.
pub fn call(root: PathBuf, output: Arc<Mutex<io::Stdout>>, message: Value) {
    let name = message["params"]["name"].as_str().unwrap_or("").to_owned();
    let id = message["params"]["arguments"]["id"].as_str().unwrap_or("").to_owned();
    let request = message["id"].clone();
    if !IDS.contains(&id.as_str()) {
        super::send(&output, json!({"jsonrpc":"2.0","id":request,"error":{"code":-32602,"message":"unknown installable",
            "data":{"reason":"unknown_installable","run":null,"installable":id}}}));
        return;
    }
    std::fs::write(root.join(format!("{name}-calls")), &id).ok();
    if name == "uninstall" {
        if state(&root, &id) == "installing" {
            std::fs::write(dir(&root).join(format!("{id}.cancel")), "").unwrap();
        } else if state(&root, &id) == "installed" {
            set(&root, &id, "not_installed");
            outcome(&root, &id, "uninstall", "succeeded", Value::Null);
        }
        let value = result(&root, &id, Value::Null);
        super::send(&output, json!({"jsonrpc":"2.0","id":request,"result":structured(value)}));
        return;
    }
    std::thread::spawn(move || {
        let before = state(&root, &id);
        set(&root, &id, "installing");
        let cancel = dir(&root).join(format!("{id}.cancel"));
        let release = root.join("release-install");
        let error = loop {
            if cancel.exists() {
                std::fs::remove_file(&cancel).ok();
                set(&root, &id, if before == "installed" { "installed" } else { "not_installed" });
                outcome(&root, &id, "install", "cancelled", Value::Null);
                break json!({"code":"cancelled","message":"Cancelled; nothing was changed."});
            }
            if std::fs::remove_file(&release).is_ok() {
                set(&root, &id, "installed");
                outcome(&root, &id, "install", "succeeded", Value::Null);
                break Value::Null;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        let value = result(&root, &id, error);
        super::send(&output, json!({"jsonrpc":"2.0","id":request,"result":structured(value)}));
    });
}
