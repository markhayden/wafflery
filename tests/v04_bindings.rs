#[path = "../testkit/v04_bindings.rs"]
mod bindings;
// This suite consumes part of the shared fixture catalogue; schema tests cover the rest.
#[allow(dead_code)]
#[path = "../testkit/v04_fixtures.rs"]
mod v04_fixtures;
#[path = "../testkit/v04_store.rs"]
mod v04_store;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

fn image() -> Value {
    json!({"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}})
}
fn rpc(error: bindings::Failure) -> Value {
    match error {
        bindings::Failure::Rpc(value) => value,
        other => panic!("expected RPC refusal, got {other:?}"),
    }
}

#[test]
fn defaults_and_delivery_controls_resolve_without_changing_the_saved_reference() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let token = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    store.remember(&token).unwrap();
    let prepared = bindings::prepare(
        &store,
        "sample-lab",
        "soften_token",
        json!({"token":token["artifact"],"return_images":"inline","image_blocks":true}),
    )
    .unwrap();
    assert_eq!(prepared.action["name"], "soften_token");
    assert_eq!(
        prepared.arguments,
        json!({"token":token["artifact"],"amount":0.25})
    );
    assert_eq!(prepared.inputs.len(), 1);
    assert_eq!(prepared.inputs[0].port, "token");
    assert_eq!(prepared.inputs[0].document, token);
    assert_eq!(prepared.return_images, "inline");
    assert!(prepared.image_blocks);
}

#[test]
fn list_bindings_report_every_problem_in_port_and_item_order() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "test-prism", vec![]).unwrap();
    let token = v04_fixtures::artifact("test-prism", "test-prism:Token");
    let marker = v04_fixtures::artifact("test-prism", "location");
    store.remember(&token).unwrap();
    store.remember(&marker).unwrap();
    let mut wrong_version = token["artifact"].clone();
    wrong_version["schema_version"] = json!(2);
    let mut wrong_frame = token["artifact"].clone();
    wrong_frame["frame"] = json!("different-frame");
    let missing = v04_fixtures::artifact_variant("test-prism", "test-prism:Token", 0.75);
    let error = rpc(bindings::prepare(
        &store,
        "test-prism",
        "collect_tokens",
        json!({
            "tokens":[marker["artifact"],wrong_version,wrong_frame,missing["artifact"]],
            "guides":[token["artifact"]]
        }),
    )
    .unwrap_err());
    assert_eq!(error["code"], -32602);
    assert_eq!(error["data"]["reason"], "binding");
    assert_eq!(error["data"]["run"], Value::Null);
    let _: table_manners::Refusal = serde_json::from_value(error["data"].clone()).unwrap();
    let problems = error["data"]["bindings"].as_array().unwrap();
    assert_eq!(
        problems
            .iter()
            .map(|b| (
                b["port"].as_str().unwrap(),
                b["index"].as_u64().unwrap(),
                b["problem"].as_str().unwrap()
            ))
            .collect::<Vec<_>>(),
        [
            ("tokens", 0, "wrong_type"),
            ("tokens", 1, "wrong_version"),
            ("tokens", 2, "wrong_frame"),
            ("tokens", 3, "unresolved"),
            ("guides", 0, "wrong_type")
        ]
    );
    assert!(problems[3]["remedy"].is_string());
    let good = bindings::prepare(
        &store,
        "test-prism",
        "collect_tokens",
        json!({"tokens":[token["artifact"],token["artifact"]],"guides":[marker["artifact"]]}),
    )
    .unwrap();
    assert_eq!(
        good.inputs
            .iter()
            .map(|i| i.port.as_str())
            .collect::<Vec<_>>(),
        ["tokens", "tokens", "guides"]
    );
    assert_eq!(good.arguments["tokens"].as_array().unwrap().len(), 2);
}

