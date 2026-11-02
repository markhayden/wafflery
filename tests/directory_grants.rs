use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};
use wafflery::tools::{client::ToolClient, registration::Descriptor};

fn record(root: &Path) -> Value {
    let executable = root.join("unrelated tool");
    fs::write(&executable, "#!/bin/sh\nprintf '%s\\n' \"$@\" > argv.txt\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    json!({"id":"unrelated", "executable":executable, "args":["serve"], "cwd":root,
        "allow_dirs":[], "evidence_dirs":[]})
}

#[test]
fn directory_fields_are_explicit_independent_and_canonical() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input");
    let output = root.path().join("output");
    fs::create_dir(&input).unwrap();
    fs::create_dir(&output).unwrap();
    let link = root.path().join("input-link");
    symlink(&input, &link).unwrap();
    let mut value = record(root.path());
    value["allow_dirs"] = json!([link]);
    value["evidence_dirs"] = json!([output]);
    let descriptor: Descriptor = serde_json::from_value(value).unwrap();
    let saved = serde_json::to_value(descriptor.validate().unwrap()).unwrap();
    assert_eq!(saved["allow_dirs"], json!([input.canonicalize().unwrap()]));
    assert_eq!(
        saved["evidence_dirs"],
        json!([output.canonicalize().unwrap()])
    );
    assert!(saved.get("roots").is_none());
}

#[test]
fn legacy_roots_missing_fields_and_handwritten_grants_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let value = record(root.path());
    let mut old = value.clone();
    old.as_object_mut().unwrap().remove("allow_dirs");
    old.as_object_mut().unwrap().remove("evidence_dirs");
    old["roots"] = json!([root.path()]);
    assert!(serde_json::from_value::<Descriptor>(old.clone()).is_err());
    let legacy = root.path().join("old-tools.json");
    fs::write(&legacy, serde_json::to_vec(&json!([old])).unwrap()).unwrap();
    let error = wafflery::tools::registration::load(&legacy)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("README") && error.contains("backup"),
        "{error}"
    );
    for name in ["allow_dirs", "evidence_dirs"] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(name);
        assert!(serde_json::from_value::<Descriptor>(missing).is_err());
    }
    for args in [
        json!(["serve", "--allow-dir", root.path()]),
        json!(["serve", "--allow-dir=/tmp"]),
    ] {
        let mut conflict = value.clone();
        conflict["args"] = args;
        let descriptor: Descriptor = serde_json::from_value(conflict).unwrap();
        assert!(
            descriptor
                .validate()
                .unwrap_err()
                .to_string()
                .contains("--allow-dir")
        );
    }
}

#[test]
fn both_directory_lists_reject_relative_missing_and_file_paths() {
    let root = tempfile::tempdir().unwrap();
    let value = record(root.path());
    for field in ["allow_dirs", "evidence_dirs"] {
        for path in [
            ".".into(),
            root.path().join("missing"),
            root.path().join("unrelated tool"),
        ] {
            let mut invalid = value.clone();
            invalid[field] = json!([path]);
            let descriptor: Descriptor = serde_json::from_value(invalid).unwrap();
            assert!(descriptor.validate().is_err(), "{field}");
        }
    }
}

#[test]
fn directory_grants_refuse_non_utf8_names() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(OsString::from_vec(b"input-\xff".to_vec()));
    for field in ["allow_dirs", "evidence_dirs"] {
        let mut descriptor: Descriptor = serde_json::from_value(record(root.path())).unwrap();
        if field == "allow_dirs" {
            descriptor.allow_dirs.push(target.clone());
        } else {
            descriptor.evidence_dirs.push(target.clone());
        }
        let error = descriptor
            .validate()
            .expect_err("unrepresentable grant accepted");
        assert!(error.to_string().contains("UTF-8"), "{field}: {error}");
    }
}

// macOS rejects creation of these names; Linux also needs the post-canonicalization check.
#[cfg(target_os = "linux")]
#[test]
fn directory_grants_refuse_symlinks_to_non_utf8_names() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join(OsString::from_vec(b"input-\xff".to_vec()));
    fs::create_dir(&target).unwrap();
    let link = root.path().join("readable-link");
    symlink(&target, &link).unwrap();
    for field in ["allow_dirs", "evidence_dirs"] {
        let mut value = record(root.path());
        value[field] = json!([link]);
        let descriptor: Descriptor = serde_json::from_value(value).unwrap();
        let error = descriptor
            .validate()
            .expect_err("unrepresentable grant accepted");
        assert!(error.to_string().contains("UTF-8"), "{field}: {error}");
    }
}

#[tokio::test]
async fn child_receives_only_input_grants_as_literal_arguments() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("input with spaces $(literal)");
    let second = root.path().join("second");
    let output = root.path().join("evidence only");
    for path in [&input, &second, &output] {
        fs::create_dir(path).unwrap();
    }
    let mut value = record(root.path());
    value["allow_dirs"] = json!([input, second]);
    value["evidence_dirs"] = json!([output]);
    let client = ToolClient::spawn(serde_json::from_value(value).unwrap()).unwrap();
    assert!(client.close().await.unwrap());
    assert_eq!(
        fs::read_to_string(root.path().join("argv.txt")).unwrap(),
        format!(
            "serve\n--allow-dir\n{}\n--allow-dir\n{}\n",
            input.canonicalize().unwrap().display(),
            second.canonicalize().unwrap().display()
        )
    );
    let client = ToolClient::spawn(serde_json::from_value(record(root.path())).unwrap()).unwrap();
    assert!(client.close().await.unwrap());
    assert_eq!(
        fs::read_to_string(root.path().join("argv.txt")).unwrap(),
        "serve\n"
    );
}

