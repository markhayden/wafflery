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
    frame.resize(1048575, b' ');
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
