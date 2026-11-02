//! Synthetic paid execution. Never connects to a provider or incurs a charge.
use super::{PeerState, estimates, fixtures, paid_scenarios, reply, send};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
fn now() -> String {
    OffsetDateTime::now_utc().format(&Rfc3339).unwrap()
}

pub fn execute(
    output: Arc<Mutex<io::Stdout>>,
    state: Arc<Mutex<PeerState>>,
    root: PathBuf,
    message: Value,
    cancelled: Arc<AtomicBool>,
) {
    if message["params"]["arguments"]["label"] == "paid-queued" {
        fs::write(root.join("queued"), "waiting before allocation").unwrap();
        while !cancelled.load(Ordering::Acquire) && !root.join("release-queue").exists() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    if cancelled.load(Ordering::Acquire) {
        let mut state = state.lock().unwrap();
        state.active.retain(|(id, _)| id != &message["id"]);
        fs::write(root.join("queue-cancelled"), "cancelled before allocation").unwrap();
        return;
    }
    let params = &message["params"];
    let arguments = &params["arguments"];
    let label = arguments["label"].as_str().unwrap_or("");
    let mut run = fixtures::result("running");
    let id = format!("run-{:04}", state.lock().unwrap().runs.len() + 1);
    let directory = root.join(&id);
    fs::create_dir_all(&directory).unwrap();
    run["action"] = json!("price_test");
    run["run"] = json!({"id":id,"resource":format!("{}://run/{id}/run.json",fixtures::TOOL),"dir":directory,
        "session_id":state.lock().unwrap().session["id"],"client_run_id":params["_meta"]["table-manners/client_run_id"]});
    run["started"] = json!(now());
    run["finished"] = Value::Null;
    let roots = state.lock().unwrap().allow_dirs.clone();
    let resolved = super::scenarios::prepare_images(arguments, &roots);
    let input_error = resolved.as_ref().err().map(ToString::to_string);
    run["images"]["in"] = resolved
        .map(|(images, _, _)| images)
        .unwrap_or_else(|_| json!([]));
    let mut effective =
        json!({"model":"offline-script","config_identity":format!("sha256:{}","1".repeat(64))});
    if label == "paid-stale" {
        effective["model"] = json!("changed-script");
    }
    run["notes"] = json!(["Synthetic spending observation; no provider or charge."]);
    super::result_file::publish(&directory.join("run.json"), &run).unwrap();
    state.lock().unwrap().runs.push(run.clone());
    writeln!(
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("calls.ndjson"))
            .unwrap(),
        "{params}"
    )
    .unwrap();
    let mut reference = run["run"].clone();
    reference.as_object_mut().unwrap().remove("dir");
    if let Some(token) = params["_meta"].get("progressToken") {
        send(
            &output,
            json!({"jsonrpc":"2.0","method":"notifications/progress","params":{
            "progressToken":token,"progress":1,"message":"Checking synthetic spending limits","_meta":{"table-manners/run":reference}}}),
        );
    }
    let session = state.lock().unwrap().session.clone();
    let binding = estimates::binding(arguments, &run["tool"], &effective, &session);
    let mut ledger = state.lock().unwrap();
    let stale = params["_meta"]
        .get("table-manners/estimate")
        .is_some_and(|requested| {
            binding
                .as_ref()
                .map_or(true, |actual| requested.as_str() != Some(actual))
                || requested
                    .as_str()
                    .and_then(|key| ledger.estimates.get(key))
                    .is_none_or(|estimate| {
                        OffsetDateTime::parse(estimate["expires"].as_str().unwrap(), &Rfc3339)
                            .unwrap()
                            <= OffsetDateTime::now_utc()
                    })
        });
    let cap = arguments["max_spend_usd"].as_f64().unwrap_or(0.0);
    let remaining = ledger.session["remaining_usd"].as_f64().unwrap();
    let error = if let Some(reason) = input_error.as_deref() {
        Some(("invalid", "input", reason))
    } else if stale {
        Some((
            "invalid",
            "stale_estimate",
            "The synthetic binding changed or expired",
        ))
    } else if binding.is_err() {
        Some(("invalid", "input", "Synthetic input is unavailable"))
    } else if cap < if label == "paid-cap" { 0.6 } else { 0.12 } {
        Some(("error", "cap", "Synthetic cost exceeds the cap"))
    } else if remaining + 1e-6 < 0.12 {
        Some((
            "error",
            "budget",
            "Synthetic budget cannot reserve the worst case",
        ))
    } else {
        None
    };
    if let Some((status, code, reason)) = error {
        run["status"] = json!(status);
        run["error"] = json!({"code":code,"reason":reason,"retryable":false});
    } else if cancelled.load(Ordering::Acquire) {
        run["status"] = json!("cancelled");
    } else {
        // Reserve before the simulated provider starts, then release the ledger mutex so
        // describe, estimates and resources remain available throughout held requests.
        ledger.session["reserved_usd"] =
            json!(ledger.session["reserved_usd"].as_f64().unwrap() + 0.12);
        paid_scenarios::balance(&mut ledger.session);
        drop(ledger);
        let mut progress = 1;
        paid_scenarios::settle(&root, &state, &cancelled, label, &mut run, || {
            if let Some(token) = params["_meta"].get("progressToken") {
                progress += 1;
                send(
                    &output,
                    json!({"jsonrpc":"2.0","method":"notifications/progress","params":{
                    "progressToken":token,"progress":progress,"message":"Waiting for synthetic provider",
                    "_meta":{"table-manners/run":reference}}}),
                );
            }
        });
        ledger = state.lock().unwrap();
        run["input_identity"] = json!(binding.unwrap());
    }
    run["finished"] = json!(now());
    fs::write(
        directory.join("journal.ndjson"),
        format!(
            "{}\n",
            json!({"step":1,"synthetic":true,"status":run["status"]})
        ),
    )
    .unwrap();
    super::result_file::publish(&directory.join("run.json"), &run).unwrap();
    *ledger
        .runs
        .iter_mut()
        .find(|saved| saved["run"]["id"] == id)
        .unwrap() = run.clone();
    ledger.active.retain(|(id, _)| id != &message["id"]);
    drop(ledger);
    if !cancelled.load(Ordering::Acquire) {
        reply(
            &output,
            &message["id"],
            json!({"structuredContent":run,"content":[],"isError":run["status"]!="completed"}),
        );
    }
}
