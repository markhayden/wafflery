#[path = "../testkit/v04_store.rs"]
mod store;
#[path = "../testkit/v04_fixtures.rs"]
mod v04_fixtures;

use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn inline(document: &Value) -> Value {
    let media = if document["artifact"]["type"] == "location" {
        "application/json".to_string()
    } else {
        format!(
            "application/x-{}-token",
            document["tool"]["name"].as_str().unwrap()
        )
    };
    let bytes = serde_json::to_vec(document).unwrap();
    json!({"inline":{"media_type":media,"data":STANDARD.encode(&bytes)},"sha256":hash(&bytes)})
}
fn new_store(root: &Path, grants: &[&Path]) -> store::Store {
    store::Store::new(
        root,
        "sample-lab",
        grants.iter().map(|p| p.canonicalize().unwrap()).collect(),
    )
    .unwrap()
}
fn invalid(error: Value) {
    assert_eq!(error["code"], -32602);
    assert_eq!(error["data"]["reason"], "invalid_arguments");
    assert_eq!(error["data"]["run"], Value::Null);
    assert_eq!(error["data"]["error"]["retryable"], false);
}

#[test]
fn controls_preserve_bytes_and_identity_across_restart_and_workspace_move() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path().join("before");
    std::fs::create_dir(&root).unwrap();
    let store = new_store(&root, &[]);
    // Exercise the independently authored documents, not a real peer's portable form.
    for (_, document) in v04_fixtures::documents("sample-lab")
        .into_iter()
        .filter(|(schema, d)| {
            *schema == "urn:table-manners:operations:artifact"
                && (d["artifact"]["type"] == "location"
                    || d["artifact"]["type"] == "sample-lab:Token")
        })
    {
        let imported = store.import(inline(&document)).unwrap();
        assert_eq!(imported["already_present"], false);
        assert_eq!(imported["artifact"], document["artifact"]);
        let again = store.import(inline(&document)).unwrap();
        assert_eq!(again["already_present"], true);
        let reference = &imported["artifact"];
        assert_eq!(
            store.read(reference["identity"].as_str().unwrap()).unwrap(),
            document
        );
        let preview = store
            .preview(reference["identity"].as_str().unwrap())
            .unwrap();
        assert_eq!(hash(&preview), document["preview"]["image"]["file_sha256"]);
        assert_eq!(image::load_from_memory(&preview).unwrap().width(), 160);
        let exported = store.export(json!({"artifact":reference})).unwrap();
        let bytes = std::fs::read(exported["file"]["path"].as_str().unwrap()).unwrap();
        assert_eq!(hash(&bytes), exported["file"]["sha256"]);
        assert_eq!(
            bytes.len(),
            exported["file"]["bytes"].as_u64().unwrap() as usize
        );
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), document);
        assert!(
            Path::new(exported["file"]["path"].as_str().unwrap())
                .starts_with(root.canonicalize().unwrap().join("exports"))
        );
        assert_eq!(
            store.export(json!({"artifact":reference})).unwrap(),
            exported
        );
        let reopened = new_store(&root, &[]);
        assert_eq!(
            reopened.import(inline(&document)).unwrap()["already_present"],
            true
        );
        let destination = tempfile::tempdir().unwrap();
        let other = new_store(destination.path(), &[&root]);
        let request =
            json!({"file":{"path":exported["file"]["path"]},"sha256":exported["file"]["sha256"]});
        assert_eq!(other.import(request).unwrap()["artifact"], *reference);
    }
    drop(store);
    let moved = workspace.path().join("after");
    std::fs::rename(&root, &moved).unwrap();
    let reopened = new_store(&moved, &[]);
    let document = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    assert_eq!(
        reopened
            .read(document["artifact"]["identity"].as_str().unwrap())
            .unwrap(),
        document
    );
    assert_eq!(
        std::fs::read_dir(moved.join("artifacts")).unwrap().count(),
        2
    );
    assert!(!moved.join("runs").exists());
}

