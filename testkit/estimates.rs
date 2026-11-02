//! Independent estimate controls; filesystem gates let tests observe the remote reader.
use super::{PeerState, reply, send};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

pub fn start(
    root: PathBuf,
    state: Arc<Mutex<PeerState>>,
    output: Arc<Mutex<io::Stdout>>,
    message: Value,
) {
    std::thread::spawn(move || {
        let arguments = &message["params"]["arguments"]["arguments"];
        let label = arguments["label"].as_str().unwrap_or("");
        let mut journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("estimates.ndjson"))
            .unwrap();
        writeln!(journal, "{}", message["params"]).unwrap();
        std::fs::write(root.join("estimate-started"), arguments.to_string()).unwrap();
        if label == "disconnect" {
            let mut output = output.lock().unwrap();
            writeln!(output, "not a JSON-RPC response").unwrap();
            output.flush().unwrap();
        }
        if matches!(label, "hold" | "disconnect") {
            while !root.join("release-estimate").exists() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        if label.ends_with("error") {
            let mut error = json!({"code":-32602,"message":"Scripted estimate rejection"});
            if label == "structured-error" {
                error["data"] = super::v03_fixtures::refusal("invalid_arguments");
            }
            send(
                &output,
                json!({"jsonrpc":"2.0","id":message["id"],"error":error}),
            );
            return;
        }
        let path = arguments["source"]["path"].as_str().unwrap();
        let bytes =
            std::fs::read(path).expect("prepared inputs must remain until this reader finishes");
        let digest = hash(&bytes);
        std::fs::write(root.join("estimate-read"), &digest).unwrap();
        if label == "ledger" {
            state.lock().unwrap().session["spent_usd"] = json!(0.08);
        }
        let session = state.lock().unwrap().session.clone();
        let mut value = super::v03_fixtures::estimate(
            super::fixtures::TOOL,
            &session,
            (label == "readiness").then_some("readiness"),
        );
        value["effective"]["x-sample-lab"] = json!({"path":path,"binding":"internal-debug"});
        value["session"] = session.clone();
        value["cap_usd"] = arguments["max_spend_usd"].clone();
        let mut bound = arguments.clone();
        bound.as_object_mut().unwrap().remove("return_images");
        bound.as_object_mut().unwrap().remove("image_blocks");
        bound["source"] = json!({"file_sha256":digest,"expect":arguments["source"]["expect"]});
        value["binding"] = json!(format!(
            "sha256:{}",
            hash(
                &serde_json::to_vec(&json!([
                    "price_test",
                    bound,
                    value["tool"],
                    value["effective"]["model"],
                    value["effective"]["config_identity"],
                    session["id"]
                ]))
                .unwrap()
            )
        ));
        value["expires"] = json!(
            (time::OffsetDateTime::now_utc() + time::Duration::minutes(5))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
        );
        if label == "wrong-session" {
            value["session_id"] = json!("33333333-3333-4333-8333-333333333333");
        }
        reply(
            &output,
            &message["id"],
            json!({"structuredContent":value,"content":[],"isError":false}),
        );
    });
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
