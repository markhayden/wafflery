use std::{
    io::{Read, Write},
    net::TcpStream,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

struct Host(Child);
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start(root: &std::path::Path) -> Host {
    Host(
        Command::new(env!("CARGO_BIN_EXE_wafflehouse"))
            .args(["--foreground", "--no-open", "--data-dir"])
            .arg(root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}

fn ready(root: &std::path::Path) -> serde_json::Value {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(value) = wafflehouse::host::control::request(root, "status") {
            return value;
        }
        assert!(Instant::now() < deadline, "host never became ready");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn foreground_serves_embedded_ui_and_preserves_a_single_owner() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let _host = start(&root);
    let state = ready(&root);
    let origin = state["origin"].as_str().unwrap();
    assert!(origin.starts_with("http://127.0.0.1:"));
    let address = origin.strip_prefix("http://").unwrap();
    let mut connection = TcpStream::connect(address).unwrap();
    connection
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write!(
        connection,
        "GET / HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    connection.read_to_string(&mut response).unwrap();
    assert!(response.contains("200 OK"));
    assert!(response.contains("<title>Wafflehouse</title>"));
    let result = Command::new(env!("CARGO_BIN_EXE_wafflehouse"))
        .args(["--foreground", "--no-open", "--data-dir"])
        .arg(&root)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(69));
    assert_eq!(ready(&root), state);
}

#[test]
fn stale_socket_is_replaced_only_after_acquiring_the_lock() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let mut first = start(&root);
    ready(&root);
    first.0.kill().unwrap();
    first.0.wait().unwrap();
    let _second = start(&root);
    ready(&root);
}

#[test]
fn a_symlinked_data_root_is_refused_without_modifying_the_target() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let other = temp.path().join("other");
    std::fs::create_dir(&other).unwrap();
    std::os::unix::fs::symlink(&other, &root).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_wafflehouse"))
        .args(["--foreground", "--no-open", "--data-dir"])
        .arg(&root)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(std::fs::read_dir(other).unwrap().count(), 0);
}

#[test]
fn concurrent_launchers_attach_and_the_host_survives_their_exit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library with spaces");
    let spawn = || {
        Command::new(env!("CARGO_BIN_EXE_wafflehouse"))
            .args(["--no-open", "--tool", "unknown-tool", "--data-dir"])
            .arg(&root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    let first = spawn();
    let second = spawn();
    let a = first.wait_with_output().unwrap();
    let b = second.wait_with_output().unwrap();
    assert!(a.status.success(), "{}", String::from_utf8_lossy(&a.stderr));
    assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
    assert_eq!(a.stdout, b.stdout);
    ready(&root);
    wafflehouse::host::control::request(&root, "quit").unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while wafflehouse::host::control::request(&root, "status").is_ok() {
        assert!(Instant::now() < deadline, "host did not quit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn malformed_launcher_arguments_never_start_a_host() {
    for arguments in [
        vec!["--tool"],
        vec!["--tool", "../rascal"],
        vec!["--tool", "rascal", "--tool", "squint"],
        vec!["--no-open", "--no-open"],
        vec!["--unknown"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_wafflehouse"))
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(64));
    }
}

#[test]
fn an_incompatible_live_host_is_refused_without_starting_another() {
    use std::os::unix::net::UnixListener;
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    let owner = wafflehouse::host::control::Ownership::acquire(&data).unwrap();
    let listener = UnixListener::bind(&owner.socket).unwrap();
    owner.publish().unwrap();
    let root = owner.root.clone();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = String::new();
        stream.read_to_string(&mut request).unwrap();
        let reply = serde_json::json!({"protocol": 1, "root": root, "version": "incompatible", "origin": "http://127.0.0.1:1"});
        stream.write_all(reply.to_string().as_bytes()).unwrap();
    });
    let result = Command::new(env!("CARGO_BIN_EXE_wafflehouse"))
        .args(["--no-open", "--data-dir"])
        .arg(&data)
        .output()
        .unwrap();
    server.join().unwrap();
    assert_eq!(result.status.code(), Some(69));
    assert!(String::from_utf8_lossy(&result.stderr).contains("incompatible host"));
    assert!(owner.socket.exists());
}

#[test]
fn launchers_with_a_different_temporary_directory_attach_to_the_existing_host() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("data");
    let _host = start(&root);
    let state = ready(&root);
    let alternate = temp.path().join("different-temp");
    std::fs::create_dir(&alternate).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_wafflehouse"))
        .args(["--no-open", "--data-dir"])
        .arg(&root)
        .env("TMPDIR", &alternate)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        format!("{}/", state["origin"].as_str().unwrap())
    );
}
