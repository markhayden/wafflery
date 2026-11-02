use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{OnceLock, mpsc},
    time::Duration,
};

fn executable() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            Command::new(env!("CARGO"))
                .args(["build", "--locked", "--example", "sample-lab"])
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
        root.join("target/debug/examples/sample-lab")
    })
}

struct Peer {
    child: Child,
    input: Option<ChildStdin>,
    messages: mpsc::Receiver<Value>,
    root: PathBuf,
    progress: usize,
}

impl Peer {
    fn start(root: &Path, scenario: &str) -> Self {
        let mut child = Command::new(executable())
            .arg(root.join("runs"))
            .arg(scenario)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else {
                    break;
                };
                let Ok(value) = serde_json::from_str(&line) else {
                    break;
                };
                if sender.send(value).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            messages,
            root: root.canonicalize().unwrap(),
            progress: 0,
        }
    }
    fn send(&mut self, value: Value) {
        writeln!(self.input.as_mut().unwrap(), "{value}").unwrap();
    }
    fn call(&mut self, id: i64, method: &str, params: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}));
        self.until(id)
    }
    fn until(&mut self, id: i64) -> Value {
        loop {
            let value = self
                .messages
                .recv_timeout(Duration::from_secs(5))
                .expect("bounded wire reply");
            if value["method"] == "roots/list" {
                self.send(json!({"jsonrpc":"2.0","id":value["id"],"result":{"roots":[{"uri":url::Url::from_directory_path(&self.root).unwrap().as_str(),"name":"Fixture inputs"}]}}));
            } else if value["method"] == "notifications/progress" {
                self.progress += 1;
            } else if value["id"] == id {
                return value;
            } else {
                panic!("unexpected wire reply: {value}");
            }
        }
    }
    fn initialize(&mut self) {
        let response = self.call(1, "initialize", json!({"protocolVersion":"2025-11-25","capabilities":{"roots":{}},"clientInfo":{"name":"independent-probe","version":"1"}}));
        assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        // Drain the roots exchange using a separate ping response.
        self.call(2, "ping", json!({}));
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.input.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn independent_peer_pages_tools_and_returns_verifiable_images_and_resources() {
    use sha2::{Digest, Sha256};
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source image.png");
    std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let mut peer = Peer::start(temp.path(), "normal");
    peer.initialize();
    let first = peer.call(3, "tools/list", json!({}));
    assert_eq!(first["result"]["tools"][0]["name"], "describe");
    let second = peer.call(
        4,
        "tools/list",
        json!({"cursor":first["result"]["nextCursor"]}),
    );
    assert_eq!(second["result"]["tools"][0]["name"], "mark_band");
    let description = peer.call(5, "tools/call", json!({"name":"describe","arguments":{}}));
    assert_eq!(
        description["result"]["structuredContent"]["tool"]["name"],
        "sample-lab"
    );
    for id in [6, 7] {
        let response = peer.call(id, "tools/call", json!({"name":"mark_band","arguments":{"source":{"path":source},"label":"wire test","tone":"amber"},"_meta":{"progressToken":id}}));
        let run = &response["result"]["structuredContent"];
        assert_eq!(run["status"], "completed", "{response}");
        let _: table_manners::RunResult = serde_json::from_value(run.clone()).unwrap();
        let image = &run["images"]["out"][0];
        let output = std::fs::read(image["path"].as_str().unwrap()).unwrap();
        let digest: String = Sha256::digest(&output)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(image["image"]["file_sha256"], digest);
        assert_ne!(output, include_bytes!("../ui/tests/fixtures/sample.png"));
        let saved = peer.call(
            id + 10,
            "resources/read",
            json!({"uri":run["run"]["resource"]}),
        );
        assert_eq!(
            serde_json::from_str::<Value>(saved["result"]["contents"][0]["text"].as_str().unwrap())
                .unwrap(),
            *run
        );
    }
    assert!(peer.progress >= 4);
    peer.input.take();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = peer.child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(std::time::Instant::now() < deadline, "peer ignored EOF");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn cancellation_persists_a_cancelled_run_without_an_mcp_call_result() {
    for scenario in ["slow", "held"] {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.png");
        std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let mut peer = Peer::start(temp.path(), scenario);
        peer.initialize();
        peer.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"mark_band","arguments":{"source":{"path":source},"label":"cancel test"},"_meta":{"progressToken":3}}}));
        let first = peer.messages.recv_timeout(Duration::from_secs(3)).unwrap();
        assert_eq!(first["method"], "notifications/progress");
        peer.send(
            json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":3}}),
        );
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            let response = peer.call(4, "resources/read", json!({"uri":"sample-lab://runs"}));
            let runs: Value =
                serde_json::from_str(response["result"]["contents"][0]["text"].as_str().unwrap())
                    .unwrap();
            if runs["runs"][0]["status"] == "cancelled" {
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            peer.messages
                .recv_timeout(Duration::from_millis(100))
                .is_err()
        );
    }
}

