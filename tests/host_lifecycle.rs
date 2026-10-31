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