#[test]
fn scalar_refusals_have_no_index_and_schema_failures_are_plain_invalid_params() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let token = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    let error = rpc(bindings::prepare(
        &store,
        "sample-lab",
        "soften_token",
        json!({"token":token["artifact"]}),
    )
    .unwrap_err());
    assert_eq!(error["data"]["bindings"][0]["port"], "token");
    assert!(error["data"]["bindings"][0].get("index").is_none());
    for (action, args) in [
        ("unknown", json!({})),
        ("soften_token", json!({})),
        ("collect_tokens", json!({"tokens":[]})),
        (
            "soften_token",
            json!({"token":token["artifact"],"amount":2}),
        ),
        (
            "soften_token",
            json!({"token":token["artifact"],"surprise":true}),
        ),
    ] {
        let error = rpc(bindings::prepare(&store, "sample-lab", action, args).unwrap_err());
        assert_eq!(error["code"], -32602);
        assert!(error.get("data").is_none());
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn actual_image_bytes_and_expected_fingerprint_are_checked_without_saving_or_allocating() {
    let root = tempfile::tempdir().unwrap();
    let allowed = tempfile::tempdir().unwrap();
    let store =
        v04_store::Store::new(root.path(), "sample-lab", vec![allowed.path().to_owned()]).unwrap();
    let document = v04_fixtures::artifact("sample-lab", "image");
    let file = allowed.path().join("original.png");
    std::fs::write(&file, include_bytes!("../ui/tests/fixtures/sample.png")).unwrap();
    for input in [image(), json!({"path":file,"expect":document["record"]})] {
        let prepared = bindings::prepare(
            &store,
            "sample-lab",
            "locate_marker",
            json!({"image":input}),
        )
        .unwrap();
        assert_eq!(
            prepared.arguments["image"],
            json!({"expect":document["record"]})
        );
        assert_eq!(prepared.inputs[0].document, document);
        assert_eq!(prepared.return_images, "resource");
        assert!(!prepared.image_blocks);
    }
    let mut wrong = image();
    wrong["expect"] = document["record"].clone();
    wrong["expect"]["width"] = json!(999);
    let mut corrupt = image();
    corrupt["inline"]["data"] = json!(STANDARD.encode(b"corrupt image"));
    for input in [wrong, corrupt, json!({"path":"/ungranted/synthetic.png"})] {
        match bindings::prepare(
            &store,
            "sample-lab",
            "locate_marker",
            json!({"image":input}),
        ) {
            Err(bindings::Failure::Invalid(message)) => assert!(!message.is_empty()),
            other => panic!("expected an invalid-input outcome, got {other:?}"),
        }
    }
    let mut pixel_hint = image();
    pixel_hint["expect"] = document["record"].clone();
    pixel_hint["expect"]["pixel_sha256"] = json!("0".repeat(64));
    assert!(
        bindings::prepare(
            &store,
            "sample-lab",
            "locate_marker",
            json!({"image":pixel_hint})
        )
        .is_ok()
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn image_refs_and_literal_locations_keep_distinct_invocation_forms() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let image = v04_fixtures::artifact("sample-lab", "image");
    let marker = v04_fixtures::artifact("sample-lab", "location");
    store.remember(&image).unwrap();
    let prepared = bindings::prepare(
        &store,
        "sample-lab",
        "stamp_marker",
        json!({"image":image["artifact"],"marker":marker["record"]}),
    )
    .unwrap();
    assert_eq!(prepared.arguments["image"], image["artifact"]);
    assert_eq!(prepared.arguments["marker"], marker["record"]);
    assert_eq!(prepared.inputs[1].document, marker);
    assert_eq!(
        store
            .preview(image["artifact"]["identity"].as_str().unwrap())
            .unwrap(),
        include_bytes!("../ui/tests/fixtures/sample.png")
    );
}

#[test]
fn task_images_are_bound_and_utilities_need_no_ports() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let task = bindings::prepare(
        &store,
        "sample-lab",
        "make_token",
        json!({"source":image()}),
    )
    .unwrap();
    assert_eq!(task.inputs[0].port, "source");
    assert_eq!(task.action["kind"], "task");
    let utility = bindings::prepare(&store, "sample-lab", "summarize", json!({})).unwrap();
    assert_eq!(utility.arguments, json!({}));
    assert!(utility.inputs.is_empty());
}

#[test]
fn portable_input_binds_after_the_producer_workspace_is_deleted() {
    let original = tempfile::tempdir().unwrap();
    let producer = v04_store::Store::new(original.path(), "sample-lab", vec![]).unwrap();
    let token = v04_fixtures::artifact_variant("sample-lab", "sample-lab:Token", 0.5);
    producer.remember(&token).unwrap();
    let exported = producer
        .export(json!({"artifact":token["artifact"]}))
        .unwrap();
    let bytes = std::fs::read(exported["file"]["path"].as_str().unwrap()).unwrap();
    original.close().unwrap();
    let destination = tempfile::tempdir().unwrap();
    let receiver = v04_store::Store::new(destination.path(), "sample-lab", vec![]).unwrap();
    let imported = receiver.import(json!({"inline":{"media_type":exported["file"]["media_type"],"data":STANDARD.encode(bytes)},"sha256":exported["file"]["sha256"]})).unwrap();
    let prepared = bindings::prepare(
        &receiver,
        "sample-lab",
        "soften_token",
        json!({"token":imported["artifact"],"amount":0.75}),
    )
    .unwrap();
    assert_eq!(prepared.inputs[0].document, token);
    assert_eq!(prepared.arguments["amount"], 0.75);
}

#[test]
fn optional_pixel_hints_never_override_matching_file_identity() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let image = v04_fixtures::artifact("sample-lab", "image");
    store.remember(&image).unwrap();
    let mut reference = image["artifact"].clone();
    reference["on"]["pixel_sha256"] = json!("0".repeat(64));
    let mut marker = v04_fixtures::artifact("sample-lab", "location")["record"].clone();
    marker["image"]["pixel_sha256"] = json!("1".repeat(64));
    let prepared = bindings::prepare(
        &store,
        "sample-lab",
        "stamp_marker",
        json!({"image":reference,"marker":marker}),
    )
    .unwrap();
    assert_eq!(prepared.arguments["image"], image["artifact"]);
    assert_eq!(prepared.arguments["marker"], marker);
}