#[test]
fn wrong_initialize_protocol_is_rejected_before_any_action() {
    let temp = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(temp.path(), "normal");
    assert_eq!(
        peer.call(1, "initialize", json!({"protocolVersion":"2026-07-28"}))["error"]["code"],
        -32602
    );
    assert!(!temp.path().join("runs/calls.ndjson").exists());
}

#[test]
fn inline_images_and_mismatched_fingerprints_follow_the_shared_contract() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let temp = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(temp.path(), "normal");
    peer.initialize();
    let source = json!({"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}});
    let response = peer.call(3, "tools/call", json!({"name":"mark_band","arguments":{"source":source,"label":"inline test","return_images":"inline","image_blocks":true}}));
    let result = &response["result"]["structuredContent"];
    assert_eq!(result["status"], "completed");
    assert_eq!(response["result"]["content"][1]["type"], "image");
    assert!(result["images"]["out"][0].get("path").is_none());
    let mut source = source;
    source["expect"] = result["images"]["in"][0]["image"].clone();
    source["expect"]["file_sha256"] = json!("0".repeat(64));
    let refused = peer.call(
        4,
        "tools/call",
        json!({"name":"mark_band","arguments":{"source":source,"label":"must reject"}}),
    );
    assert_eq!(refused["result"]["structuredContent"]["status"], "invalid");
    assert_eq!(refused["result"]["isError"], true);
    let _: table_manners::RunResult =
        serde_json::from_value(refused["result"]["structuredContent"].clone()).unwrap();
}

#[test]
fn optional_setup_refuses_only_needed_calls_and_releases_the_correlation_id() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.png");
    std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let mut peer = Peer::start(temp.path(), "readiness");
    peer.initialize();
    let description = peer.call(3, "tools/call", json!({"name":"describe","arguments":{}}))["result"]["structuredContent"].clone();
    assert_eq!(
        description["actions"][0]["readiness"]["state"],
        "needs_setup"
    );
    assert_eq!(
        description["actions"][1]["readiness"]["requirements"][0]["state"],
        "present"
    );
    let listed = peer.call(20, "tools/list", json!({"cursor":"actions"}));
    let paid_schema = &listed["result"]["tools"][1]["inputSchema"];
    assert!(
        wafflery::tools::validator(paid_schema)
            .unwrap()
            .is_valid(&json!({"label":"No images needed","max_spend_usd":0.5}))
    );
    for state in ["valid", "invalid", "missing", "unknown"] {
        std::fs::create_dir_all(temp.path().join("runs")).unwrap();
        std::fs::write(temp.path().join("runs/credential-state"), state).unwrap();
        let refreshed = peer.call(4, "tools/call", json!({"name":"describe","arguments":{}}))["result"]["structuredContent"].clone();
        wafflery::tools::validate_contract("urn:table-manners:core:description", &refreshed)
            .unwrap();
        assert_eq!(
            refreshed["actions"][1]["readiness"]["requirements"][0]["state"],
            state
        );
    }
    let mut arguments = json!({"source":{"path":source},"label":"optional setup","filler":true});
    let refused = peer.call(5, "tools/call", json!({"name":"mark_band","arguments":arguments,"_meta":{"table-manners/client_run_id":"released-id"}}));
    assert_eq!(refused["error"]["code"], -32602);
    assert_eq!(refused["error"]["data"]["reason"], "not_ready");
    assert_eq!(refused["error"]["data"]["run"], Value::Null);
    assert!(!temp.path().join("runs/calls.ndjson").exists());
    let runs = peer.call(6, "resources/read", json!({"uri":"sample-lab://runs"}));
    assert_eq!(
        serde_json::from_str::<Value>(runs["result"]["contents"][0]["text"].as_str().unwrap())
            .unwrap()["runs"],
        json!([])
    );
    arguments.as_object_mut().unwrap().remove("filler");
    let completed = peer.call(7, "tools/call", json!({"name":"mark_band","arguments":arguments,"_meta":{"table-manners/client_run_id":"released-id"}}));
    assert_eq!(
        completed["result"]["structuredContent"]["status"],
        "completed"
    );
}

