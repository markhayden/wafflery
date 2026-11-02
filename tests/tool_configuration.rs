use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};
use wafflery::host::control;

fn peer() -> &'static Path {
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
struct Host {
    child: Child,
    _temporary: tempfile::TempDir,
    root: PathBuf,
    tool: PathBuf,
    origin: String,
    token: String,
}
impl Host {
    fn start(mode: &str, scenario: &str, extra: bool) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("host");
        let tool = temporary.path().join("tool");
        let home = temporary.path().join("home");
        let xdg = temporary.path().join("xdg config");
        for path in [&tool, &home, &xdg] {
            fs::create_dir_all(path).unwrap();
        }
        let config = match mode {
            "home" => home.join(".config/sample-lab.json"),
            "xdg" => xdg.join("sample-lab.json"),
            _ => temporary.path().join("selected config.json"),
        };
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        fs::write(&config, json!({"credential":"SCRIPTED_KEY", "env":"SCRIPTED_MODEL_DIR", "extra_when_configured":extra}).to_string()).unwrap();
        let mut descriptor = json!({"id":"sample", "executable":peer(), "args":[tool,scenario],"cwd":tool,"roots":[tool]});
        if mode == "explicit" {
            descriptor["config"] = json!(config);
        }
        let registrations = temporary.path().join("registration.json");
        fs::write(&registrations, json!([descriptor]).to_string()).unwrap();
        wafflery::tools::registration::register(&root, &registrations).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_wafflery"));
        command
            .args(["--foreground", "--no-open", "--data-dir"])
            .arg(&root)
            .env("HOME", &home)
            .env_remove("XDG_CONFIG_HOME")
            .env("SCRIPTED_KEY", "synthetic-provider-value")
            .env("SCRIPTED_MODEL_DIR", "synthetic-model-value")
            .env("SCRIPTED_EXTRA_KEY", "synthetic-extra-value")
            .env("SCRIPTED_UNRELATED", "synthetic-unrelated-value")
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if mode == "xdg" {
            command.env("XDG_CONFIG_HOME", &xdg);
        }
        if scenario == "configuration-missing-key" {
            command.env_remove("SCRIPTED_KEY");
        }
        let child = command.spawn().unwrap();
        let mut host = Self {
            child,
            _temporary: temporary,
            root,
            tool,
            origin: String::new(),
            token: String::new(),
        };
        let deadline = Instant::now() + Duration::from_secs(12);
        loop {
            if let Ok(state) = control::request(&host.root, "status") {
                host.origin = state["origin"].as_str().unwrap().into();
                break;
            }
            assert!(
                host.child.try_wait().unwrap().is_none(),
                "host exited during probe"
            );
            assert!(
                Instant::now() < deadline,
                "host did not finish probe startup"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let bootstrap = control::request(&host.root, "bootstrap").unwrap();
        host.token = host.http(
            "POST",
            "/api/session",
            json!({"bootstrap":bootstrap["bootstrap"]}),
        )["token"]
            .as_str()
            .unwrap()
            .into();
        host
    }
    fn http(&self, method: &str, path: &str, body: Value) -> Value {
        let address = self.origin.strip_prefix("http://").unwrap();
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(6)))
            .unwrap();
        let body = body.to_string();
        write!(stream, "{method} {path} HTTP/1.1\r\nHost: {address}\r\nOrigin: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", self.origin, self.token, body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        assert!(
            headers.starts_with("HTTP/1.1 200"),
            "request failed: {method} {path}"
        );
        serde_json::from_str(body).unwrap()
    }
    fn events(&self) -> Vec<Value> {
        fs::read_to_string(self.tool.join("startup.ndjson"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
    fn stop(&mut self) {
        control::request(&self.root, "quit").unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        while self.child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "host failed graceful shutdown");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[test]
fn probe_discovers_configured_names_before_a_fresh_serving_session() {
    for mode in ["explicit", "home", "xdg"] {
        let mut host = Host::start(mode, "configuration", false);
        let state = host.http("GET", "/api/state", Value::Null);
        let events = host.events();
        host.stop();
        assert_eq!(
            events.len(),
            3,
            "probe start, probe exit, then serving start: {mode}"
        );
        assert_eq!(events[0]["event"], "start");
        assert_eq!(events[1]["event"], "exit");
        assert_eq!(events[2]["event"], "start");
        assert_ne!(events[0]["session"], events[2]["session"]);
        assert_eq!(
            state["tools"][0]["declaration"]["tool_session"]["id"],
            events[2]["session"]
        );
        assert_eq!(events[0]["present"], json!([]));
        assert_eq!(
            events[2]["present"],
            json!(["SCRIPTED_KEY", "SCRIPTED_MODEL_DIR"])
        );
        assert_eq!(state["attempts"], json!([]));
        assert_eq!(
            state["tools"][0]["declaration"]["document"]["actions"][0]["readiness"]["state"],
            "ready"
        );
        assert!(!host.tool.join("calls.ndjson").exists());
        for content in [
            state.to_string(),
            fs::read_to_string(host.root.join("tools.json")).unwrap(),
            fs::read_to_string(host.tool.join("startup.ndjson")).unwrap(),
        ] {
            assert!(!content.contains("synthetic-provider-value"));
            assert!(!content.contains("synthetic-model-value"));
            assert!(!content.contains("synthetic-extra-value"));
            assert!(!content.contains("synthetic-unrelated-value"));
        }
    }
}

#[test]
fn new_serving_requirements_are_visible_without_a_bootstrap_loop() {
    let mut host = Host::start("explicit", "configuration", true);
    host.http("POST", "/api/tools/sample/refresh", json!({}));
    let state = host.http("GET", "/api/state", Value::Null);
    let events = host.events();
    host.stop();
    assert_eq!(events.len(), 3);
    assert!(
        state["tools"][0]["error"]
            .as_str()
            .unwrap()
            .contains("Incomplete setup")
    );
    let requirements =
        state["tools"][0]["declaration"]["document"]["actions"][0]["readiness"]["requirements"]
            .as_array()
            .unwrap();
    assert!(
        requirements
            .iter()
            .any(|requirement| requirement["name"] == "SCRIPTED_EXTRA_KEY"
                && requirement["state"] == "missing")
    );
    assert_eq!(
        events[2]["present"],
        json!(["SCRIPTED_KEY", "SCRIPTED_MODEL_DIR"])
    );
    assert_eq!(state["attempts"], json!([]));
}

#[test]
fn failed_probe_exits_without_starting_a_serving_session() {
    let mut host = Host::start("explicit", "configuration-fails", false);
    let state = host.http("GET", "/api/state", Value::Null);
    let events = host.events();
    host.stop();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1]["event"], "exit");
    assert!(state["tools"][0]["declaration"].is_null());
    assert!(state["tools"][0]["error"].is_string());
    assert!(!host.tool.join("calls.ndjson").exists());
}

#[test]
fn missing_inherited_credentials_remain_missing_without_preventing_discovery() {
    let mut host = Host::start("explicit", "configuration-missing-key", false);
    let state = host.http("GET", "/api/state", Value::Null);
    let events = host.events();
    host.stop();
    assert_eq!(events.len(), 3);
    assert_eq!(events[2]["present"], json!(["SCRIPTED_MODEL_DIR"]));
    assert!(state["tools"][0]["error"].is_null());
    let readiness = &state["tools"][0]["declaration"]["document"]["actions"][0]["readiness"];
    assert_eq!(readiness["state"], "needs_setup");
    assert!(
        readiness["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|requirement| requirement["name"] == "SCRIPTED_KEY"
                && requirement["state"] == "missing")
    );
    assert_eq!(state["attempts"], json!([]));
}

#[test]
fn explicit_restart_reprobes_edited_config_before_selecting_environment() {
    let mut host = Host::start("explicit", "configuration", false);
    let before = host.http("GET", "/api/state", Value::Null);
    fs::write(
        host._temporary.path().join("selected config.json"),
        json!({"credential":"SCRIPTED_EXTRA_KEY", "env":"SCRIPTED_UNRELATED"}).to_string(),
    )
    .unwrap();
    let restarted = host.http(
        "POST",
        "/api/tools/sample/restart",
        json!({"confirmed":true}),
    );
    let events = host.events();
    host.stop();
    assert_eq!(events.len(), 7);
    assert_eq!(events[3]["event"], "exit");
    assert_eq!(
        events[3]["session"],
        before["tools"][0]["declaration"]["tool_session"]["id"]
    );
    assert_eq!(events[4]["present"], json!([]));
    assert_eq!(events[5]["event"], "exit");
    assert_eq!(
        events[6]["present"],
        json!(["SCRIPTED_EXTRA_KEY", "SCRIPTED_UNRELATED"])
    );
    assert_eq!(events[6]["session"], restarted["tool_session"]["id"]);
    assert!(!host.tool.join("calls.ndjson").exists());
}
