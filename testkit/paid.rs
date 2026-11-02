//! Synthetic paid execution. Never connects to a provider or incurs a charge.
use super::{PeerState, estimates, fixtures, reply, send};
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
    if cancelled.load(Ordering::Acquire) {
        state
            .lock()
            .unwrap()
            .active
            .retain(|(id, _)| id != &message["id"]);
        return;
    }
    let params = &message["params"];
    let arguments = &params["arguments"];
    let mut run = fixtures::result("running");
    let id = format!("run-{:04}", state.lock().unwrap().runs.len() + 1);
    let directory = root.join(&id);
    fs::create_dir_all(&directory).unwrap();
    run["action"] = json!("price_test");
    run["run"] = json!({"id":id,"resource":format!("{}://run/{id}/run.json",fixtures::TOOL),"dir":directory,
        "session_id":state.lock().unwrap().session["id"],"client_run_id":params["_meta"]["table-manners/client_run_id"]});
    run["started"] = json!(now());
    run["finished"] = Value::Null;
    run["images"]["in"] = json!([]);
    run["notes"] = json!(["Synthetic spending observation; no provider or charge."]);
    fs::write(directory.join("run.json"), run.to_string()).unwrap();
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
    let effective =
        json!({"model":"offline-script","config_identity":format!("sha256:{}","1".repeat(64))});
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
    let error = if stale {
        Some((
            "invalid",
            "stale_estimate",
            "The synthetic binding changed or expired",
        ))
    } else if binding.is_err() {
        Some(("invalid", "input", "Synthetic input is unavailable"))
    } else if cap < 0.12 {
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
        // The synthetic provider completes inside this atomic limit/settlement step.
        let spent = ledger.session["spent_usd"].as_f64().unwrap() + 0.08;
        ledger.session["spent_usd"] = json!(spent);
        ledger.session["remaining_usd"] =
            json!((ledger.session["budget_usd"].as_f64().unwrap() - spent).max(0.0));
        run["status"] = json!("completed");
        run["spend"]["usd"] = json!(0.08);
        run["input_identity"] = json!(binding.unwrap());
        run["images"]["in"]
            .as_array_mut()
            .unwrap()
            .push(json!({"slot":"source","role":"source","image":arguments["source"]["expect"]}));
        for image in arguments["references"].as_array().into_iter().flatten() {
            run["images"]["in"]
                .as_array_mut()
                .unwrap()
                .push(json!({"slot":"references","role":"reference","image":image["expect"]}));
        }
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
    fs::write(directory.join("run.json"), run.to_string()).unwrap();
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