#[test]
fn pre_run_refusals_release_the_claim_for_a_corrected_wire_request() {
    for (scenario, label) in [
        ("refusals", "refuse plain"),
        ("refusals", "refuse structured"),
        ("refusals", "refuse internal"),
        ("readiness", "Needs setup"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.png");
        std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let mut peer = Peer::start(temp.path(), scenario);
        peer.initialize();
        let mut params = json!({"name":"mark_band","arguments":{"source":{"path":source},"label":label},"_meta":{"table-manners/client_run_id":"released-id"}});
        if scenario == "readiness" {
            params["arguments"]["filler"] = json!(true);
        }
        let refusal = peer.call(3, "tools/call", params.clone());
        assert!(refusal["error"].is_object(), "{refusal}");
        assert!(!temp.path().join("runs/calls.ndjson").exists());
        params["arguments"]["label"] = json!("Corrected input");
        if scenario == "readiness" {
            params["arguments"]["filler"] = json!(false);
        }
        let corrected = peer.call(4, "tools/call", params.clone());
        assert_eq!(
            corrected["result"]["structuredContent"]["status"], "completed",
            "{corrected}"
        );
        let duplicate = peer.call(5, "tools/call", params);
        assert_eq!(duplicate["error"]["data"]["reason"], "client_run_id_reused");
        assert_eq!(
            duplicate["error"]["data"]["run"]["client_run_id"],
            "released-id"
        );
        assert_eq!(
            std::fs::read_to_string(temp.path().join("runs/calls.ndjson"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }
}

#[test]
fn a_queued_cancel_keeps_its_claim_and_does_not_allocate_a_run() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.png");
    std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let mut peer = Peer::start(temp.path(), "queued");
    peer.initialize();
    let mut params = json!({"name":"mark_band","arguments":{"source":{"path":source},"label":"Queued"},"_meta":{"table-manners/client_run_id":"queued-id"}});
    peer.send(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":params}));
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !temp.path().join("runs/queued").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "peer never queued the call"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let duplicate = peer.call(4, "tools/call", params.clone());
    assert_eq!(
        duplicate["error"]["data"],
        json!({"reason":"client_run_id_reused","run":null})
    );
    peer.send(json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":3,"reason":"test"}}));
    while !temp.path().join("runs/queue-cancelled").exists() {
        assert!(
            std::time::Instant::now() < deadline,
            "queued cancel did not finish"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let duplicate = peer.call(5, "tools/call", params.clone());
    assert_eq!(
        duplicate["error"]["data"],
        json!({"reason":"client_run_id_reused","run":null})
    );
    assert!(!temp.path().join("runs/calls.ndjson").exists());
    std::fs::write(temp.path().join("runs/release-queue"), "").unwrap();
    params["_meta"]["table-manners/client_run_id"] = json!("fresh-id");
    let fresh = peer.call(6, "tools/call", params);
    assert_eq!(
        fresh["result"]["structuredContent"]["status"], "completed",
        "{fresh}"
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("runs/calls.ndjson"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}
