use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use wafflery::host::control;

struct Host {
    child: Child,
    stdout: Arc<Mutex<String>>,
    stderr: Arc<Mutex<String>>,
    readers: Vec<std::thread::JoinHandle<()>>,
    tool_pids: Option<PathBuf>,
}
impl Host {
    fn spawn(root: &Path) -> Self {
        Self::with_output(root, true)
    }
    fn with_output(root: &Path, capture_output: bool) -> Self {
        let mut child = wafflery::process::spawn_std(
            Command::new(env!("CARGO_BIN_EXE_wafflery"))
                .args(["start", "--no-open", "--data-dir"])
                .arg(root)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .process_group(0),
        )
        .unwrap();
        let stdout = Arc::new(Mutex::new(String::new()));
        let stderr = Arc::new(Mutex::new(String::new()));
        let readers = if capture_output {
            vec![
                capture(child.stdout.take().unwrap(), stdout.clone()),
                capture(child.stderr.take().unwrap(), stderr.clone()),
            ]
        } else {
            drop(child.stdout.take());
            drop(child.stderr.take());
            vec![]
        };
        Self {
            child,
            stdout,
            stderr,
            readers,
            tool_pids: None,
        }
    }
    fn ready(&mut self, root: &Path) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Ok(state) = control::request(root, "status") {
                return state;
            }
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "{}",
                self.stderr.lock().unwrap()
            );
            assert!(Instant::now() < deadline, "host did not become ready");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn interrupt(&mut self) {
        self.signal(nix::sys::signal::Signal::SIGINT);
    }
    fn signal(&mut self, signal: nix::sys::signal::Signal) {
        nix::sys::signal::killpg(nix::unistd::Pid::from_raw(self.child.id() as i32), signal)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "{}", self.stderr.lock().unwrap());
                break;
            }
            assert!(
                Instant::now() < deadline,
                "shutdown did not finish: {}",
                self.stderr.lock().unwrap()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        for reader in self.readers.drain(..) {
            reader.join().unwrap();
        }
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        // Only disposable processes launched by this fixture are eligible for failed-test cleanup.
        if let Some(path) = &self.tool_pids {
            for pid in fs::read_to_string(path).unwrap_or_default().lines() {
                if let Ok(pid) = pid.parse() {
                    let _ = nix::sys::signal::kill(
                        nix::unistd::Pid::from_raw(pid),
                        nix::sys::signal::Signal::SIGKILL,
                    );
                }
            }
        }
    }
}
fn capture(
    pipe: impl Read + Send + 'static,
    output: Arc<Mutex<String>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines() {
            let mut output = output.lock().unwrap();
            output.push_str(&line.unwrap());
            output.push('\n');
        }
    })
}

#[test]
fn start_stays_attached_prints_a_bootstrap_and_logs_request_errors_without_secrets() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("data");
    let mut host = Host::spawn(&root);
    let state = host.ready(&root);
    let origin = state["origin"].as_str().unwrap();
    for (path, status) in [
        ("/api/state?secret=do-not-log", "401 Unauthorized"),
        (
            "/views/frame-credential-do-not-log?ignored",
            "400 Bad Request",
        ),
    ] {
        let mut request = TcpStream::connect(origin.strip_prefix("http://").unwrap()).unwrap();
        request
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            request,
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            origin.strip_prefix("http://").unwrap()
        )
        .unwrap();
        let mut response = String::new();
        request.read_to_string(&mut response).unwrap();
        assert!(response.contains(status));
    }
    assert!(host.child.try_wait().unwrap().is_none());
    host.interrupt();
    let stdout = host.stdout.lock().unwrap();
    let url = stdout
        .lines()
        .find(|line| line.starts_with(origin))
        .expect("bootstrap URL printed");
    assert_eq!(
        url.strip_prefix(&format!("{origin}/#bootstrap="))
            .unwrap()
            .len(),
        64
    );
    let stderr = host.stderr.lock().unwrap();
    assert!(
        stderr.contains(&format!("Listening at {origin}")),
        "{stderr}"
    );
    assert!(stderr.contains("GET /api/state: 401"), "{stderr}");
    assert!(stderr.contains("GET /views/{token}: 400"), "{stderr}");
    assert!(!stderr.contains("do-not-log"));
    assert!(stderr.contains("Shutdown complete"), "{stderr}");
    assert!(control::request(&root, "status").is_err());
}

