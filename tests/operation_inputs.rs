#[path = "../testkit/v04_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;
use serde_json::{Value, json};
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
    image: Asset,
}
impl Host {
    async fn new() -> Self {
        static BIN: OnceLock<PathBuf> = OnceLock::new();
        let binary = BIN.get_or_init(|| {
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
        let assets = AssetStore::open(&root.path().join("Library")).unwrap();
        let source = root.path().join("input.png");
        std::fs::write(&source, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
        let image = assets.import_path(source).await.unwrap();
        let c = Arc::new(
            Coordinator::open(
                assets,
                vec![Descriptor {
                    id: "peer".into(),
                    executable: binary.clone(),
                    args: vec![
                        root.path().join("peer").display().to_string(),
                        "v04-operations".into(),
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
        Self { root, c, image }
    }
    async fn request(&self, id: &str, action: &str, bindings: Value) -> Submission {
        serde_json::from_value(json!({"id":id,"tool":"peer","action":action,"revision":self.c.description("peer").await.unwrap().revision,"params":{},"images":{},"bindings":bindings})).unwrap()
    }
    async fn settled(&self, id: &str) -> Attempt {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let a = self.c.attempts.get(id).await.unwrap();
                if a.preservation["contract"] != "pending"
                    && (a.execution == "interrupted"
                        || self.c.snapshot().await.unwrap()["tools"][0]["busy"] == false)
                {
                    return a;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap()
    }
    async fn close(&self) {
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        self.c.begin_shutdown(true, tx).await.unwrap();
        tokio::time::timeout(Duration::from_secs(8), rx.wait_for(|v| *v))
            .await
            .unwrap()
            .unwrap();
    }
}
#[tokio::test]
async fn an_image_operation_uses_owned_copies_and_the_normal_saved_run() {
    let h = Host::new().await;
    let request = h
        .request(
            "locate",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await;
    h.c.submit(request).await.unwrap();
    let saved = h.settled("locate").await;
    assert_eq!(saved.execution, "completed");
    assert_eq!(saved.preservation["contract"], "complete");
    assert_eq!(
        saved.arguments["image"]["expect"]["file_sha256"],
        h.image.sha256
    );
    assert_eq!(saved.inputs["image"][0]["asset"]["id"], h.image.id);
    assert_eq!(
        saved.result.as_ref().unwrap()["artifacts"]["out"][0]["port"],
        "marker"
    );
    h.close().await;
    assert!(!h.c.assets.root().join("execution/locate").exists());
    assert!(h.root.path().join("input.png").exists());
}

#[tokio::test]
async fn saved_native_references_chain_without_rerunning_upstream_and_refusals_stay_indexed() {
    let h = Host::new().await;
    let image = json!({"kind":"image","asset":h.image.id});
    let request = h
        .request("locate", "locate_marker", json!({"image":[image]}))
        .await;
    h.c.submit(request).await.unwrap();
    let located = h.settled("locate").await;
    let image_reference =
        located.result.as_ref().unwrap()["artifacts"]["in"][0]["artifact"].clone();
    let image_request = h
        .request(
            "locate-reference",
            "locate_marker",
            json!({"image":[{"kind":"artifact","reference":image_reference}]}),
        )
        .await;
    h.c.submit(image_request).await.unwrap();
    let image_result = h.settled("locate-reference").await;
    assert_eq!(image_result.execution, "completed");
    // References are usable now; their native/image preservation belongs to M5.08–12.
    assert_eq!(image_result.preservation["contract"], "incomplete");
    assert!(
        image_result
            .notices
            .iter()
            .any(|note| note.contains("image bytes that were not preserved"))
    );
    let marker = located.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let request = h
        .request(
            "stamp",
            "stamp_marker",
            json!({"image":[image],"marker":[{"kind":"artifact","reference":marker}]}),
        )
        .await;
    h.c.submit(request).await.unwrap();
    let stamped = h.settled("stamp").await;
    assert_eq!(stamped.execution, "completed");
    let token = stamped.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let binding = json!({"kind":"artifact","reference":token});
    let request = h
        .request(
            "collect",
            "collect_tokens",
            json!({"tokens":[binding,binding]}),
        )
        .await;
    let expected = request.bindings.clone();
    h.c.submit(request).await.unwrap();
    let collected = h.settled("collect").await;
    assert_eq!(collected.execution, "completed");
    assert_eq!(collected.request.bindings, expected);
    assert_eq!(collected.arguments["tokens"], json!([token, token]));
    assert!(collected.arguments.get("guides").is_none());
    let mut wrong_version = token.clone();
    wrong_version["schema_version"] = json!(2);
    let mut wrong_frame = token.clone();
    wrong_frame["frame"] = json!("wrong");
    let missing =
        fixtures::artifact_variant("sample-lab", "sample-lab:Token", 0.75)["artifact"].clone();
    let request = h
        .request(
            "refused",
            "collect_tokens",
            json!({"tokens":[
        {"kind":"artifact","reference":marker},{"kind":"artifact","reference":wrong_version},
        {"kind":"artifact","reference":wrong_frame},{"kind":"artifact","reference":missing}]}),
        )
        .await;
    h.c.submit(request).await.unwrap();
    let refused = h.settled("refused").await;
    assert_eq!(refused.execution, "rejected");
    assert!(refused.result.is_none());
    assert_eq!(refused.observations["spent_usd"], 0);
    let problems = refused.observations["refusal"]["bindings"]
        .as_array()
        .unwrap();
    assert_eq!(
        problems
            .iter()
            .map(|v| (&v["index"], v["problem"].as_str().unwrap()))
            .collect::<Vec<_>>(),
        vec![
            (&json!(0), "wrong_type"),
            (&json!(1), "wrong_version"),
            (&json!(2), "wrong_frame"),
            (&json!(3), "unresolved")
        ]
    );
    assert!(problems[3]["remedy"].is_string());
    h.close().await;
}

#[tokio::test]
async fn counts_unknown_ports_raw_paths_and_parameter_collisions_fail_before_copying() {
    let h = Host::new().await;
    let image = json!({"kind":"image","asset":h.image.id});
    for (action, bindings) in [
        ("locate_marker", json!({})),
        ("locate_marker", json!({"image":[image,image]})),
        ("locate_marker", json!({"unknown":[image]})),
        ("collect_tokens", json!({"tokens":[]})),
        ("collect_tokens", json!({"tokens":[image]})),
    ] {
        let request = h.request("bad", action, bindings).await;
        assert!(h.c.prepare(request).await.is_err());
        assert!(!h.c.assets.root().join("execution/bad").exists());
        assert!(h.c.attempts.get("bad").await.is_err());
    }
    let mut request = h
        .request("bad", "locate_marker", json!({"image":[image]}))
        .await;
    request.params = json!({"image":{"path":"/not-an-owned-image"}});
    assert!(h.c.prepare(request).await.is_err());
    let malformed = json!({"id":"bad","tool":"peer","action":"locate_marker","revision":"r","params":{},"images":{},
        "bindings":{"image":[{"kind":"image","path":"/not-an-owned-image"}]}});
    assert!(serde_json::from_value::<Submission>(malformed).is_err());
    h.close().await;
}

#[tokio::test]
async fn shared_location_values_use_the_declared_port_without_native_decoding() {
    let h = Host::new().await;
    let location = fixtures::artifact("sample-lab", "location")["record"].clone();
    let request=h.request("location-value","stamp_marker",json!({
        "image":[{"kind":"image","asset":h.image.id}],"marker":[{"kind":"location","value":location}]
    })).await;
    h.c.submit(request).await.unwrap();
    let saved = h.settled("location-value").await;
    assert_eq!(saved.execution, "completed");
    assert_eq!(saved.arguments["marker"], location);
    h.close().await;
}
