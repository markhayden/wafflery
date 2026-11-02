#[path = "../testkit/v04_fixtures.rs"]
mod fixtures;
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{OnceLock, mpsc},
    time::{Duration, Instant},
};

fn binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        assert!(
            Command::new(env!("CARGO"))
                .args(["build", "--locked", "--example", "sample-lab"])
                .status()
                .unwrap()
                .success()
        );
        Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
    })
}
struct Peer {
    child: Child,
    input: Option<ChildStdin>,
    messages: mpsc::Receiver<Value>,
}
impl Peer {
    fn start(root: &Path, arguments: &[&str]) -> Self {
        let mut child = wafflery::process::spawn_std(
            Command::new(binary())
                .arg(root)
                .arg("v04-operations")
                .args(arguments)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit()),
        )
        .unwrap();
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if sender.send(serde_json::from_str(&line).unwrap()).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            messages,
        }
    }
    fn call(&mut self, id: u64, method: &str, params: Value) -> Value {
        writeln!(
            self.input.as_mut().unwrap(),
            "{}",
            json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
        )
        .unwrap();
        let response = self
            .messages
            .recv_timeout(Duration::from_secs(5))
            .expect("bounded wire response");
        assert_eq!(
            response["id"], id,
            "unexpected request/notification: {response}"
        );
        response
    }
    fn initialize(&mut self) {
        let response = self.call(1, "initialize", json!({"protocolVersion":"2025-11-25","capabilities":{"roots":{"listChanged":true}},"clientInfo":{"name":"v04-wire-probe","version":"1"}}));
        assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
        writeln!(
            self.input.as_mut().unwrap(),
            "{}",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .unwrap();
    }
    fn read(&mut self, uri: &str) -> Value {
        let response = self.call(8, "resources/read", json!({"uri":uri}));
        serde_json::from_str(response["result"]["contents"][0]["text"].as_str().unwrap()).unwrap()
    }
    fn close(&mut self) {
        drop(self.input.take());
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(
                Instant::now() < deadline,
                "peer did not finish after stdin EOF"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn v04_discovery_uses_explicit_grants_and_never_requests_roots() {
    let root = tempfile::tempdir().unwrap();
    let allowed = tempfile::tempdir().unwrap();
    let mut sessions = std::collections::BTreeSet::new();
    for tool in ["sample-lab", "sample-lab", "test-prism"] {
        let mut peer = Peer::start(
            root.path(),
            &[
                "--tool-name",
                tool,
                "--allow-dir",
                allowed.path().to_str().unwrap(),
            ],
        );
        peer.initialize();
        let description = peer.call(2, "tools/call", json!({"name":"describe","arguments":{}}))["result"]["structuredContent"].clone();
        let expected = fixtures::documents(tool)
            .into_iter()
            .find(|(id, _)| *id == "urn:table-manners:core:description")
            .unwrap()
            .1;
        assert_eq!(description, expected);
        let _: table_manners_v04::Description = serde_json::from_value(description).unwrap();
        let session = peer.read(&format!("{tool}://session"));
        let _: table_manners_v04::Session = serde_json::from_value(session.clone()).unwrap();
        assert_eq!(
            session["allowed_dirs"],
            json!([allowed.path().canonicalize().unwrap()])
        );
        assert!(sessions.insert(session["id"].as_str().unwrap().to_owned()));
        assert_eq!(session["spent_usd"], 0);
        assert_eq!(session["reserved_usd"], 0);
        assert_eq!(peer.read(&format!("{tool}://runs"))["runs"], json!([]));
        peer.close();
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn bad_grants_exit_64_before_protocol_start() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("not a directory");
    std::fs::write(&file, b"synthetic test").unwrap();
    let missing = root.path().join("missing");
    for grant in [
        "relative",
        file.to_str().unwrap(),
        missing.to_str().unwrap(),
    ] {
        let mut child = wafflery::process::spawn_std(
            Command::new(binary())
                .arg(root.path())
                .arg("v04-operations")
                .args(["--allow-dir", grant])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped()),
        )
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert_eq!(status.code(), Some(64), "invalid grant {grant}");
                break;
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("invalid grant hung");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[test]
fn controls_require_initialization_and_do_not_allocate_runs() {
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &[]);
    assert_eq!(
        peer.call(1, "tools/list", json!({}))["error"]["code"],
        -32000
    );
    assert_eq!(
        peer.call(1, "initialize", json!({"protocolVersion":"2026-07-28"}))["error"]["code"],
        -32602
    );
    peer.initialize();
    let before = peer.read("sample-lab://session");
    assert_eq!(before["allowed_dirs"], json!([]));
    assert_eq!(
        peer.call(3, "tools/list", json!({}))["result"]["tools"][0]["name"],
        "describe"
    );
    assert_eq!(
        peer.call(
            4,
            "tools/call",
            json!({"name":"describe","arguments":{"unexpected":true}})
        )["error"]["code"],
        -32602
    );
    assert_eq!(
        peer.call(
            5,
            "tools/call",
            json!({"name":"soften_token","arguments":{}})
        )["error"]["code"],
        -32602
    );
    assert_eq!(
        peer.call(6, "resources/read", json!({"uri":"foreign://session"}))["error"]["code"],
        -32002
    );
    assert_eq!(peer.read("sample-lab://session"), before);
    assert_eq!(peer.read("sample-lab://runs")["runs"], json!([]));
    peer.close();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn grants_follow_symlinks_and_preserve_paths_with_spaces() {
    let root = tempfile::tempdir().unwrap();
    let allowed = tempfile::tempdir().unwrap();
    let link = root.path().join("input alias");
    std::os::unix::fs::symlink(allowed.path(), &link).unwrap();
    let mut peer = Peer::start(
        root.path(),
        &[
            "--allow-dir",
            link.to_str().unwrap(),
            "--allow-dir",
            allowed.path().to_str().unwrap(),
        ],
    );
    peer.initialize();
    assert_eq!(
        peer.read("sample-lab://session")["allowed_dirs"],
        json!([allowed.path().canonicalize().unwrap()])
    );
    peer.close();
}

#[test]
fn oversized_input_stops_without_protocol_output_or_run_writes() {
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &[]);
    peer.initialize();
    let request = br#"{"jsonrpc":"2.0","id":9,"method":"ping"}"#;
    let mut frame = request.to_vec();
    frame.resize(2097151, b' ');
    frame.push(b'\n');
    peer.input.as_mut().unwrap().write_all(&frame).unwrap();
    let response = peer.messages.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(response["id"], 9);
    assert_eq!(response["result"], json!({}));
    frame.insert(frame.len() - 1, b' ');
    peer.input.as_mut().unwrap().write_all(&frame).unwrap();
    // Keep stdin open: the size limit, rather than EOF or invalid JSON, must stop it.
    assert!(matches!(
        peer.messages.recv_timeout(Duration::from_secs(5)),
        Err(mpsc::RecvTimeoutError::Disconnected)
    ));
    assert!(!peer.child.wait().unwrap().success());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn artifact_controls_round_trip_without_runs_or_spend() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &[]);
    peer.initialize();
    let before = peer.read("sample-lab://session");
    let listing = peer.call(9, "tools/list", json!({}));
    let tools = listing["result"]["tools"].as_array().unwrap();
    assert_eq!(
        tools
            .iter()
            .take(3)
            .map(|t| t["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["describe", "export", "import"]
    );
    for entry in tools {
        assert_eq!(entry["inputSchema"]["type"], "object");
    }
    for kind in ["location", "sample-lab:Token"] {
        let document = fixtures::artifact("sample-lab", kind);
        let bytes = serde_json::to_vec(&document).unwrap();
        let media = if kind == "location" {
            "application/json"
        } else {
            "application/x-sample-lab-token"
        };
        let arguments = json!({"inline":{"media_type":media,"data":STANDARD.encode(&bytes)}});
        let schema = &tools.iter().find(|t| t["name"] == "import").unwrap()["inputSchema"];
        let validator = jsonschema::validator_for(schema).unwrap();
        validator.validate(&arguments).unwrap();
        assert!(!validator.is_valid(&json!({"unexpected":true})));
        let export_schema = &tools.iter().find(|t| t["name"] == "export").unwrap()["inputSchema"];
        jsonschema::validator_for(export_schema)
            .unwrap()
            .validate(&json!({"artifact":document["artifact"]}))
            .unwrap();
        let response = peer.call(
            10,
            "tools/call",
            json!({"name":"import","arguments":arguments}),
        );
        let imported = &response["result"]["structuredContent"];
        let _: table_manners_v04::operations::ImportResponse =
            serde_json::from_value(imported.clone()).unwrap();
        assert_eq!(imported["already_present"], false);
        assert_eq!(imported["artifact"], document["artifact"]);
        assert_eq!(
            peer.read(document["artifact"]["resource"].as_str().unwrap()),
            document
        );
        let preview = peer.call(
            11,
            "resources/read",
            json!({"uri":document["preview"]["resource"]}),
        );
        let image = &preview["result"]["contents"][0];
        assert_eq!(image["mimeType"], "image/png");
        let image_bytes = STANDARD.decode(image["blob"].as_str().unwrap()).unwrap();
        let hash: String = Sha256::digest(&image_bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hash, document["preview"]["image"]["file_sha256"]);
        let response = peer.call(
            12,
            "tools/call",
            json!({"name":"export","arguments":{"artifact":imported["artifact"]}}),
        );
        let exported = &response["result"]["structuredContent"];
        let _: table_manners_v04::operations::ExportResponse =
            serde_json::from_value(exported.clone()).unwrap();
        let exported_bytes = std::fs::read(exported["file"]["path"].as_str().unwrap()).unwrap();
        assert_eq!(exported_bytes, bytes);
        if kind == "sample-lab:Token" {
            let mut padded = bytes.clone();
            padded.resize(1048576, b' ');
            let accepted = peer.call(13, "tools/call", json!({"name":"import","arguments":{"inline":{"media_type":media,"data":STANDARD.encode(&padded)}}}));
            assert_eq!(
                accepted["result"]["structuredContent"]["already_present"],
                true
            );
            padded.push(b' ');
            let refused = peer.call(14, "tools/call", json!({"name":"import","arguments":{"inline":{"media_type":media,"data":STANDARD.encode(&padded)}}}));
            assert_eq!(refused["error"]["data"]["reason"], "invalid_arguments");
            assert!(
                refused["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("1 MiB")
            );
        }
    }
    assert_eq!(peer.read("sample-lab://session"), before);
    assert_eq!(peer.read("sample-lab://runs")["runs"], json!([]));
    peer.close();
    let mut restarted = Peer::start(root.path(), &[]);
    restarted.initialize();
    assert_ne!(restarted.read("sample-lab://session")["id"], before["id"]);
    let document = fixtures::artifact("sample-lab", "sample-lab:Token");
    assert_eq!(
        restarted.read(document["artifact"]["resource"].as_str().unwrap()),
        document
    );
    restarted.close();
}

#[test]
fn artifact_control_refusals_are_errors_not_runs_or_results() {
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &["--tool-name", "test-prism"]);
    peer.initialize();
    let session = peer.read("test-prism://session");
    let document = fixtures::artifact("test-prism", "test-prism:Token");
    let read = peer.call(
        10,
        "resources/read",
        json!({"uri":document["artifact"]["resource"]}),
    );
    assert_eq!(read["error"]["data"]["reason"], "binding");
    assert_eq!(read["error"]["data"]["bindings"][0]["port"], Value::Null);
    let export = peer.call(
        11,
        "tools/call",
        json!({"name":"export","arguments":{"artifact":document["artifact"]}}),
    );
    assert_eq!(export["error"]["data"]["bindings"][0]["port"], "artifact");
    let import = peer.call(
        12,
        "tools/call",
        json!({"name":"import","arguments":{"file":{"path":"/ungranted/synthetic.bundle"}}}),
    );
    assert_eq!(import["error"]["code"], -32602);
    assert_eq!(import["error"]["data"]["reason"], "invalid_arguments");
    for response in [read, export, import] {
        assert!(response.get("result").is_none());
        assert_eq!(response["error"]["data"]["run"], Value::Null);
        let _: table_manners_v04::Refusal =
            serde_json::from_value(response["error"]["data"].clone()).unwrap();
    }
    assert_eq!(peer.read("test-prism://session"), session);
    assert_eq!(peer.read("test-prism://runs")["runs"], json!([]));
    peer.close();
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

fn source_image() -> Value {
    use base64::{Engine, engine::general_purpose::STANDARD};
    json!({"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}})
}
fn action(peer: &mut Peer, name: &str, arguments: Value, client: &str) -> Value {
    peer.call(
        40,
        "tools/call",
        json!({"name":name,"arguments":arguments,"_meta":{"table-manners/client_run_id":client}}),
    )
}
fn completed(response: &Value) -> Value {
    assert!(response.get("error").is_none(), "{response}");
    assert_eq!(response["result"]["isError"], false);
    let value = response["result"]["structuredContent"].clone();
    let _: table_manners_v04::RunResult = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(value["status"], "completed");
    value
}

#[test]
fn renamed_wire_peer_executes_a_chain_and_serves_paged_saved_evidence() {
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &["--tool-name", "test-prism"]);
    peer.initialize();
    let before = peer.read("test-prism://session");
    let tools = peer.call(2, "tools/list", json!({}))["result"]["tools"].clone();
    assert_eq!(tools.as_array().unwrap().len(), 9);
    for declared in fixtures::description("test-prism")["actions"]
        .as_array()
        .unwrap()
    {
        let callable = tools
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == declared["name"])
            .expect("declared action is callable");
        assert_eq!(callable["inputSchema"]["type"], "object");
        jsonschema::validator_for(&callable["inputSchema"]).unwrap();
    }
    let located = completed(&action(
        &mut peer,
        "locate_marker",
        json!({"image":source_image()}),
        "locate",
    ));
    let stamped = completed(&action(
        &mut peer,
        "stamp_marker",
        json!({"image":source_image(),"marker":located["artifacts"]["out"][0]["artifact"]}),
        "stamp",
    ));
    let softened = completed(&action(
        &mut peer,
        "soften_token",
        json!({"token":stamped["artifacts"]["out"][0]["artifact"],"amount":0.75}),
        "soften",
    ));
    assert_eq!(softened["invocation"]["arguments"]["amount"], 0.75);
    assert_eq!(
        softened["artifacts"]["in"][0]["artifact"],
        stamped["artifacts"]["out"][0]["artifact"]
    );
    assert_ne!(
        softened["artifacts"]["out"][0]["artifact"]["identity"],
        stamped["artifacts"]["out"][0]["artifact"]["identity"]
    );
    for run in [&located, &stamped, &softened] {
        assert_eq!(run["run"]["session_id"], before["id"]);
        assert_eq!(peer.read(run["run"]["resource"].as_str().unwrap()), *run);
        let preview = run["images"]["out"][0]["path"]
            .as_str()
            .expect("stdio defaults to path previews");
        assert!(Path::new(preview).is_absolute() && Path::new(preview).is_file());
        use sha2::{Digest, Sha256};
        let hash: String = Sha256::digest(std::fs::read(preview).unwrap())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hash, run["images"]["out"][0]["image"]["file_sha256"]);
        let journal = peer.call(9,"resources/read",json!({"uri":format!("test-prism://run/{}/journal",run["run"]["id"].as_str().unwrap())}));
        assert!(
            journal["result"]["contents"][0]["text"]
                .as_str()
                .unwrap()
                .contains("completed")
        );
    }
    let page = peer.read("test-prism://runs");
    assert_eq!(page["runs"].as_array().unwrap().len(), 2);
    let next = peer.read(&format!(
        "test-prism://runs?cursor={}",
        page["next_cursor"].as_str().unwrap()
    ));
    assert_eq!(next["runs"].as_array().unwrap().len(), 1);
    assert_eq!(next["next_cursor"], Value::Null);
    assert_eq!(peer.read("test-prism://session"), before);
    peer.close();
}

#[test]
fn wire_binding_refusals_release_ids_and_completed_calls_keep_them() {
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &[]);
    peer.initialize();
    let missing = fixtures::artifact("sample-lab", "sample-lab:Token");
    let refused = action(
        &mut peer,
        "collect_tokens",
        json!({"tokens":[missing["artifact"]]}),
        "fixed",
    );
    assert_eq!(refused["error"]["data"]["reason"], "binding");
    assert_eq!(refused["error"]["data"]["bindings"][0]["index"], 0);
    assert_eq!(peer.read("sample-lab://runs")["runs"], json!([]));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    let made = completed(&action(
        &mut peer,
        "make_token",
        json!({"source":source_image()}),
        "make",
    ));
    let fixed = completed(&action(
        &mut peer,
        "collect_tokens",
        json!({"tokens":[made["artifacts"]["out"][0]["artifact"]]}),
        "fixed",
    ));
    let reused = action(&mut peer, "summarize", json!({}), "fixed");
    assert_eq!(reused["error"]["data"]["reason"], "client_run_id_reused");
    assert_eq!(reused["error"]["data"]["run"]["id"], fixed["run"]["id"]);
    assert!(reused["error"]["data"]["run"].get("dir").is_none());
    let bad = action(&mut peer, "soften_token", json!({}), "correct-shape");
    assert_eq!(bad["error"]["code"], -32602);
    assert!(bad["error"].get("data").is_none());
    completed(&action(&mut peer, "summarize", json!({}), "correct-shape"));
    peer.close();
}

#[test]
fn wire_outcomes_distinguish_invalid_inputs_and_io_errors_from_rpc_refusals() {
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &[]);
    peer.initialize();
    let invalid = action(
        &mut peer,
        "make_token",
        json!({"source":{"inline":{"media_type":"image/png","data":"e30="}}}),
        "invalid",
    );
    assert_eq!(invalid["result"]["isError"], true);
    let invalid = &invalid["result"]["structuredContent"];
    assert_eq!(invalid["status"], "invalid");
    assert_eq!(invalid["invocation"], Value::Null);
    assert_eq!(
        peer.read(invalid["run"]["resource"].as_str().unwrap()),
        *invalid
    );
    std::fs::write(root.path().join("previews"), b"blocked").unwrap();
    let failed = action(
        &mut peer,
        "make_token",
        json!({"source":source_image()}),
        "io",
    );
    assert_eq!(failed["result"]["isError"], true);
    let failed = &failed["result"]["structuredContent"];
    assert_eq!(failed["status"], "error");
    assert_eq!(failed["error"]["code"], "io");
    assert_eq!(failed["invocation"]["action"], "make_token");
    assert_eq!(
        peer.read(failed["run"]["resource"].as_str().unwrap()),
        *failed
    );
    peer.close();
}

#[test]
fn wire_image_blocks_are_opt_in_and_match_the_recorded_previews() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    use sha2::{Digest, Sha256};
    let root = tempfile::tempdir().unwrap();
    let mut peer = Peer::start(root.path(), &[]);
    peer.initialize();
    for blocks in [false, true] {
        let response = action(
            &mut peer,
            "make_token",
            json!({"source":source_image(),"return_images":"resource","image_blocks":blocks}),
            if blocks { "blocks" } else { "no-blocks" },
        );
        let run = completed(&response);
        let images: Vec<_> = response["result"]["content"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["type"] == "image")
            .collect();
        assert_eq!(images.len(), usize::from(blocks));
        if let Some(image) = images.first() {
            let bytes = STANDARD.decode(image["data"].as_str().unwrap()).unwrap();
            let hash: String = Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(hash, run["images"]["out"][0]["image"]["file_sha256"]);
            assert_eq!(image["mimeType"], "image/png");
        }
    }
    peer.close();
}