#[tokio::test]
async fn host_grants_only_staging_and_explicit_inputs_and_reads_only_evidence() {
    use std::{sync::Arc, time::Duration};
    use wafflery::{library::assets::AssetStore, tools::coordinator::Coordinator};
    assert!(
        std::process::Command::new(env!("CARGO"))
            .args(["build", "--locked", "--example", "sample-lab"])
            .status()
            .unwrap()
            .success()
    );
    let executable = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab");
    for read_outputs in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let tool = root.join("tool");
        fs::create_dir(&tool).unwrap();
        let assets = AssetStore::open(&root.join("Library")).unwrap();
        let input = root.join("original.png");
        fs::write(&input, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let asset = assets.import_path(input).await.unwrap();
        let descriptor = serde_json::from_value(json!({"id":"unrelated", "executable":executable,
            "args":[tool,"normal"], "cwd":tool,
            "allow_dirs": if read_outputs { json!([]) } else { json!([tool]) },
            "evidence_dirs":if read_outputs { json!([tool]) } else { json!([]) }}))
        .unwrap();
        let host = Arc::new(Coordinator::open(assets, vec![descriptor]).await.unwrap());
        let description = host.description("unrelated").await.unwrap();
        let observed = host.snapshot().await.unwrap()["tools"][0]["directory_grants"].clone();
        let handshake: Value =
            serde_json::from_slice(&fs::read(tool.join("initialize.json")).unwrap()).unwrap();
        let grants: Value =
            serde_json::from_slice(&fs::read(tool.join("allowed-dirs.json")).unwrap()).unwrap();
        host.submit(serde_json::from_value(json!({"id":"boundary", "tool":"unrelated", "action":"mark_band",
            "revision":description.revision, "params":{"label":"grants"}, "images":{"source":[asset.id]}})).unwrap()).await.unwrap();
        let saved = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let saved = host.attempts.get("boundary").await.unwrap();
                if saved.preservation["contract"] != "pending" {
                    break saved;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let (quit, mut stopped) = tokio::sync::watch::channel(false);
        host.begin_shutdown(true, quit).await.unwrap();
        tokio::time::timeout(Duration::from_secs(8), stopped.wait_for(|done| *done))
            .await
            .unwrap()
            .unwrap();
        host.assets.database.shutdown().await.unwrap();
        assert!(handshake["capabilities"].get("roots").is_none());
        let mut expected = if read_outputs {
            vec![]
        } else {
            vec![tool.clone()]
        };
        expected.push(root.join("Library/execution"));
        assert_eq!(grants, json!(expected));
        assert_eq!(observed["requested_dirs"], json!(expected));
        assert_eq!(
            observed["evidence_dirs"],
            if read_outputs {
                json!([tool])
            } else {
                json!([])
            }
        );
        assert!(
            observed.get("allowed_dirs").is_none(),
            "requested is not an effective grant"
        );
        assert_eq!(saved.execution, "completed", "{:?}", saved.notices);
        if read_outputs {
            assert_eq!(saved.outputs.len(), 1);
            assert_eq!(saved.preservation["contract"], "complete");
        } else {
            assert!(saved.outputs.is_empty());
            assert!(
                saved
                    .notices
                    .iter()
                    .any(|message| message.contains("outside registered")),
                "{:?}",
                saved.notices
            );
        }
    }
}

#[tokio::test]
async fn automatic_staging_cannot_be_redirected_to_an_unrelated_directory() {
    use wafflery::{library::assets::AssetStore, tools::coordinator::Coordinator};
    let temp = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&temp.path().join("Library")).unwrap();
    let unrelated = temp.path().join("unrelated");
    fs::create_dir(&unrelated).unwrap();
    symlink(&unrelated, assets.root().join("execution")).unwrap();
    let result = Coordinator::open(assets.clone(), vec![]).await;
    let refused = result.is_err();
    if let Ok(host) = result {
        assert!(host.close().await);
    }
    assets.database.shutdown().await.unwrap();
    assert!(
        refused,
        "a staging symlink must not become an automatic grant"
    );
}

#[tokio::test]
async fn unsolicited_roots_requests_are_rejected_not_served_as_empty_grants() {
    use std::time::Duration;
    assert!(
        std::process::Command::new(env!("CARGO"))
            .args(["build", "--locked", "--example", "sample-lab"])
            .status()
            .unwrap()
            .success()
    );
    let root = tempfile::tempdir().unwrap();
    let mut value = record(root.path());
    value["executable"] =
        json!(Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab"));
    value["args"] = json!([root.path(), "unsolicited-roots"]);
    let mut client = ToolClient::spawn(serde_json::from_value(value).unwrap()).unwrap();
    client.initialize().await.unwrap();
    let observed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(bytes) = fs::read(root.path().join("roots-reply.json"))
                && let Ok(value) = serde_json::from_slice::<Value>(&bytes)
            {
                break value;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    assert!(client.close().await.unwrap());
    let reply = observed.expect("no response to the unsupported method");
    assert_eq!(reply["error"]["code"], -32601, "{reply}");
    assert!(reply.get("result").is_none());
}
