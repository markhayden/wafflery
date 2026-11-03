use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, OnceLock},
    time::Duration,
};
use wafflery::{
    library::{
        assets::{Asset, AssetStore},
        attempts::{Attempt, Submission},
    },
    tools::{coordinator::Coordinator, registration::Descriptor},
};
struct Host {
    root: tempfile::TempDir,
    c: Arc<Coordinator>,
    a: Asset,
    b: Asset,
}
impl Host {
    async fn new(scenario: &str) -> Self {
        static BINARY: OnceLock<PathBuf> = OnceLock::new();
        let binary = BINARY.get_or_init(|| {
            assert!(
                Command::new(env!("CARGO"))
                    .args(["build", "--locked", "--example", "sample-lab"])
                    .status()
                    .unwrap()
                    .success()
            );
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/examples/sample-lab")
        });
        let root = tempfile::tempdir().unwrap();
        let store = AssetStore::open(&root.path().join("Library")).unwrap();
        let source = root.path().join("original.png");
        std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let a = store.import_path(source).await.unwrap();
        let second = root.path().join("second.png");
        image::RgbaImage::from_pixel(8, 4, image::Rgba([12, 34, 56, 255]))
            .save(&second)
            .unwrap();
        let b = store.import_path(second).await.unwrap();
        let c = Arc::new(
            Coordinator::open(
                store,
                vec![Descriptor {
                    id: "fixture".into(),
                    executable: binary.clone(),
                    args: vec![
                        root.path().join("tool").display().to_string(),
                        scenario.into(),
                    ],
                    cwd: root.path().into(),
                    allow_dirs: vec![root.path().into()],
                    evidence_dirs: vec![root.path().into()],
                    config: None,
                }],
            )
            .await
            .unwrap(),
        );
        Self { root, c, a, b }
    }
    async fn request(&self, references: Option<Vec<String>>) -> Submission {
        let revision = self.c.description("fixture").await.unwrap().revision;
        let mut images = json!({"source":[self.a.id]});
        if let Some(ids) = references {
            images["references"] = json!(ids);
        }
        serde_json::from_value(json!({"id":"binding-test","tool":"fixture","action":"mark_band","revision":revision,"params":{"label":"List test"},"images":images})).unwrap()
    }
    async fn response(&self) -> Attempt {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let attempt = self.c.attempts.get("binding-test").await.unwrap();
                if attempt.raw_response.is_some() {
                    return attempt;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
    async fn settled(&self) -> Attempt {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let attempt = self.c.attempts.get("binding-test").await.unwrap();
                if !attempt.preservation["contract"].is_null()
                    && attempt.preservation["contract"] != "pending"
                {
                    return attempt;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
    async fn close(&self) {
        let (quit, mut stopped) = tokio::sync::watch::channel(false);
        self.c.begin_shutdown(true, quit).await.unwrap();
        tokio::time::timeout(Duration::from_secs(8), stopped.wait_for(|done| *done))
            .await
            .unwrap()
            .unwrap();
    }
    fn directory(&self) -> PathBuf {
        self.c.assets.root().join("execution/binding-test")
    }
}

#[tokio::test]
async fn list_bindings_keep_wire_order_duplicate_copies_and_frames() {
    let h = Host::new("image-list").await;
    let submitted =
        h.c.submit(
            h.request(Some(vec![h.b.id.clone(), h.a.id.clone(), h.b.id.clone()]))
                .await,
        )
        .await;
    if submitted.is_err() {
        h.close().await;
    }
    let submitted = submitted.unwrap();
    let refs = submitted.arguments["references"].as_array().unwrap();
    assert_eq!(refs.len(), 3);
    assert!(submitted.arguments["source"].is_object());
    assert_ne!(refs[0]["path"], refs[2]["path"]);
    assert_eq!(refs[0]["expect"]["file_sha256"], h.b.sha256);
    assert_eq!(refs[1]["expect"]["file_sha256"], h.a.sha256);
    assert_eq!(refs[2]["expect"], refs[0]["expect"]);
    h.response().await;
    let saved = h.settled().await;
    h.close().await;
    assert_eq!(saved.execution, "completed");
    assert_eq!(saved.preservation["contract"], "complete");
    assert_eq!(
        saved.wire.as_ref().unwrap()["params"]["arguments"]["references"],
        submitted.arguments["references"]
    );
    assert_eq!(saved.inputs["references"][0]["asset"]["id"], h.b.id);
    assert_eq!(saved.inputs["references"][1]["asset"]["id"], h.a.id);
    assert_eq!(saved.inputs["references"][2]["role"], "reference");
    assert!(
        !h.directory().exists(),
        "all scalar and list copies must be removed"
    );
    assert!(h.root.path().join("original.png").exists());
    assert!(h.c.assets.read(&h.b.id).await.is_ok());
}

#[tokio::test]
async fn omitted_optional_list_is_absent_and_one_item_still_uses_an_array() {
    for references in [false, true] {
        let h = Host::new("image-list").await;
        let result =
            h.c.submit(h.request(references.then(|| vec![h.b.id.clone()])).await)
                .await;
        if result.is_err() {
            h.close().await;
        }
        let attempt = result.unwrap();
        h.response().await;
        h.close().await;
        if references {
            assert!(attempt.arguments["references"].is_array());
        } else {
            assert!(attempt.arguments.get("references").is_none());
        }
        assert!(!h.directory().exists());
    }
}

#[tokio::test]
async fn empty_lists_missing_required_lists_and_scalar_overflow_never_dispatch() {
    for case in ["empty", "required", "scalar", "frames", "list-frames"] {
        let h = Host::new(if case == "required" {
            "image-list-required"
        } else {
            "image-list"
        })
        .await;
        let mut request = h
            .request(if case == "empty" { Some(vec![]) } else { None })
            .await;
        if case == "scalar" {
            request
                .images
                .get_mut("source")
                .unwrap()
                .push(h.b.id.clone());
        }
        if case == "frames" {
            request.frames.insert("source".into(), vec![]);
        }
        if case == "list-frames" {
            request
                .images
                .insert("references".into(), vec![h.b.id.clone(), h.b.id.clone()]);
            request
                .frames
                .insert("references".into(), vec![Default::default()]);
        }
        let result = h.c.submit(request).await;
        h.close().await;
        assert!(result.is_err(), "{case}");
        assert!(!h.directory().exists());
        assert!(!h.root.path().join("tool/calls.ndjson").exists());
    }
}

#[tokio::test]
async fn count_and_byte_limits_are_checked_before_reading_or_copying_any_image() {
    for case in ["count", "bytes", "file"] {
        let h = Host::new("image-list").await;
        let count = if case == "count" { 32 } else { 4 };
        let request = h.request(Some(vec![h.b.id.clone(); count])).await;
        if case != "count" {
            let size = if case == "file" { 129i64 } else { 128 } * 1024 * 1024;
            h.c.assets
                .database
                .call(move |db| {
                    db.execute("UPDATE assets SET size=?1", [size])?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        // A byte read would fail: limit preflight must win before touching this file.
        let source_path =
            h.c.assets
                .root()
                .join("assets/sha256")
                .join(&h.a.sha256[..2])
                .join(&h.a.sha256);
        std::fs::remove_file(source_path).unwrap();
        let result = h.c.submit(request).await;
        h.close().await;
        let message = result.unwrap_err().to_string();
        assert!(
            message.contains(match case {
                "count" => "32 images",
                "bytes" => "512 MiB",
                _ => "128 MiB",
            }),
            "{case}: {message}"
        );
        assert!(!h.directory().exists());
        assert!(!h.root.path().join("tool/calls.ndjson").exists());
    }
}

#[tokio::test]
async fn corrupt_later_input_removes_earlier_copies_without_dispatch() {
    let h = Host::new("image-list").await;
    let request = h.request(Some(vec![h.b.id.clone()])).await;
    let path =
        h.c.assets
            .root()
            .join("assets/sha256")
            .join(&h.b.sha256[..2])
            .join(&h.b.sha256);
    std::fs::remove_file(&path).unwrap();
    std::fs::write(path, b"corrupt fixture").unwrap();
    let result = h.c.submit(request).await;
    h.close().await;
    assert!(result.unwrap_err().to_string().contains("image"));
    assert!(!h.directory().exists());
    assert!(!h.root.path().join("tool/calls.ndjson").exists());
    assert!(h.c.assets.read(&h.a.id).await.is_ok());
}

#[tokio::test]
async fn list_frames_are_bound_per_occurrence_even_for_the_same_asset() {
    let h = Host::new("image-list").await;
    let mut request = h.request(Some(vec![h.b.id.clone(), h.b.id.clone()])).await;
    request.frames.insert(
        "references".into(),
        serde_json::from_value(json!([
            {"projection":"flat","topology":"bounded"},
            {"projection":"equirectangular","topology":"wrap"}
        ]))
        .unwrap(),
    );
    let result = h.c.submit(request).await;
    h.close().await;
    let attempt = result.unwrap();
    for (index, projection, topology) in [(0, "flat", "bounded"), (1, "equirectangular", "wrap")] {
        let reference = &attempt.arguments["references"][index]["expect"];
        assert_eq!(reference["file_sha256"], h.b.sha256);
        assert_eq!(reference["projection"], projection);
        assert_eq!(reference["topology"], topology);
        assert_eq!(&attempt.inputs["references"][index]["image"], reference);
    }
    assert!(!h.directory().exists());
}

#[tokio::test]
async fn partial_input_evidence_preserves_noncompleted_outcomes_with_notices() {
    for status in ["error", "invalid", "cancelled"] {
        for extent in ["partial", "empty"] {
            let h = Host::new(&format!("image-list-{extent}-{status}")).await;
            h.c.submit(
                h.request(Some(vec![h.b.id.clone(), h.a.id.clone(), h.b.id.clone()]))
                    .await,
            )
            .await
            .unwrap();
            let saved = h.settled().await;
            h.close().await;
            assert_eq!(saved.execution, status);
            assert_eq!(saved.result.as_ref().unwrap()["status"], status);
            assert_eq!(saved.preservation["contract"], "incomplete");
            assert!(
                saved
                    .notices
                    .iter()
                    .any(|notice| notice.contains("input evidence is incomplete"))
            );
            assert!(!h.directory().exists());
        }
    }
}

#[tokio::test]
async fn task_image_lineage_can_identify_an_owned_images_profile_output() {
    let h = Host::new("normal").await;
    h.c.submit(h.request(None).await).await.unwrap();
    let seed = h.settled().await;
    assert_eq!(seed.outputs.len(), 1);
    let image = &seed.outputs[0];
    assert_ne!(image["asset"]["sha256"], h.a.sha256);
    tokio::time::timeout(Duration::from_secs(5), async {
        while h.c.snapshot().await.unwrap()["tools"][0]["busy"] == true {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut next = h.request(None).await;
    next.id = "from-image".into();
    next.images.insert(
        "source".into(),
        vec![image["asset"]["id"].as_str().unwrap().to_owned()],
    );
    next.lineage = serde_json::from_value(
        json!({"source":[{"attempt":"binding-test","port":image["role"],"occurrence":0}]}),
    )
    .unwrap();
    h.c.submit(next).await.unwrap();
    let saved = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let saved = h.c.attempts.get("from-image").await.unwrap();
            if saved.preservation["contract"] != "pending" {
                return saved;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(saved.execution, "completed", "{:?}", saved.result);
    assert_eq!(
        saved.arguments["source"]["expect"]["file_sha256"],
        image["asset"]["sha256"]
    );
    h.close().await;
}
