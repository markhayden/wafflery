use serde_json::json;
use std::{fs, process::Command};

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wafflehouse"));
    command.env_remove("WAFFLEHOUSE_DATA_DIR");
    command
}

#[test]
fn registration_is_setup_only_and_explicit_data_dir_wins() {
    let dir = tempfile::tempdir().unwrap();
    let descriptor = dir.path().join("local tools.json");
    fs::write(
        &descriptor,
        json!([{"id":"example","executable":env!("CARGO_BIN_EXE_wafflehouse"),
        "args":["--version"],"cwd":dir.path(),"roots":[dir.path()]}])
        .to_string(),
    )
    .unwrap();
    let environment_root = dir.path().join("environment");
    let explicit_root = dir.path().join("explicit");
    let result = cli()
        .env("WAFFLEHOUSE_DATA_DIR", &environment_root)
        .arg("--register")
        .arg(&descriptor)
        .arg("--data-dir")
        .arg(&explicit_root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!environment_root.exists());
    assert!(explicit_root.join("tools.json").exists());
    assert!(!explicit_root.join("host.json").exists());
    let result = cli()
        .env("WAFFLEHOUSE_DATA_DIR", &environment_root)
        .arg("--register")
        .arg(&descriptor)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(environment_root.join("tools.json").exists());
    assert!(!environment_root.join("host.json").exists());
    assert!(!environment_root.join("Library").exists());
    let result = cli()
        .env("WAFFLEHOUSE_DATA_DIR", "")
        .arg("--register")
        .arg(&descriptor)
        .arg("--data-dir")
        .arg(&explicit_root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "Explicit selection must override the environment"
    );
}

#[test]
fn invalid_setup_combinations_and_bad_environment_never_launch() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("data");
    for arguments in [
        vec!["--register"],
        vec!["--register", "missing", "--no-open"],
        vec!["--register", "missing", "--tool", "rascal"],
        vec!["--register", "missing", "--foreground"],
        vec!["--register", "missing", "--internal-host"],
        vec!["--register", "one", "--register", "two"],
    ] {
        let result = cli()
            .env("WAFFLEHOUSE_DATA_DIR", &root)
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(64));
        assert!(!root.exists());
    }
    for invalid in ["", "relative"] {
        let result = cli()
            .env("WAFFLEHOUSE_DATA_DIR", invalid)
            .arg("--no-open")
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(64));
        assert!(String::from_utf8_lossy(&result.stderr).contains("WAFFLEHOUSE_DATA_DIR"));
    }
}
