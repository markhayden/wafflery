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
        Self::with_flags(&[]).await
    }
    async fn with_flags(flags: &[&str]) -> Self {
        Self::configured(flags, false).await
    }
    async fn native(flags: &[&str]) -> Self {
        Self::configured(flags, true).await
    }
    async fn configured(flags: &[&str], preserve_native: bool) -> Self {
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
            Coordinator::open_with_options(
                assets,
                vec![Descriptor {
                    id: "peer".into(),
                    executable: binary.clone(),
                    args: [
                        vec![
                            root.path().join("peer").display().to_string(),
                            "v04-operations".into(),
                        ],
                        flags.iter().map(|flag| (*flag).to_owned()).collect(),
                    ]
                    .concat(),
                    cwd: root.path().into(),
                    allow_dirs: vec![root.path().into()],
                    evidence_dirs: vec![root.path().into()],
                    config: None,
                }],
                wafflery::tools::coordinator::Options { preserve_native },
            )
            .await
            .unwrap(),
        );
        let snapshot = c.snapshot().await.unwrap();
        assert!(
            snapshot["tools"][0]["declaration"].is_object(),
            "{}",
            snapshot["tools"][0]["error"]
        );
        Self { root, c, image }
    }
    async fn request(&self, id: &str, action: &str, bindings: Value) -> Submission {
        serde_json::from_value(json!({"id":id,"tool":"peer","action":action,"revision":self.c.description("peer").await.unwrap_or_else(|error| panic!("description: {error}")).revision,"params":{},"images":{},"bindings":bindings})).unwrap()
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
            json!({"image":[{"kind":"artifact","reference":image_reference.clone()}]}),
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
            .any(|note| note.contains("image bytes"))
    );
    let marker = located.result.unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let request = h
        .request(
            "stamp",
            "stamp_marker",
            json!({"image":[image],"marker":[{"kind":"artifact","reference":marker.clone()}]}),
        )
        .await;
    h.c.submit(request).await.unwrap();
    let stamped = h.settled("stamp").await;
    let referenced = h.request("stamp-reference", "stamp_marker", json!({"image":[{"kind":"artifact","reference":image_reference.clone()}],"marker":[{"kind":"artifact","reference":marker.clone()}]})).await;
    h.c.submit(referenced).await.unwrap();
    let referenced = h.settled("stamp-reference").await;
    assert_eq!(referenced.execution, "completed");
    assert_eq!(referenced.preservation["contract"], "incomplete");
    assert!(
        referenced
            .notices
            .iter()
            .any(|note| note.contains("passed by reference"))
    );

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
        {"kind":"artifact","reference":marker.clone()},{"kind":"artifact","reference":wrong_version},
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