#[test]
fn start_forwards_prefixed_tool_stderr_and_reaps_every_launched_child_on_sigint() {
    let built = Command::new(env!("CARGO"))
        .args(["build", "--locked", "--example", "sample-lab"])
        .status()
        .unwrap();
    assert!(built.success());
    let peer = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab");
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("data");
    let wrapper = temporary.path().join("tool with spaces");
    let pids = temporary.path().join("pids");
    // Use positional arguments rather than interpolating shell paths.
    fs::write(&wrapper, "#!/bin/sh\nprintf '%s\\n' \"$$\" >> \"$1\"\nshift\nprintf 'diagnostic line\\n' >&2\n\"$@\"\nstatus=$?\n/bin/sleep 0.15\n/usr/bin/printf '%100000s\\n' 'shutdown diagnostics' >&2\nexit \"$status\"\n").unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    let tools = ["rascal", "squint"].map(|id| {
        json!({
            "id":id, "executable":wrapper, "cwd":temporary.path(),
            "args":[pids, peer, temporary.path().join(id), "normal"],
            "allow_dirs":[temporary.path()], "evidence_dirs":[temporary.path()]
        })
    });
    let mut tools = tools.to_vec();
    tools.push(
        json!({"id":"broken", "executable":"/bin/sh", "cwd":temporary.path(),
        "args":["-c", "printf 'synthetic early failure' >&2; exit 7"],
        "allow_dirs":[], "evidence_dirs":[]}),
    );
    let source = temporary.path().join("registration.json");
    fs::write(&source, serde_json::to_vec(&tools).unwrap()).unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_wafflery"))
            .arg("--register")
            .arg(source)
            .arg("--data-dir")
            .arg(&root)
            .status()
            .unwrap()
            .success()
    );
    let mut host = Host::spawn(&root);
    host.tool_pids = Some(pids.clone());
    host.ready(&root);
    assert!(host.child.try_wait().unwrap().is_none());
    host.interrupt();
    let stderr = host.stderr.lock().unwrap();
    for id in ["rascal", "squint"] {
        assert!(
            stderr.contains(&format!("[{id}] diagnostic line")),
            "{stderr}"
        );
        assert!(stderr.contains(&format!("Tool {id} (pid ")), "{stderr}");
    }
    assert!(
        stderr.contains("[broken] synthetic early failure"),
        "{stderr}"
    );
    assert!(stderr.contains("exited: exit status: 7"), "{stderr}");
    let pids = fs::read_to_string(pids).unwrap();
    assert_eq!(
        pids.lines().count(),
        4,
        "both tools must finish their probe and start a serving session"
    );
    for pid in pids.lines() {
        assert!(
            stderr.contains(&format!("pid {pid}) exited: exit status: 0")),
            "{stderr}"
        );
        assert_eq!(
            nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid.parse().unwrap()), None),
            Err(nix::errno::Errno::ESRCH)
        );
    }
    host.tool_pids = None;
}

#[test]
fn closed_terminal_outputs_do_not_prevent_interrupt_or_hangup_shutdown() {
    for signal in [
        nix::sys::signal::Signal::SIGINT,
        nix::sys::signal::Signal::SIGHUP,
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("data");
        let mut host = Host::with_output(&root, false);
        host.ready(&root);
        host.signal(signal);
        assert!(control::request(&root, "status").is_err());
    }
}

#[test]
fn start_rejects_other_modes_and_misplaced_subcommands_without_launching() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("data");
    for args in [
        vec!["--foreground"],
        vec!["start", "--tool", "rascal"],
        vec!["start", "--internal-host"],
        vec!["start", "start"],
        vec!["--no-open", "start"],
        vec!["start", "--register", "missing"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_wafflery"))
            .args(args)
            .arg("--data-dir")
            .arg(&root)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(64));
        assert!(!root.exists());
    }
}
