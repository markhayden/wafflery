use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use wafflery::host::control::Ownership;

fn descriptor(dir: &Path, id: &str, config: &str) -> Value {
    json!({"id":id,"executable":env!("CARGO_BIN_EXE_wafflery"),
        "args":["--version"],"cwd":dir,"roots":[dir],"config":dir.join(config)})
}
fn invoke(dir: &Path, root: &Path, records: Value, replace: bool) -> std::process::Output {
    let source = dir.join("registrations.json");
    fs::write(&source, records.to_string()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_wafflery"));
    command
        .args(["--register"])
        .arg(source)
        .arg("--data-dir")
        .arg(root);
    if replace {
        command.arg("--replace-registration");
    }
    command.output().unwrap()
}

#[test]
fn explicit_replacement_changes_only_existing_records_while_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("host");
    let one = descriptor(dir.path(), "one", "first.toml");
    let two = descriptor(dir.path(), "two", "other.toml");
    assert!(
        invoke(dir.path(), &root, json!([one.clone(), two.clone()]), false)
            .status
            .success()
    );
    let original = fs::read(root.join("tools.json")).unwrap();
    let changed = descriptor(dir.path(), "one", "changed config.toml");
    assert!(
        !invoke(dir.path(), &root, json!([changed.clone()]), false)
            .status
            .success()
    );
    assert_eq!(fs::read(root.join("tools.json")).unwrap(), original);
    assert!(
        invoke(dir.path(), &root, json!([changed.clone()]), true)
            .status
            .success()
    );
    let saved: Value = serde_json::from_slice(&fs::read(root.join("tools.json")).unwrap()).unwrap();
    assert_eq!(saved[0]["config"], changed["config"]);
    assert_eq!(saved[1]["config"], two["config"]);
    assert!(!root.join("host.json").exists());
    assert!(!root.join("Library").exists());
}

#[test]
fn replacement_rejects_unknown_ids_and_running_hosts_without_partial_changes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("host");
    let original = descriptor(dir.path(), "one", "original.toml");
    assert!(
        invoke(dir.path(), &root, json!([original]), false)
            .status
            .success()
    );
    let bytes = fs::read(root.join("tools.json")).unwrap();
    let changed = descriptor(dir.path(), "one", "changed.toml");
    let unknown = descriptor(dir.path(), "missing", "missing.toml");
    assert!(
        !invoke(dir.path(), &root, json!([changed.clone(), unknown]), true)
            .status
            .success()
    );
    assert_eq!(fs::read(root.join("tools.json")).unwrap(), bytes);
    let _owner = Ownership::acquire(&root).unwrap();
    let result = invoke(dir.path(), &root, json!([changed]), true);
    assert_eq!(result.status.code(), Some(69));
    assert_eq!(fs::read(root.join("tools.json")).unwrap(), bytes);
}

#[test]
fn replace_flag_requires_registration_and_cannot_be_duplicated() {
    for args in [
        vec!["--replace-registration"],
        vec![
            "--register",
            "missing",
            "--replace-registration",
            "--replace-registration",
        ],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_wafflery"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(64));
    }
}
