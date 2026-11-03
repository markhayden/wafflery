#[path = "support/contract_schema.rs"]
mod schema;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{OnceLock, mpsc},
    time::Duration,
};

struct Peer {
    child: Child,
    input: ChildStdin,
    messages: mpsc::Receiver<Value>,
}
impl Peer {
    fn start(root: &Path, scenario: &str) -> Self {
        static BINARY: OnceLock<PathBuf> = OnceLock::new();
        let binary = BINARY.get_or_init(|| {
            assert!(
                Command::new(env!("CARGO"))
                    .args(["build", "--locked", "--example", "sample-lab"])
                    .status()
                    .unwrap()
                    .success()
            );
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
        });
        let mut child = wafflery::process::spawn_std(
            Command::new(binary)
                .arg(root)
                .arg(scenario)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit()),
        )
        .unwrap();
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (tx, messages) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if tx.send(serde_json::from_str(&line).unwrap()).is_err() {
                    break;
                }
            }
        });
        let mut peer = Self {
            child,
            input,
            messages,
        };
        assert_eq!(peer.call(1, "initialize", json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"independent-control-probe","version":"1"}}))["result"]["protocolVersion"], "2025-11-25");
        writeln!(
            peer.input,
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
        peer
    }
    fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        writeln!(
            self.input,
            "{}",
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
        )
        .unwrap();
        let response = self
            .messages
            .recv_timeout(Duration::from_secs(5))
            .expect("bounded peer response");
        assert_eq!(response["id"], id, "{response}");
        response
    }

    fn session(&mut self, tool: &str) -> Value {
        let response = self.call(
            50,
            "resources/read",
            json!({"uri":format!("{tool}://session")}),
        );
        let value: Value =
            serde_json::from_str(response["result"]["contents"][0]["text"].as_str().unwrap())
                .unwrap();
        schema::validator("session").validate(&value).unwrap();
        value
    }
    fn assert_no_runs(&mut self, tool: &str) {
        let response = self.call(
            51,
            "resources/read",
            json!({"uri":format!("{tool}://runs")}),
        );
        let value: Value =
            serde_json::from_str(response["result"]["contents"][0]["text"].as_str().unwrap())
                .unwrap();
        schema::validator("runs").validate(&value).unwrap();
        assert_eq!(value["runs"], json!([]));
    }
    fn estimate(&mut self, cap: f64) -> Value {
        self.call(4, "tools/call", json!({"name":"estimate","arguments":{"action":"price_test","arguments":{"label":"An offline quote","max_spend_usd":cap}},"_meta":{"table-manners/client_run_id":"reused-control-id"}}))
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn control_discovery_and_estimates_never_allocate_or_spend() {
    let mut sessions = std::collections::HashSet::new();
    for (scenario, tool, refused) in [
        ("control-ready", "sample-lab", None),
        ("control-lens", "test-lens", None),
        ("control-needs-setup", "sample-lab", Some("readiness")),
        ("control-unavailable", "sample-lab", Some("readiness")),
        ("control-budget", "sample-lab", Some("budget")),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut peer = Peer::start(root.path(), scenario);
        let before = peer.session(tool);
        assert!(sessions.insert(before["id"].as_str().unwrap().to_owned()));
        for field in ["spent_usd", "unresolved_usd", "reserved_usd"] {
            assert_eq!(before[field], 0);
        }
        let description = peer.call(2,"tools/call",json!({"name":"describe","arguments":{}}))["result"]["structuredContent"].clone();
        schema::validator("description")
            .validate(&description)
            .unwrap();
        assert!(
            description["actions"]
                .as_array()
                .unwrap()
                .iter()
                .all(|action| action["name"] != "estimate" && action["name"] != "describe")
        );
        assert_eq!(description["contract"], "0.4.0");
        assert_eq!(description["tool"]["name"], tool);
        let tools = peer.call(3, "tools/list", json!({}));
        assert!(
            tools["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|tool| tool["name"] == "estimate")
        );
        let first = peer.estimate(0.5)["result"]["structuredContent"].clone();
        schema::validator("estimate").validate(&first).unwrap();
        assert_eq!(first["session"], before);
        assert_eq!(first["allowed"], refused.is_none());
        assert_eq!(first["refused"], json!(refused));
        assert_eq!(first["session_id"], first["session"]["id"]);
        let second = peer.estimate(0.5)["result"]["structuredContent"].clone();
        schema::validator("estimate").validate(&second).unwrap();
        assert_eq!(first["binding"], second["binding"]);
        assert!(second.get("run").is_none());
        assert_eq!(peer.session(tool), before);
        peer.assert_no_runs(tool);
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
#[test]
fn control_estimate_errors_and_caps_are_controls_not_runs() {
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), "control-ready");
    let before = peer.session("sample-lab");
    for field in ["spent_usd", "unresolved_usd", "reserved_usd"] {
        assert_eq!(before[field], 0);
    }
    let capped = peer.estimate(0.01);
    schema::validator("estimate")
        .validate(&capped["result"]["structuredContent"])
        .unwrap();
    assert_eq!(capped["result"]["structuredContent"]["refused"], "cap");
    let bad_cap = peer.estimate(2.0);
    schema::validator("refusal")
        .validate(&bad_cap["error"]["data"])
        .unwrap();
    assert_eq!(bad_cap["error"]["code"], -32602);
    assert_eq!(bad_cap["error"]["data"]["reason"], "invalid_arguments");
    assert!(bad_cap["error"]["data"]["run"].is_null());
    let malformed = peer.call(7,"tools/call",json!({"name":"estimate","arguments":{"action":"price_test","arguments":{"label":"invalid cap","max_spend_usd":0}}}));
    assert_eq!(malformed["error"]["code"], -32602);
    assert!(malformed["error"].get("data").is_none());
    let unknown = peer.call(
        8,
        "tools/call",
        json!({"name":"estimate","arguments":{"action":"other","arguments":{}}}),
    );
    schema::validator("refusal")
        .validate(&unknown["error"]["data"])
        .unwrap();
    assert_eq!(unknown["error"]["data"]["reason"], "invalid_arguments");
    assert_eq!(
        peer.estimate(0.5)["result"]["structuredContent"]["allowed"],
        true
    );
    assert_eq!(peer.session("sample-lab"), before);
    peer.assert_no_runs("sample-lab");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn estimates_refuse_ungranted_paths_without_runs_and_accept_inline_inputs() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("source.png");
    let bytes = include_bytes!("../ui/tests/fixtures/sample.png");
    std::fs::write(&path, bytes).unwrap();
    let mut peer = Peer::start(root.path(), "estimates");
    let before = peer.session("sample-lab");
    let mut args = json!({"label":"grant check","max_spend_usd":0.5,"source":{"path":path}});
    let refused = peer.call(
        60,
        "tools/call",
        json!({"name":"estimate","arguments":{"action":"price_test","arguments":args}}),
    );
    assert_eq!(refused["error"]["code"], -32602, "{refused}");
    let data = &refused["error"]["data"];
    schema::validator("refusal").validate(data).unwrap();
    assert_eq!(data["reason"], "invalid_arguments");
    assert_eq!(data["error"]["code"], "input");
    assert!(
        data["error"]["reason"]
            .as_str()
            .unwrap()
            .contains("outside input grants")
    );
    assert!(!root.path().join("estimate-read").exists());
    assert_eq!(peer.session("sample-lab"), before);
    peer.assert_no_runs("sample-lab");
    args["source"] = json!({"inline":{"media_type":"image/png","data":STANDARD.encode(bytes)}});
    let allowed = peer.call(
        61,
        "tools/call",
        json!({"name":"estimate","arguments":{"action":"price_test","arguments":args}}),
    );
    let quote = &allowed["result"]["structuredContent"];
    schema::validator("estimate").validate(quote).unwrap();
    assert_eq!(quote["allowed"], true);
    args.as_object_mut().unwrap().remove("max_spend_usd");
    let default_cap = peer.call(
        64,
        "tools/call",
        json!({"name":"estimate","arguments":{"action":"price_test","arguments":args}}),
    );
    let quote = &default_cap["result"]["structuredContent"];
    schema::validator("estimate").validate(quote).unwrap();
    assert_eq!(quote["cap_usd"], 1.0);
    assert_eq!(peer.session("sample-lab"), before);
    peer.assert_no_runs("sample-lab");
}

#[test]
fn invalid_estimate_reference_lists_are_refused_without_losing_the_reader() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), "estimates");
    let before = peer.session("sample-lab");
    let args = json!({"label":"invalid list","max_spend_usd":0.5,
        "source":{"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}},
        "references":"not a list"});
    let refused = peer.call(
        62,
        "tools/call",
        json!({"name":"estimate","arguments":{"action":"price_test","arguments":args}}),
    );
    assert_eq!(refused["error"]["code"], -32602, "{refused}");
    assert!(
        refused["error"].get("data").is_none(),
        "schema errors are plain invalid params"
    );
    assert!(peer.call(63, "ping", json!({}))["result"].is_object());
    assert_eq!(peer.session("sample-lab"), before);
    peer.assert_no_runs("sample-lab");
}

#[test]
fn inline_estimate_and_execution_share_the_optional_cap_default() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), "estimates");
    let args = json!({"label":"default cap","source":{"inline":{"media_type":"image/png",
        "data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}}});
    let quoted = peer.call(
        70,
        "tools/call",
        json!({"name":"estimate","arguments":{"action":"price_test","arguments":args}}),
    );
    let quote = &quoted["result"]["structuredContent"];
    schema::validator("estimate").validate(quote).unwrap();
    assert_eq!(quote["cap_usd"], 1.0);
    let called = peer.call(71,"tools/call",json!({"name":"price_test","arguments":args,
        "_meta":{"table-manners/client_run_id":"default-cap-test","table-manners/estimate":quote["binding"]}}));
    let result = &called["result"]["structuredContent"];
    schema::validator("result").validate(result).unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    assert_eq!(result["spend"]["usd"], 0.08);
    assert_eq!(peer.session("sample-lab")["spent_usd"], 0.08);
}