#[test]
fn literal_input_failures_are_explicit_and_binding_still_wins_before_run_allocation() {
    let root = tempfile::tempdir().unwrap();
    let granted = tempfile::tempdir().unwrap();
    let store =
        v04_store::Store::new(root.path(), "sample-lab", vec![granted.path().to_owned()]).unwrap();
    let mut marker = v04_fixtures::artifact("sample-lab", "location")["record"].clone();
    marker["parts"][0]["column"] = json!(0.25);
    assert!(matches!(
        bindings::prepare(
            &store,
            "sample-lab",
            "stamp_marker",
            json!({"image":image(),"marker":marker})
        ),
        Err(bindings::Failure::Invalid(_))
    ));
    let missing_image = v04_fixtures::artifact("sample-lab", "image")["artifact"].clone();
    let refusal = rpc(bindings::prepare(
        &store,
        "sample-lab",
        "stamp_marker",
        json!({"image":missing_image,"marker":marker}),
    )
    .unwrap_err());
    assert_eq!(refusal["data"]["reason"], "binding");
    let remedy = refusal["data"]["bindings"][0]["remedy"].as_str().unwrap();
    assert!(remedy.contains("image") && remedy.contains("inline"));
    let path = granted.path().join("bad.png");
    std::fs::write(&path, b"not an image").unwrap();
    let mut wrong_media = image();
    wrong_media["inline"]["media_type"] = json!("image/jpeg");
    for source in [json!({"path":path}), wrong_media] {
        assert!(matches!(
            bindings::prepare(
                &store,
                "sample-lab",
                "locate_marker",
                json!({"image":source})
            ),
            Err(bindings::Failure::Invalid(_))
        ));
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}
