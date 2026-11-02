//! Synthetic provider outcomes and ledger settlement; no network or credentials are read.
use super::PeerState;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub fn readiness(root: &Path) -> Value {
    let value = std::fs::read_to_string(root.join("paid-credential-state"))
        .unwrap_or_else(|_| "present".into());
    let state = match value.trim() {
        "valid" => "valid",
        "invalid" => "invalid",
        "missing" => "missing",
        _ => "present",
    };
    json!({"state":if matches!(state,"missing"|"invalid") {"needs_setup"} else {"ready"},
        "requirements":[{"kind":"credential","name":"SCRIPTED_PAID_KEY","state":state,
            "setup":"Configure the scripted provider outside the playground, then restart the tool."}]})
}
pub fn refusal(root: &Path, arguments: &Value) -> Option<Value> {
    if arguments["label"] == "paid-not-ready" {
        // Simulate a requirement becoming unavailable at the call's pre-allocation check.
        std::fs::write(root.join("paid-credential-state"), "missing").unwrap();
    }
    let ready = readiness(root);
    (ready["state"] != "ready")
        .then(|| json!({"reason":"not_ready","run":null,"requirements":ready["requirements"]}))
}
pub fn balance(session: &mut Value) {
    session["remaining_usd"] = json!(
        (session["budget_usd"].as_f64().unwrap()
            - session["spent_usd"].as_f64().unwrap()
            - session["reserved_usd"].as_f64().unwrap())
        .max(0.0)
    );
}
pub fn settle(
    root: &Path,
    state: &Arc<Mutex<PeerState>>,
    cancelled: &AtomicBool,
    label: &str,
    run: &mut Value,
    mut heartbeat: impl FnMut(),
) {
    if label == "paid-held" {
        std::fs::write(root.join("paid-provider-started"), "synthetic request sent").unwrap();
        let mut last = std::time::Instant::now();
        while !cancelled.load(Ordering::Acquire) && !root.join("release-paid").exists() {
            if last.elapsed() >= Duration::from_secs(1) {
                heartbeat();
                last = std::time::Instant::now();
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    let cancelled = cancelled.load(Ordering::Acquire);
    let unresolved = if cancelled || label == "paid-unresolved" {
        0.12
    } else {
        0.0
    };
    let usd = if unresolved > 0.0 || label == "paid-full" {
        0.12
    } else if label == "paid-credential" {
        0.0
    } else {
        0.08
    };
    let mut ledger = state.lock().unwrap();
    ledger.session["reserved_usd"] =
        json!((ledger.session["reserved_usd"].as_f64().unwrap() - 0.12).max(0.0));
    ledger.session["spent_usd"] = json!(ledger.session["spent_usd"].as_f64().unwrap() + usd);
    ledger.session["unresolved_usd"] =
        json!(ledger.session["unresolved_usd"].as_f64().unwrap() + unresolved);
    balance(&mut ledger.session);
    run["spend"] = json!({"usd":usd,"unresolved_usd":unresolved,"basis":"measured"});
    run["status"] = json!(if cancelled { "cancelled" } else { "completed" });
    if !cancelled && matches!(label, "paid-credential" | "paid-unresolved" | "paid-failed") {
        run["status"] = json!("error");
        run["error"] = json!({"code":if label == "paid-credential" {"credential"} else {"provider"},
            "reason":if label == "paid-credential" {"Synthetic provider rejected authentication (401)."} else {"Synthetic provider failed."},"retryable":false});
    }
    drop(ledger);
    if label == "paid-credential" || label == "paid-validate" {
        std::fs::write(
            root.join("paid-credential-state"),
            if label == "paid-credential" {
                "invalid"
            } else {
                "valid"
            },
        )
        .unwrap();
    }
}