#[tokio::test]
async fn priced_operation_estimates_bind_ports_without_creating_attempts() {
    let h = Host::with_flags(&["--priced-operations"]).await;
    let mut request = h
        .request(
            "priced",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await;
    request.params = json!({"max_spend_usd":0.5});
    h.c.prepare(request.clone()).await.unwrap();
    let quote = h.c.estimate_prepared("priced").await.unwrap();
    assert!(h.c.attempts.get("priced").await.is_err());
    let mut edited = request.clone();
    edited.bindings.get_mut("image").unwrap()[0] = serde_json::from_value(json!({"kind":"image","asset":h.image.id,"frame":{"projection":"equirectangular","topology":"wrap"}})).unwrap();
    assert!(h.c.submit_prepared(edited).await.is_err());
    h.c.submit_prepared(request).await.unwrap();
    let saved = h.settled("priced").await;
    assert_eq!(saved.execution, "completed");
    assert!(quote.estimate.get("binding").is_none()); // Binding stays in the host, not the browser receipt.
    assert!(saved.observations["estimate"]["binding"].is_string());
    assert_eq!(
        saved.wire.as_ref().unwrap()["params"]["_meta"]["table-manners/estimate"],
        saved.observations["estimate"]["binding"]
    );
    h.close().await;
}

#[tokio::test]
async fn estimate_binding_refusals_preserve_indices_without_runs_or_spend() {
    let h = Host::with_flags(&["--priced-operations"]).await;
    let missing =
        fixtures::artifact_variant("sample-lab", "sample-lab:Token", 0.75)["artifact"].clone();
    let mut request = h
        .request(
            "refused-estimate",
            "collect_tokens",
            json!({"tokens":[{"kind":"artifact","reference":missing}]}),
        )
        .await;
    request.params = json!({"max_spend_usd":0.5});
    h.c.prepare(request.clone()).await.unwrap();
    let error = h.c.estimate_prepared(&request.id).await.err().unwrap();
    let wafflery::tools::Error::BindingRefusal(data) = error else {
        panic!("Expected structured binding refusal: {error}")
    };
    assert_eq!(data["bindings"][0]["port"], "tokens");
    assert_eq!(data["bindings"][0]["index"], 0);
    assert_eq!(data["bindings"][0]["problem"], "unresolved");
    assert!(data["bindings"][0]["remedy"].is_string());
    assert!(h.c.attempts.get(&request.id).await.is_err());
    assert!(h.c.prepared_estimate(&request.id).await.is_err());
    assert_eq!(h.c.session("peer").await.unwrap()["spent_usd"], 0);
    h.c.discard_preparation(&request.id).await.unwrap();
    h.c.prepare(request).await.unwrap(); // No attempt claimed this client ID.
    h.close().await;
}

#[tokio::test]
async fn port_dispatch_reports_progress_and_cancels_through_the_normal_owner() {
    let h = Host::with_flags(&["--hold-operation"]).await;
    let request = h
        .request(
            "cancel-port",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await;
    h.c.submit(request).await.unwrap();
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let snapshot = h.c.snapshot().await.unwrap();
            if snapshot["tools"][0]["progress"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("Waiting"))
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    h.c.cancel("cancel-port").await.unwrap();
    let saved = h.settled("cancel-port").await;
    assert_eq!(saved.result.as_ref().unwrap()["status"], "cancelled");
    h.close().await;
}

#[tokio::test]
async fn artifact_inspection_reads_only_a_saved_output_reference() {
    let h = Host::new().await;
    h.c.submit(
        h.request(
            "inspect",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    let saved = h.settled("inspect").await;
    let resource = h.c.inspect_artifact("inspect", "out", 0).await.unwrap();
    assert_eq!(
        resource["artifact"],
        saved.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"]
    );
    assert!(resource["record"].is_object());
    h.c.restart("peer", true).await.unwrap();
    assert_eq!(
        h.c.inspect_artifact("inspect", "out", 0).await.unwrap()["artifact"],
        resource["artifact"]
    );
    assert!(h.c.inspect_artifact("inspect", "out", 1).await.is_err());
    assert!(h.c.inspect_artifact("inspect", "forged", 0).await.is_err());
    std::fs::remove_dir_all(h.root.path().join("peer/artifacts")).unwrap();
    let error = h.c.inspect_artifact("inspect", "out", 0).await.unwrap_err();
    assert!(error.to_string().contains("current tool store"), "{error}");
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    h.close().await;
}

#[tokio::test]
async fn lineage_is_saved_as_host_metadata_and_cannot_forge_an_output_occurrence() {
    let h = Host::new().await;
    h.c.submit(
        h.request(
            "upstream",
            "locate_marker",
            json!({"image":[{"kind":"image","asset":h.image.id}]}),
        )
        .await,
    )
    .await
    .unwrap();
    let upstream = h.settled("upstream").await;
    let reference = upstream.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone();
    let mut next=h.request("downstream","stamp_marker",json!({"image":[{"kind":"image","asset":h.image.id}],"marker":[{"kind":"artifact","reference":reference}]})).await;
    next.lineage = serde_json::from_value(
        json!({"marker":[{"attempt":"upstream","port":"marker","occurrence":9}]}),
    )
    .unwrap();
    assert!(h.c.prepare(next.clone()).await.is_err());
    next.lineage = serde_json::from_value(
        json!({"marker":[{"attempt":"upstream","port":"marker","occurrence":0}]}),
    )
    .unwrap();
    for source in [
        json!({"attempt":"upstream","port":"wrong","occurrence":0}),
        json!({"attempt":"missing","port":"marker","occurrence":0}),
    ] {
        let mut invalid = next.clone();
        invalid.lineage = serde_json::from_value(json!({"marker":[source]})).unwrap();
        assert!(h.c.prepare(invalid).await.is_err());
    }
    let mut invalid = next.clone();
    invalid.lineage = serde_json::from_value(json!({"marker":[]})).unwrap();
    assert!(h.c.prepare(invalid).await.is_err());
    h.c.submit(next.clone()).await.unwrap();
    let saved = h.settled("downstream").await;
    assert_eq!(saved.request.lineage, next.lineage);
    let selection = wafflery::library::cleanup::Selection {
        attempts: vec!["upstream".into()],
        ..Default::default()
    };
    let preview = wafflery::library::cleanup::Cleanup(h.c.assets.clone())
        .preview(selection.clone())
        .await
        .unwrap();
    h.c.cleanup(selection, preview.token).await.unwrap();
    assert_eq!(h.c.submit(next.clone()).await.unwrap().id, "downstream");

    assert!(saved.arguments.get("lineage").is_none());
    assert_eq!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    h.close().await;
}

#[tokio::test]
async fn native_references_cannot_cross_tool_boundaries_before_dispatch() {
    let h = Host::new().await;
    let reference = fixtures::artifact("foreign-lab", "foreign-lab:Token")["artifact"].clone();
    let request = h
        .request(
            "foreign",
            "collect_tokens",
            json!({"tokens":[{"kind":"artifact","reference":reference}]}),
        )
        .await;
    let error = h.c.prepare(request).await.err().unwrap();
    assert!(error.to_string().contains("another tool"));
    assert!(
        h.c.attempts
            .history(None, 100, None, None)
            .await
            .unwrap()
            .is_empty()
    );
    h.close().await;
}

#[path = "artifact_controls/cases.rs"]
mod artifact_control_cases;

#[path = "native_preservation/cases.rs"]
mod native_preservation_cases;
