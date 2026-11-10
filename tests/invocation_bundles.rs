#[allow(dead_code)]
#[path = "../testkit/v04_fixtures.rs"]
mod fixtures;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, io::Read};
use wafflery::{
    library::{
        assets::AssetStore,
        attempts::{Attempt, AttemptStore},
        blob_links::{BlobLink, Owner},
        blobs::BlobStore,
    },
    tools::bundles::{self, Options},
};
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
async fn fixture() -> (tempfile::TempDir, AssetStore, Attempt, Value) {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let path = root.path().join("source.png");
    std::fs::write(&path, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let image = assets.import_path(path).await.unwrap();
    let marker = fixtures::artifact("sample", "location");
    let expect = marker["record"]["image"].clone();
    let action = json!({"name":"rasterize","kind":"operation","ports":{"in":[{"name":"pictures","type":"image","many":true,"required":true},{"name":"markers","type":"location","many":true,"required":true}]}});
    let declaration = json!({"document":{"contract":"0.6.0","tool":{"name":"sample","version":"0.1.0"},"profiles":["operations"],"artifact_types":[{"type":"location","portable":"json"}],"actions":[action]}});
    let request=serde_json::from_value(json!({"id":"selected","tool":"peer","action":"rasterize","revision":"r","params":{},"images":{},"bindings":{"pictures":[{"kind":"image","asset":image.id},{"kind":"image","asset":image.id}],"markers":[{"kind":"artifact","reference":marker["artifact"]},{"kind":"artifact","reference":marker["artifact"]}]}})).unwrap();
    let mut attempt=AttemptStore::new(assets.database.clone()).create(request,json!({}),declaration,json!({"pictures":[{"asset":image,"image":expect,"argument_index":0},{"asset":image,"image":expect,"argument_index":1}]})).await.unwrap();
    let mut result = fixtures::result("sample", "completed");
    result["action"] = json!("rasterize");
    result["invocation"]["action"] = json!("rasterize");
    result["invocation"]["arguments"] = json!({"pictures":[{"expect":expect},{"expect":expect}],"markers":[marker["artifact"],marker["artifact"]],"amount":0.75});
    result["artifacts"]["in"] = json!([{"port":"markers","artifact":marker["artifact"],"preview":marker["preview"]},{"port":"markers","artifact":marker["artifact"],"preview":null}]);
    result["images"]["in"] = json!([{"slot":"pictures","role":"source","image":expect},{"slot":"pictures","role":"source","image":expect}]);
    attempt.result = Some(result);
    attempt.execution = "completed".into();
    attempt.preservation["contract"] = json!("complete");
    let bytes = serde_json::to_vec(&marker["record"]).unwrap();
    let digest = hash(&bytes);
    let blobs = BlobStore(assets.clone());
    blobs.publish(bytes, digest.clone()).await.unwrap();
    for occurrence in 0..2 {
        blobs
            .attach(
                Owner::Attempt(attempt.id.clone()),
                BlobLink {
                    direction: "in".into(),
                    occurrence,
                    sha256: digest.clone(),
                    media_type: "application/json".into(),
                    artifact: marker["artifact"].clone(),
                },
            )
            .await
            .unwrap();
    }
    (root, assets, attempt, marker)
}
fn contents(path: &std::path::Path) -> (Vec<String>, BTreeMap<String, Vec<u8>>) {
    let mut archive = tar::Archive::new(std::fs::File::open(path).unwrap());
    let mut names = vec![];
    let mut files = BTreeMap::new();
    for item in archive.entries().unwrap() {
        let mut item = item.unwrap();
        let header = item.header();
        assert!(header.as_ustar().is_some());
        assert_eq!(header.mode().unwrap(), 0o644);
        assert_eq!(header.uid().unwrap(), 0);
        assert_eq!(header.gid().unwrap(), 0);
        assert_eq!(header.mtime().unwrap(), 0);
        assert_eq!(header.username().unwrap(), Some(""));
        assert_eq!(header.groupname().unwrap(), Some(""));
        let name = item.path().unwrap().to_str().unwrap().to_owned();
        let mut bytes = vec![];
        item.read_to_end(&mut bytes).unwrap();
        names.push(name.clone());
        files.insert(name, bytes);
    }
    (names, files)
}
#[tokio::test]
async fn bundle_is_schema_valid_canonical_and_keeps_every_occurrence_without_duplicate_members() {
    let (_root, assets, attempt, marker) = fixture().await;
    let a = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let b = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    assert_eq!(a.sha256, b.sha256);
    assert_eq!(
        a.sha256,
        "57accb5693ba1fe88a5cd1ee27c051bc4484715ad93b73eaf01942a1e6a72b39"
    );
    assert_eq!(
        std::fs::read(a.file.path()).unwrap(),
        std::fs::read(b.file.path()).unwrap()
    );
    let (names, files) = contents(a.file.path());
    assert_eq!(names.len(), 3);
    assert_eq!(names[0], "invocation.json");
    assert!(names[1..].is_sorted());
    let manifest: Value = serde_json::from_slice(&files["invocation.json"]).unwrap();
    assert!(
        wafflery::tools::validator(&json!({"$ref":"urn:table-manners:core:invocation-bundle"}))
            .unwrap()
            .is_valid(&manifest)
    );
    let step = &manifest["steps"][0];
    assert_eq!(step["result"], *attempt.result.as_ref().unwrap());
    assert_eq!(
        step["invocation"],
        attempt.result.as_ref().unwrap()["invocation"]
    );
    assert_eq!(step["inputs"].as_array().unwrap().len(), 2);
    assert_eq!(step["artifacts"].as_array().unwrap().len(), 2);
    assert_eq!(step["artifacts"][0]["artifact"], marker["artifact"]);
    assert_eq!(step["artifacts"][0], step["artifacts"][1]);
    assert_eq!(
        files[step["inputs"][0]["member"].as_str().unwrap()],
        include_bytes!("../ui/tests/fixtures/sample.png")
    );
    assert_eq!(
        files[step["artifacts"][0]["member"].as_str().unwrap()],
        serde_json::to_vec(&marker["record"]).unwrap()
    );
    assert!(!a.workspace_dependent);
    assert_eq!(a.native_links.len(), 2);
    assert_eq!(a.input_assets.len(), 1);
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn location_by_value_needs_no_native_export_but_references_do_and_omissions_are_explicit() {
    let (_root, assets, mut attempt, marker) = fixture().await;
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]["markers"] =
        json!([marker["record"]]);
    attempt.result.as_mut().unwrap()["artifacts"]["in"] =
        json!([{"port":"markers","artifact":marker["artifact"],"preview":null}]);
    let by_value = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    assert!(
        by_value.manifest["steps"][0]["artifacts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!by_value.workspace_dependent);
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]["markers"] =
        json!([marker["artifact"], marker["artifact"]]);
    attempt.result.as_mut().unwrap()["artifacts"]["in"] = json!([{"port":"markers","artifact":marker["artifact"],"preview":null},{"port":"markers","artifact":marker["artifact"],"preview":null}]);
    let omitted = bundles::build(
        &assets,
        &attempt,
        &Options {
            omit: BTreeMap::from([("pictures".into(), "Left out by the person.".into())]),
        },
    )
    .await
    .unwrap();
    assert!(omitted.workspace_dependent);
    assert_eq!(
        omitted.manifest["steps"][0]["reference_only"],
        json!([{"port":"pictures","why":"Left out by the person."}])
    );
    assert!(
        omitted.manifest["steps"][0]["inputs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn missing_native_bytes_and_nonportable_types_are_labelled_never_replaced_by_previews() {
    let (_root, assets, mut attempt, _marker) = fixture().await;
    let links = BlobStore(assets.clone())
        .links(Owner::Attempt(attempt.id.clone()))
        .await
        .unwrap();
    let digest = &links[0].sha256;
    std::fs::remove_file(
        assets
            .root()
            .join("blobs/sha256")
            .join(&digest[..2])
            .join(digest),
    )
    .unwrap();
    let bundle = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    assert!(bundle.workspace_dependent);
    assert_eq!(
        bundle.manifest["steps"][0]["unportable"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        bundle.manifest["steps"][0]["artifacts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(contents(bundle.file.path()).0.len(), 2);
    attempt.declaration["document"]["artifact_types"][0]["portable"] = json!("none");
    let bundle = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    assert!(
        bundle.manifest["steps"][0]["unportable"][0]["reason"]
            .as_str()
            .unwrap()
            .contains("portable")
    );
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn missing_images_conflicting_expectations_and_unresolved_invocations_fail_without_an_archive()
 {
    let (_root, assets, mut attempt, _marker) = fixture().await;
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]["pictures"][0]["expect"]["width"] =
        json!(999);
    assert!(
        bundles::build(&assets, &attempt, &Options::default())
            .await
            .is_err()
    );
    attempt.result.as_mut().unwrap()["invocation"] = Value::Null;
    assert!(
        bundles::build(&assets, &attempt, &Options::default())
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_dir(assets.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn mixed_image_lists_keep_argument_indices_and_do_not_treat_artifact_previews_as_sources() {
    let (_root, assets, mut attempt, _) = fixture().await;
    let owned = attempt.inputs["pictures"][0].clone();
    let expect = attempt.result.as_ref().unwrap()["invocation"]["arguments"]["pictures"][0].clone();
    let image = fixtures::artifact("sample", "image");
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]["pictures"] =
        json!([image["artifact"], expect, expect]);
    let mut first = owned.clone();
    first["argument_index"] = json!(1);
    let mut second = owned;
    second["argument_index"] = json!(2);
    attempt.inputs["pictures"] = json!([first, second]);
    let entries = attempt.result.as_ref().unwrap()["artifacts"]["in"]
        .as_array()
        .unwrap()
        .clone();
    let mut mixed =
        vec![json!({"port":"pictures","artifact":image["artifact"],"preview":image["preview"]})];
    mixed.extend(entries);
    attempt.result.as_mut().unwrap()["artifacts"]["in"] = json!(mixed);
    // Native links are keyed by global result occurrence, not an index in the port's list.
    let old = BlobStore(assets.clone())
        .links(Owner::Attempt(attempt.id.clone()))
        .await
        .unwrap();
    new_owner(&assets, &mut attempt, "shifted").await;
    for link in old {
        BlobStore(assets.clone())
            .attach(
                Owner::Attempt("shifted".into()),
                BlobLink {
                    occurrence: link.occurrence + 1,
                    ..link
                },
            )
            .await
            .unwrap();
    }

    let bundle = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    assert_eq!(
        bundle.manifest["steps"][0]["inputs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        bundle.manifest["steps"][0]["artifacts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        bundle.manifest["steps"][0]["unportable"][0]["artifact"],
        image["artifact"]
    );
    assert!(bundle.workspace_dependent);
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn task_slots_keep_duplicate_image_order() {
    let (root, assets, mut attempt, _) = fixture().await;
    let path = root.path().join("other.png");
    std::fs::write(&path, include_bytes!("../ui/tests/fixtures/panorama.png")).unwrap();
    let other = assets.import_path(path).await.unwrap();
    let other_image = json!({"file_sha256":other.sha256,"width":other.width,"height":other.height,"projection":"equirectangular","topology":"wrap"});
    let first = attempt.inputs["pictures"][0].clone();
    let expect = attempt.result.as_ref().unwrap()["invocation"]["arguments"]["pictures"][0].clone();
    let mut last = first.clone();
    last["argument_index"] = json!(2);
    attempt.inputs =
        json!({"references":[first,{"asset":other,"image":other_image,"argument_index":1},last]});
    attempt.declaration["document"]["actions"][0] = json!({"name":"rasterize","kind":"task","images":[{"slot":"references","required":true,"many":true}]});
    attempt.result.as_mut().unwrap()["invocation"]["arguments"] =
        json!({"references":[expect,{"expect":other_image},expect]});
    attempt.result.as_mut().unwrap()["artifacts"]["in"] = json!([]);
    let bundle = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let inputs = bundle.manifest["steps"][0]["inputs"].as_array().unwrap();
    assert_eq!(inputs.len(), 3);
    assert_eq!(inputs[0], inputs[2]);
    assert_ne!(inputs[0]["member"], inputs[1]["member"]);
    assert_eq!(inputs[1]["slot"], "references");
    assert_eq!(contents(bundle.file.path()).0.len(), 3);
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn exact_native_bundle_bytes_use_fixed_extensions_and_not_an_export_filename() {
    let (_root, assets, mut attempt, _) = fixture().await;
    let native = fixtures::artifact("sample", "sample:Token")["artifact"].clone();
    let bytes = b"opaque native payload\0\xffnot a PNG or host-parsed tar".to_vec();
    let digest = hash(&bytes);
    let blobs = BlobStore(assets.clone());
    blobs.publish(bytes.clone(), digest.clone()).await.unwrap();
    attempt.declaration["document"]["artifact_types"] =
        json!([{"type":"sample:Token","portable":"bundle"}]);
    attempt.declaration["document"]["actions"][0]["ports"]["in"] =
        json!([{"name":"token","type":"sample:Token","required":true}]);
    attempt.result.as_mut().unwrap()["invocation"]["arguments"] = json!({"token":native});
    attempt.result.as_mut().unwrap()["artifacts"]["in"] =
        json!([{"port":"token","artifact":native,"preview":null}]);
    new_owner(&assets, &mut attempt, "native-only").await;
    blobs
        .attach(
            Owner::Attempt(attempt.id.clone()),
            BlobLink {
                direction: "in".into(),
                occurrence: 0,
                sha256: digest.clone(),
                media_type: "application/x-tar".into(),
                artifact: native.clone(),
            },
        )
        .await
        .unwrap();
    let bundle = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let item = &bundle.manifest["steps"][0]["artifacts"][0];
    assert_eq!(
        item["member"],
        format!(
            "artifacts/{}.tar",
            native["identity"].as_str().unwrap().replace(':', "-")
        )
    );
    assert_eq!(item["sha256"], digest);
    assert_eq!(
        contents(bundle.file.path()).1[item["member"].as_str().unwrap()],
        bytes
    );
    assert!(!bundle.workspace_dependent);
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_omissions_and_missing_original_images_leave_no_temporary_archive() {
    let (_root, assets, mut attempt, _) = fixture().await;
    for (name, why) in [("unknown", "Chosen omission"), ("pictures", "")] {
        assert!(
            bundles::build(
                &assets,
                &attempt,
                &Options {
                    omit: BTreeMap::from([(name.into(), why.into())])
                }
            )
            .await
            .is_err()
        );
    }
    let sha = attempt.inputs["pictures"][0]["asset"]["sha256"]
        .as_str()
        .unwrap();
    std::fs::remove_file(
        assets
            .root()
            .join("assets/sha256")
            .join(&sha[..2])
            .join(sha),
    )
    .unwrap();
    assert!(
        bundles::build(&assets, &attempt, &Options::default())
            .await
            .is_err()
    );
    attempt.result.as_mut().unwrap()["tool"]["version"] = json!("newer");
    assert!(
        bundles::build(&assets, &attempt, &Options::default())
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_dir(assets.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    assets.database.shutdown().await.unwrap();
}

async fn new_owner(assets: &AssetStore, attempt: &mut Attempt, id: &str) {
    attempt.request.id = id.into();
    AttemptStore::new(assets.database.clone())
        .create(
            attempt.request.clone(),
            attempt.arguments.clone(),
            attempt.declaration.clone(),
            attempt.inputs.clone(),
        )
        .await
        .unwrap();
    attempt.id = id.into();
}

#[tokio::test]
async fn in_flight_preservation_and_nonexistent_or_literal_only_omissions_are_not_frozen_as_partial_bundles()
 {
    let (_root, assets, mut attempt, marker) = fixture().await;
    attempt.preservation["native"] = json!("preserving");
    assert!(
        bundles::build(&assets, &attempt, &Options::default())
            .await
            .is_err()
    );
    attempt.preservation["native"] = json!("preserved");
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]
        .as_object_mut()
        .unwrap()
        .remove("markers");
    attempt.declaration["document"]["actions"][0]["ports"]["in"][1]["required"] = json!(false);
    let omit = Options {
        omit: BTreeMap::from([("markers".into(), "Chosen omission".into())]),
    };
    assert!(bundles::build(&assets, &attempt, &omit).await.is_err());
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]["markers"] =
        json!([marker["record"]]);
    assert!(bundles::build(&assets, &attempt, &omit).await.is_err());
    assets.database.shutdown().await.unwrap();
}
#[path = "invocation_bundles/reader.rs"]
mod reader;
#[path = "invocation_bundles/store.rs"]
mod store;

#[path = "invocation_bundles/review_api.rs"]
mod review_api;
#[path = "invocation_bundles/step.rs"]
mod step;
