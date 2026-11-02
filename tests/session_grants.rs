#[path = "../src/tools/session_grants.rs"]
mod grants;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink, path::Path};
use wafflery::tools::{Error, Result};

fn session(paths: &[&Path]) -> Value {
    json!({"id":"11111111-1111-4111-8111-111111111111","allowed_dirs":paths})
}

#[test]
fn effective_access_can_narrow_or_deny_but_cannot_add_a_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let requested = root.join("inputs");
    let narrowed = requested.join("approved");
    let sibling = root.join("inputs-other");
    fs::create_dir_all(&narrowed).unwrap();
    fs::create_dir(&sibling).unwrap();
    for paths in [vec![requested.as_path()], vec![narrowed.as_path()], vec![]] {
        grants::validate(&session(&paths), std::slice::from_ref(&requested), None).unwrap();
    }
    for path in [&root, &sibling] {
        assert!(
            grants::validate(&session(&[path]), std::slice::from_ref(&requested), None).is_err()
        );
    }
    assert!(grants::validate(&session(&[&requested]), &[], None).is_err());
    grants::validate(&session(&[]), &[], None).unwrap();
}

#[test]
fn effective_directories_must_be_canonical_existing_directories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let input = root.join("inputs");
    let outside = root.join("outside");
    fs::create_dir(&input).unwrap();
    fs::create_dir(&outside).unwrap();
    let file = input.join("file");
    fs::write(&file, b"not a directory").unwrap();
    let link = input.join("escape");
    symlink(&outside, &link).unwrap();
    for path in [
        link,
        file,
        input.join("missing"),
        input.join("."),
        input.join("../outside"),
        Path::new("relative").to_path_buf(),
    ] {
        assert!(
            grants::validate(&session(&[&path]), std::slice::from_ref(&input), None).is_err(),
            "{path:?}"
        );
    }
}

#[test]
fn an_observation_cannot_change_access_or_identity_inside_a_session() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let a = root.join("a");
    let b = root.join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let requested = [root];
    let before = session(&[&a, &b]);
    // Lists describe a set: a harmless reordering is not a permission change.
    grants::validate(&session(&[&b, &a]), &requested, Some(&before)).unwrap();
    assert!(grants::validate(&session(&[&a]), &requested, Some(&before)).is_err());
    let mut replacement = session(&[&a]);
    replacement["id"] = json!("22222222-2222-4222-8222-222222222222");
    assert!(grants::validate(&replacement, &requested, Some(&before)).is_err());
    // Only an explicitly new client discards the old observation.
    grants::validate(&replacement, &requested, None).unwrap();
    assert_eq!(before["allowed_dirs"], json!([a, b]));
}
