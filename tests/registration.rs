use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};
use wafflehouse::{host::control::Ownership, tools::registration};

fn descriptor(dir: &Path, id: &str) -> Value {
    let executable = dir.join("tool with spaces");
    fs::write(&executable, "#!/bin/sh\ntouch should-never-run\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    json!({"id":id,"executable":executable,"args":["literal $(touch not-a-shell)"],"cwd":dir,"roots":[dir]})
}
fn input(dir: &Path, value: Value) -> std::path::PathBuf {
    let path = dir.join("descriptors.json");
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    path
}

#[test]
fn appends_private_records_idempotently_without_executing_them() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("personal library");
    let mut first = descriptor(dir.path(), "one");
    let link = dir.path().join("linked executable");
    symlink(first["executable"].as_str().unwrap(), &link).unwrap();
    first["executable"] = json!(link);
    let source = input(dir.path(), json!([first]));
    assert_eq!(registration::register(&root, &source).unwrap(), 1);
    let saved = fs::read(root.join("tools.json")).unwrap();
    assert_eq!(registration::register(&root, &source).unwrap(), 0);
    assert_eq!(fs::read(root.join("tools.json")).unwrap(), saved);
    let source = input(dir.path(), json!([descriptor(dir.path(), "two")]));
    assert_eq!(registration::register(&root, &source).unwrap(), 1);
    assert_eq!(
        registration::load(&root.join("tools.json")).unwrap().len(),
        2
    );
    assert_eq!(
        fs::metadata(root.join("tools.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(!dir.path().join("should-never-run").exists());
    assert!(!dir.path().join("not-a-shell").exists());
    assert!(!root.join("host.json").exists());
}

#[test]
fn conflicts_busy_hosts_and_symlinks_leave_existing_bytes_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("data");
    let first = descriptor(dir.path(), "one");
    let source = input(dir.path(), json!([first.clone()]));
    registration::register(&root, &source).unwrap();
    let saved = fs::read(root.join("tools.json")).unwrap();
    let owner = Ownership::acquire(&root).unwrap();
    assert_eq!(
        registration::register(&root, &source).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    drop(owner);
    let mut changed = first.clone();
    changed["args"] = json!(["different"]);
    let source = input(dir.path(), json!([descriptor(dir.path(), "two"), changed]));
    assert!(registration::register(&root, &source).is_err());
    assert_eq!(fs::read(root.join("tools.json")).unwrap(), saved);
    let source = input(dir.path(), json!([first]));
    fs::rename(root.join("tools.json"), root.join("original.json")).unwrap();
    symlink(root.join("original.json"), root.join("tools.json")).unwrap();
    assert!(registration::register(&root, &source).is_err());
    assert_eq!(fs::read(root.join("original.json")).unwrap(), saved);
}

#[test]
fn invalid_input_never_creates_a_library() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("data");
    let d = descriptor(dir.path(), "valid");
    for value in [
        json!([]),
        json!([d.clone(), d.clone()]),
        json!([{"id":"bad"}]),
        json!([{"id":"extra", "env":{"SECRET":"no"}}]),
    ] {
        assert!(registration::register(&root, &input(dir.path(), value)).is_err());
        assert!(!root.exists());
    }
    let source = input(dir.path(), json!([d.clone()]));
    fs::set_permissions(
        d["executable"].as_str().unwrap(),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    assert!(registration::register(&root, &source).is_err());
    fs::write(&source, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(registration::register(&root, &source).is_err());
    let link = dir.path().join("source-link");
    symlink(&source, &link).unwrap();
    assert!(registration::register(&root, &link).is_err());
    assert!(!root.exists());
}