#[test]
fn corrupt_or_wrong_imports_never_write_an_artifact() {
    let root = tempfile::tempdir().unwrap();
    let store = new_store(root.path(), &[]);
    let good = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    let mut wrong_digest = inline(&good);
    wrong_digest["sha256"] = json!("0".repeat(64));
    let mut wrong_identity = good.clone();
    wrong_identity["artifact"]["identity"] = json!(format!("sha256:{}", "0".repeat(64)));
    let mut wrong_record = good.clone();
    wrong_record["record"]["payload"] = json!("corrupted");
    let mut wrong_preview = good.clone();
    wrong_preview["preview"]["image"]["file_sha256"] = json!("0".repeat(64));
    let mut wrong_version = good.clone();
    wrong_version["artifact"]["schema_version"] = json!(99);
    let mut wrong_media = inline(&good);
    wrong_media["inline"]["media_type"] = json!("image/png");
    for request in [
        wrong_digest,
        inline(&wrong_identity),
        inline(&wrong_record),
        inline(&wrong_preview),
        inline(&wrong_version),
        wrong_media,
        inline(&v04_fixtures::artifact("test-prism", "test-prism:Token")),
        inline(&v04_fixtures::artifact("sample-lab", "sample-lab:Memo")),
        json!({"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}}),
        json!({"inline":{"media_type":"application/json","data":STANDARD.encode(b"{truncated")}}),
        json!({"inline":{"media_type":"application/json","data":STANDARD.encode(vec![b' ';1048577])}}),
        json!({"inline":{"media_type":"application/json","data":"not-base64"}}),
        json!({"inline":{"media_type":"application/json","data":"e30="},"file":{"path":"/unused"}}),
    ] {
        invalid(store.import(request).unwrap_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[test]
fn file_imports_need_canonical_grants_and_regular_files() {
    let root = tempfile::tempdir().unwrap();
    let allowed = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let document = v04_fixtures::artifact("sample-lab", "location");
    let path = outside.path().join("marker.json");
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    let alias = allowed.path().join("escape.json");
    std::os::unix::fs::symlink(&path, &alias).unwrap();
    let store = new_store(root.path(), &[allowed.path()]);
    let missing = outside.path().join("missing.json");
    let dangling = allowed.path().join("dangling.json");
    std::os::unix::fs::symlink(&missing, &dangling).unwrap();
    let outside_error = store.import(json!({"file":{"path":path}})).unwrap_err();
    for hidden in [&missing, &dangling, &alias] {
        assert_eq!(
            store.import(json!({"file":{"path":hidden}})).unwrap_err(),
            outside_error
        );
    }
    for path in [
        path.as_path(),
        alias.as_path(),
        allowed.path(),
        Path::new("relative.json"),
    ] {
        invalid(store.import(json!({"file":{"path":path}})).unwrap_err());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
    let accepted = allowed.path().join("local marker.json");
    std::fs::write(&accepted, serde_json::to_vec(&document).unwrap()).unwrap();
    invalid(
        new_store(root.path(), &[])
            .import(json!({"file":{"path":accepted}}))
            .unwrap_err(),
    );
    assert_eq!(
        store.import(json!({"file":{"path":accepted}})).unwrap()["artifact"],
        document["artifact"]
    );
}

#[test]
fn missing_references_are_structured_and_memos_are_not_exportable() {
    let root = tempfile::tempdir().unwrap();
    let store = new_store(root.path(), &[]);
    let document = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    let identity = document["artifact"]["identity"].as_str().unwrap();
    for (error, port) in [
        (store.read(identity).unwrap_err(), Value::Null),
        (
            store
                .export(json!({"artifact":document["artifact"]}))
                .unwrap_err(),
            json!("artifact"),
        ),
    ] {
        assert_eq!(error["code"], -32602);
        assert_eq!(error["data"]["reason"], "binding");
        assert_eq!(error["data"]["bindings"][0]["port"], port);
        assert_eq!(error["data"]["bindings"][0]["problem"], "unresolved");
        assert!(error["data"]["bindings"][0]["remedy"].is_string());
    }
    let memo = v04_fixtures::artifact("sample-lab", "sample-lab:Memo");
    store.remember(&memo).unwrap();
    assert_eq!(
        store
            .read(memo["artifact"]["identity"].as_str().unwrap())
            .unwrap()["preview"],
        Value::Null
    );
    invalid(
        store
            .export(json!({"artifact":memo["artifact"]}))
            .unwrap_err(),
    );
    assert!(!root.path().join("exports").exists());
}

#[test]
fn shared_location_can_change_tool_owner_but_keeps_identity() {
    let root = tempfile::tempdir().unwrap();
    let store = new_store(root.path(), &[]);
    let foreign = v04_fixtures::artifact("test-prism", "location");
    let local = v04_fixtures::artifact("sample-lab", "location");
    let imported = store.import(inline(&foreign)).unwrap();
    assert_eq!(imported["artifact"], local["artifact"]);
    assert_eq!(
        imported["artifact"]["identity"],
        foreign["artifact"]["identity"]
    );
}

#[test]
fn concurrent_duplicate_imports_are_idempotent() {
    let root = tempfile::tempdir().unwrap();
    let store = new_store(root.path(), &[]);
    for n in 0..12 {
        let document =
            v04_fixtures::artifact_variant("sample-lab", "sample-lab:Token", f64::from(n) / 12.0);
        let barrier = std::sync::Barrier::new(4);
        let outcomes = std::thread::scope(|scope| {
            let tasks: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        store.import(inline(&document)).unwrap()["already_present"]
                            .as_bool()
                            .unwrap()
                    })
                })
                .collect();
            tasks
                .into_iter()
                .map(|t| t.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert_eq!(outcomes.iter().filter(|present| !**present).count(), 1);
    }
    assert_eq!(
        std::fs::read_dir(root.path().join("artifacts"))
            .unwrap()
            .count(),
        12
    );
}

#[test]
fn corrupt_store_entry_is_an_internal_error_on_import_not_a_binding_refusal() {
    let root = tempfile::tempdir().unwrap();
    let store = new_store(root.path(), &[]);
    let document = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    store.import(inline(&document)).unwrap();
    let identity = document["artifact"]["identity"].as_str().unwrap();
    let path = root
        .path()
        .join("artifacts")
        .join(identity.strip_prefix("sha256:").unwrap());
    std::fs::write(&path, b"corrupt").unwrap();
    assert_eq!(
        store.read(identity).unwrap_err()["data"]["reason"],
        "binding"
    );
    assert_eq!(store.import(inline(&document)).unwrap_err()["code"], -32603);
    assert_eq!(std::fs::read(&path).unwrap(), b"corrupt");
}

#[test]
fn store_rejects_noncanonical_or_invalid_grant_inputs() {
    let root = tempfile::tempdir().unwrap();
    for grant in [
        std::path::PathBuf::new(),
        "relative".into(),
        root.path().join("missing"),
    ] {
        assert!(store::Store::new(root.path(), "sample-lab", vec![grant]).is_err());
    }
}
