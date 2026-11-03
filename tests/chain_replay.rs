#[allow(dead_code)]
#[path = "../testkit/v04_fixtures.rs"]
mod fixtures;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use wafflery::{
    library::{
        assets::AssetStore,
        attempts::{Attempt, BindingSource, InputBinding},
    },
    tools::chain_replay,
};
fn attempt(result: Value) -> Attempt {
    serde_json::from_value(json!({"id":"fresh-producer","sequence":1,"created_ms":0,"request":{"id":"fresh-producer","tool":"peer-alias","action":"soften_token","revision":"current","params":{},"images":{}},"arguments":{},"declaration":{},"inputs":{},"wire":null,"execution":"completed","result":result,"preservation":{"contract":"complete","native":"preserved"},"outputs":[],"journal":null,"notices":[]})).unwrap()
}
fn step(result: Value) -> Value {
    json!({"tool":result["tool"],"invocation":result["invocation"],"result":result,"inputs":[],"artifacts":[],"reference_only":[],"unportable":[]})
}
fn native_chain() -> (Value, Attempt, Value) {
    let old = fixtures::result("sample", "completed");
    let original = old["artifacts"]["out"][0]["artifact"].clone();
    let mut consumer = old.clone();
    consumer["invocation"]["arguments"]["token"] = json!([original, original]);
    consumer["artifacts"]["in"] =
        json!([{"port":"token","artifact":original},{"port":"token","artifact":original}]);
    consumer["artifacts"]["out"] = json!([]);
    consumer["images"]["out"] = json!([]);
    let new = fixtures::artifact_variant("sample", "sample:Token", 0.75);
    let mut fresh = old.clone();
    fresh["invocation"]["arguments"]["amount"] = json!(0.75);
    fresh["artifacts"]["out"] = json!([{"port":"other","artifact":fixtures::artifact("sample","sample:Token")["artifact"],"preview":null},{"port":"token","artifact":new["artifact"],"preview":new["preview"]}]);
    fresh["images"]["out"] = json!([new["preview"]]);
    (
        json!({"steps":[step(old),step(consumer)]}),
        attempt(fresh),
        new["artifact"].clone(),
    )
}
#[tokio::test]
async fn native_consumers_bind_the_fresh_port_item_and_keep_duplicate_occurrences_with_actual_lineage()
 {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let (manifest, fresh, reference) = native_chain();
    let completed = BTreeMap::from([(0, fresh)]);
    let bindings = chain_replay::fresh_inputs(&assets, &manifest, 1, &completed)
        .await
        .unwrap();
    assert_eq!(bindings.len(), 2);
    for (index, input) in bindings.iter().enumerate() {
        assert_eq!(input.index, index);
        assert_eq!(input.name, "token");
        assert_eq!(
            input.binding,
            InputBinding::Artifact {
                reference: reference.clone()
            }
        );
        assert!(input.changed);
        assert_eq!(
            input.source,
            BindingSource::Attempt {
                attempt: "fresh-producer".into(),
                port: "token".into(),
                occurrence: 1
            }
        );
    }
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn missing_failed_or_unconfirmed_producers_stop_before_any_consumer_can_be_prepared() {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let (manifest, good, _) = native_chain();
    assert!(
        chain_replay::fresh_inputs(&assets, &manifest, 1, &BTreeMap::new())
            .await
            .is_err()
    );
    for field in ["failed", "missing", "unconfirmed"] {
        let mut fresh = good.clone();
        match field {
            "failed" => {
                fresh.execution = "failed".into();
                fresh.result.as_mut().unwrap()["status"] = json!("failed");
            }
            "missing" => fresh.result.as_mut().unwrap()["artifacts"]["out"] = json!([]),
            _ => fresh.preservation["contract"] = json!("incomplete"),
        };
        assert!(
            chain_replay::fresh_inputs(&assets, &manifest, 1, &BTreeMap::from([(0, fresh)]))
                .await
                .is_err()
        );
    }
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn fresh_images_use_owned_verified_bytes_and_do_not_substitute_native_previews() {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let path = root.path().join("source.png");
    std::fs::write(&path, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    let asset = assets.import_path(path).await.unwrap();
    let data = fixtures::artifact("sample", "image");
    let mut old = data.clone();
    old["artifact"]["identity"] = json!(format!("sha256:{}", "a".repeat(64)));
    old["artifact"]["resource"] = json!(format!("sample://artifact/sha256:{}", "a".repeat(64)));
    old["artifact"]["on"]["file_sha256"] = json!("a".repeat(64));
    old["preview"]["image"]["file_sha256"] = json!("a".repeat(64));
    old["preview"]["resource"] = json!(format!(
        "{}/preview",
        old["artifact"]["resource"].as_str().unwrap()
    ));
    let mut old_result = fixtures::result("sample", "completed");
    old_result["artifacts"]["out"] =
        json!([{"port":"picture","artifact":old["artifact"],"preview":old["preview"]}]);
    old_result["images"]["out"] = json!([old["preview"]]);
    let mut consumer = old_result.clone();
    consumer["tool"]["name"] = json!("reviewer");
    consumer["invocation"]["arguments"] = json!({"source":{"expect":old["preview"]["image"]}});
    consumer["artifacts"]["in"] = json!([]);
    consumer["images"]["in"] =
        json!([{"slot":"source","role":"source","image":old["preview"]["image"]}]);
    consumer["artifacts"]["out"] = json!([]);
    consumer["images"]["out"] = json!([]);
    let manifest = json!({"steps":[step(old_result),step(consumer)]});
    let mut fresh = fixtures::result("sample", "completed");
    fresh["artifacts"]["out"] =
        json!([{"port":"picture","artifact":data["artifact"],"preview":data["preview"]}]);
    fresh["images"]["out"] = json!([data["preview"]]);
    let mut saved = attempt(fresh);
    assert!(
        chain_replay::fresh_inputs(&assets, &manifest, 1, &BTreeMap::from([(0, saved.clone())]))
            .await
            .unwrap_err()
            .to_string()
            .contains("owned image")
    );
    saved.outputs = vec![json!({"asset":asset,"image":data["preview"]["image"],"role":"preview"})];
    let bound =
        chain_replay::fresh_inputs(&assets, &manifest, 1, &BTreeMap::from([(0, saved.clone())]))
            .await
            .unwrap();
    assert!(matches!(&bound[0].binding,InputBinding::Image{asset:id,..} if id==&asset.id));
    assert!(bound[0].changed);
    // The same image with a different spelling of integral dimensions remains bindable.
    saved.result.as_mut().unwrap()["images"]["out"][0]["image"]["width"] = json!(160.0);
    saved.result.as_mut().unwrap()["artifacts"]["out"][0]["preview"]["image"]["width"] =
        json!(160.0);
    saved.result.as_mut().unwrap()["artifacts"]["out"][0]["artifact"]["on"]["width"] = json!(160.0);
    assert!(
        chain_replay::fresh_inputs(&assets, &manifest, 1, &BTreeMap::from([(0, saved)]))
            .await
            .is_ok()
    );
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn many_outputs_bind_the_new_port_ordinal_instead_of_searching_for_the_old_identity() {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let (mut manifest, mut fresh, _) = native_chain();
    let first = fixtures::artifact_variant("sample", "sample:Token", 0.25);
    let second = fixtures::artifact_variant("sample", "sample:Token", 0.75);
    manifest["steps"][0]["result"]["artifacts"]["out"] = json!([{"port":"token","artifact":first["artifact"],"preview":first["preview"]},{"port":"token","artifact":second["artifact"],"preview":second["preview"]}]);
    manifest["steps"][1]["invocation"]["arguments"]["token"] =
        json!([second["artifact"], second["artifact"]]);
    manifest["steps"][1]["result"]["artifacts"]["in"] = json!([{"port":"token","artifact":second["artifact"]},{"port":"token","artifact":second["artifact"]}]);
    fresh.result.as_mut().unwrap()["artifacts"]["out"] = json!([{"port":"token","artifact":second["artifact"],"preview":second["preview"]},{"port":"token","artifact":first["artifact"],"preview":first["preview"]}]);
    let inputs = chain_replay::fresh_inputs(&assets, &manifest, 1, &BTreeMap::from([(0, fresh)]))
        .await
        .unwrap();
    assert!(inputs.iter().all(|input|matches!(&input.binding,InputBinding::Artifact{reference} if *reference==first["artifact"])&&input.changed));
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn fresh_output_normalization_has_one_shared_budget_across_producers() {
    let root = tempfile::tempdir().unwrap();
    let assets = AssetStore::open(&root.path().join("Library")).unwrap();
    let (mut manifest, first, _) = native_chain();
    let mut old_second = manifest["steps"][0].clone();
    let alternate = fixtures::artifact_variant("sample", "sample:Token", 0.75);
    old_second["result"]["artifacts"]["out"][0]["artifact"] = alternate["artifact"].clone();
    let mut consumer = manifest["steps"][1].clone();
    consumer["invocation"]["arguments"]["token"][1] = alternate["artifact"].clone();
    consumer["result"]["artifacts"]["in"][1]["artifact"] = alternate["artifact"].clone();
    manifest["steps"] = json!([manifest["steps"][0], old_second, consumer]);
    let mut a = first.clone();
    let mut b = first;
    for saved in [&mut a, &mut b] {
        let result = saved.result.as_mut().unwrap();
        let primary = result["artifacts"]["out"][1].clone();
        let extra = json!({"port":"extra","artifact":fixtures::artifact("sample","sample:Token")["artifact"],"preview":null});
        let mut outputs = vec![primary];
        outputs.extend(vec![extra; 999]);
        result["artifacts"]["out"] = json!(outputs);
        let image = result["images"]["out"][0].clone();
        result["images"]["out"] = json!(vec![image; 500]);
    }
    let error =
        chain_replay::fresh_inputs(&assets, &manifest, 2, &BTreeMap::from([(0, a), (1, b)]))
            .await
            .unwrap_err()
            .to_string();
    assert!(error.contains("connection search"));
    assets.database.shutdown().await.unwrap();
}
