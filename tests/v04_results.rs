#[path = "../testkit/v04_results.rs"]
mod results;
#[allow(dead_code)]
#[path = "../testkit/v04_bindings.rs"]
mod v04_bindings;
#[allow(dead_code)]
#[path = "../testkit/v04_fixtures.rs"]
mod v04_fixtures;
#[allow(dead_code)]
#[path = "../testkit/v04_store.rs"]
mod v04_store;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let (_, text) = table_manners_v04::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or_else(|| format!("Unknown schema {uri}"))?;
        Ok(serde_json::from_str(text)?)
    }
}
fn check(run: &Value) {
    let (_, text) = table_manners_v04::SCHEMAS
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:core:result")
        .unwrap();
    jsonschema::options()
        .with_retriever(Embedded)
        .should_validate_formats(true)
        .build(&serde_json::from_str::<Value>(text).unwrap())
        .unwrap()
        .validate(run)
        .unwrap();
    assert_eq!(run["spend"]["usd"], 0);
}
fn info(root: &Path, id: &str) -> results::RunInfo {
    results::RunInfo {
        tool: "sample-lab".into(),
        root: root.to_owned(),
        started: "2026-11-02T12:00:00Z".into(),
        reference: json!({"id":id,"resource":format!("sample-lab://run/{id}/run.json"),"dir":null,"session_id":"11111111-1111-4111-8111-111111111111","client_run_id":format!("client-{id}")}),
    }
}
fn image() -> Value {
    json!({"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}})
}
fn execute(store: &v04_store::Store, root: &Path, id: &str, action: &str, args: Value) -> Value {
    let prepared = v04_bindings::prepare(store, "sample-lab", action, args).unwrap();
    let run = results::execute(store, &prepared, &info(root, id), || false);
    check(&run);
    assert_eq!(run["status"], "completed");
    assert_eq!(run["action"], action);
    run
}

#[test]
fn independent_steps_chain_recorded_references_and_effective_parameters() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let located = execute(
        &store,
        root.path(),
        "locate",
        "locate_marker",
        json!({"image":image()}),
    );
    let image_ref = &located["artifacts"]["in"][0]["artifact"];
    let marker = &located["artifacts"]["out"][0]["artifact"];
    assert_eq!(located["locations"]["found"].as_array().unwrap().len(), 1);
    assert_eq!(located["images"]["in"][0]["slot"], "image");
    let stamped = execute(
        &store,
        root.path(),
        "stamp",
        "stamp_marker",
        json!({"image":image_ref,"marker":marker}),
    );
    assert_eq!(stamped["invocation"]["arguments"]["image"], *image_ref);
    assert_eq!(stamped["locations"]["used"], located["locations"]["found"]);
    let token = &stamped["artifacts"]["out"][0]["artifact"];
    let softened = execute(
        &store,
        root.path(),
        "soften",
        "soften_token",
        json!({"token":token,"amount":0.75}),
    );
    let output = &softened["artifacts"]["out"][0];
    assert_ne!(output["artifact"]["identity"], token["identity"]);
    assert_eq!(
        store
            .read(output["artifact"]["identity"].as_str().unwrap())
            .unwrap()["record"]["amount"],
        0.75
    );
    assert_eq!(
        softened["invocation"]["arguments"],
        json!({"token":token,"amount":0.75})
    );
    assert_eq!(softened["images"]["out"][0], output["preview"]);
    assert!(
        !root.path().join("runs").exists(),
        "wire layer owns run persistence"
    );
}

#[test]
fn tasks_utilities_and_many_outputs_follow_their_declared_shapes() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let task = execute(
        &store,
        root.path(),
        "task",
        "make_token",
        json!({"source":image()}),
    );
    assert_eq!(task["artifacts"]["in"], json!([]));
    assert_eq!(task["images"]["in"][0]["slot"], "source");
    let first = &task["artifacts"]["out"][0]["artifact"];
    let other = execute(
        &store,
        root.path(),
        "other",
        "soften_token",
        json!({"token":first}),
    );
    assert_eq!(other["invocation"]["arguments"]["amount"], 0.25);
    let second = &other["artifacts"]["out"][0]["artifact"];
    let collected = execute(
        &store,
        root.path(),
        "collect",
        "collect_tokens",
        json!({"tokens":[second,first,second]}),
    );
    let out = collected["artifacts"]["out"].as_array().unwrap();
    assert_eq!(
        out.iter().map(|e| &e["artifact"]).collect::<Vec<_>>(),
        [second, first, second]
    );
    assert!(out.iter().all(|e| e["port"] == "tokens"));
    assert_eq!(collected["images"]["out"].as_array().unwrap().len(), 3);
    let utility = execute(&store, root.path(), "summary", "summarize", json!({}));
    assert_eq!(utility["images"]["out"], json!([]));
    assert_eq!(utility["artifacts"]["out"][0]["preview"], Value::Null);
    assert_eq!(
        utility["artifacts"]["out"][0]["artifact"]["type"],
        "sample-lab:Memo"
    );
}

#[test]
fn preview_locator_choice_never_changes_invocation_identity_or_original_bytes() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let mut identity = None;
    for delivery in ["resource", "inline", "path"] {
        let prepared = v04_bindings::prepare(
            &store,
            "sample-lab",
            "make_token",
            json!({"source":image(),"return_images":delivery,"image_blocks":true}),
        )
        .unwrap();
        assert!(prepared.image_blocks);
        let run = results::execute(&store, &prepared, &info(root.path(), delivery), || false);
        check(&run);
        let previous = identity.get_or_insert_with(|| run["input_identity"].clone());
        assert_eq!(run["input_identity"], *previous);
        assert_eq!(run["invocation"]["arguments"].as_object().unwrap().len(), 1);
        let out = &run["artifacts"]["out"][0];
        let preview = &out["preview"];
        assert_eq!(preview, &run["images"]["out"][0]);
        let bytes = match delivery {
            "inline" => STANDARD
                .decode(preview["inline"]["data"].as_str().unwrap())
                .unwrap(),
            "path" => std::fs::read(preview["path"].as_str().unwrap()).unwrap(),
            _ => store
                .preview(out["artifact"]["identity"].as_str().unwrap())
                .unwrap(),
        };
        let hash: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(hash, preview["image"]["file_sha256"]);
        let original_hash: String =
            Sha256::digest(include_bytes!("../ui/tests/fixtures/sample.png"))
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
        assert_eq!(
            run["images"]["in"][0]["image"]["file_sha256"],
            original_hash
        );
        let stored = store.read(&format!("sha256:{original_hash}")).unwrap();
        assert_eq!(stored["record"]["file_sha256"], original_hash);
    }
}

#[test]
fn cancelled_and_failed_outcomes_retain_completed_artifacts_and_invocation() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let completed = execute(
        &store,
        root.path(),
        "outcomes",
        "make_token",
        json!({"source":image()}),
    );
    for (outcome, status) in [
        (results::Outcome::Cancelled, "cancelled"),
        (
            results::Outcome::Error {
                code: "internal",
                reason: "Synthetic failure".into(),
            },
            "error",
        ),
        (results::Outcome::Completed, "completed"),
    ] {
        let mut run = completed.clone();
        results::settle(&mut run, outcome, Duration::from_millis(45));
        check(&run);
        assert_eq!(run["status"], status);
        assert_eq!(run["duration_ms"], 45);
        assert_eq!(run["artifacts"], completed["artifacts"]);
        assert_eq!(run["invocation"], completed["invocation"]);
        assert_eq!(run["error"].is_null(), status != "error");
    }
    let invalid = results::invalid(
        &info(root.path(), "invalid"),
        "make_token",
        &json!({"source":"bad"}),
        "Image could not be resolved",
    );
    check(&invalid);
    assert_eq!(invalid["status"], "invalid");
    assert_eq!(invalid["invocation"], Value::Null);
    assert_eq!(invalid["artifacts"], json!({"in":[],"out":[]}));
}

#[test]
fn preview_write_failure_is_a_run_error_with_its_resolved_invocation() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    std::fs::write(root.path().join("previews"), b"blocked path").unwrap();
    let prepared = v04_bindings::prepare(
        &store,
        "sample-lab",
        "make_token",
        json!({"source":image(),"return_images":"path"}),
    )
    .unwrap();
    let run = results::execute(&store, &prepared, &info(root.path(), "write-error"), || {
        false
    });
    check(&run);
    assert_eq!(run["status"], "error");
    assert_eq!(run["error"]["code"], "io");
    assert_eq!(run["invocation"]["action"], "make_token");
    assert_eq!(run["artifacts"]["out"], json!([]));
}

#[test]
fn cancellation_before_work_and_between_outputs_keeps_only_completed_work() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let prepared = v04_bindings::prepare(
        &store,
        "sample-lab",
        "make_token",
        json!({"source":image()}),
    )
    .unwrap();
    let stopped = results::execute(
        &store,
        &prepared,
        &info(root.path(), "early-cancel"),
        || true,
    );
    check(&stopped);
    assert_eq!(stopped["status"], "cancelled");
    assert_eq!(stopped["artifacts"]["out"], json!([]));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    let first = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    let second = v04_fixtures::artifact_variant("sample-lab", "sample-lab:Token", 0.75);
    store.remember(&first).unwrap();
    store.remember(&second).unwrap();
    let prepared = v04_bindings::prepare(
        &store,
        "sample-lab",
        "collect_tokens",
        json!({"tokens":[first["artifact"],second["artifact"]],"return_images":"path"}),
    )
    .unwrap();
    let partial = results::execute(
        &store,
        &prepared,
        &info(root.path(), "partial-cancel"),
        || root.path().join("previews").exists(),
    );
    check(&partial);
    assert_eq!(partial["status"], "cancelled");
    assert_eq!(partial["images"]["out"].as_array().unwrap().len(), 1);
    assert_eq!(partial["artifacts"]["out"].as_array().unwrap().len(), 1);
    assert_eq!(
        partial["artifacts"]["out"][0]["artifact"],
        first["artifact"]
    );
    assert_eq!(
        partial["invocation"]["arguments"]["tokens"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn a_late_write_failure_preserves_the_first_output_and_its_preview() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let token = v04_fixtures::artifact("sample-lab", "sample-lab:Token");
    store.remember(&token).unwrap();
    let second = execute(
        &store,
        root.path(),
        "second",
        "soften_token",
        json!({"token":token["artifact"],"amount":0.75,"return_images":"path"}),
    );
    let blocked = second["images"]["out"][0]["path"].as_str().unwrap();
    std::fs::remove_file(blocked).unwrap();
    std::fs::create_dir(blocked).unwrap();
    let prepared = v04_bindings::prepare(&store,"sample-lab","collect_tokens",json!({"tokens":[token["artifact"],second["artifacts"]["out"][0]["artifact"]],"return_images":"path"})).unwrap();
    let run = results::execute(
        &store,
        &prepared,
        &info(root.path(), "partial-error"),
        || false,
    );
    check(&run);
    assert_eq!(run["status"], "error");
    assert_eq!(run["error"]["code"], "io");
    assert_eq!(run["artifacts"]["out"].as_array().unwrap().len(), 1);
    assert_eq!(run["images"]["out"].as_array().unwrap().len(), 1);
    assert_eq!(run["artifacts"]["out"][0]["artifact"], token["artifact"]);
    assert!(!run["error"]["reason"].as_str().unwrap().starts_with('{'));
}

#[test]
fn cancellation_between_inputs_records_only_the_input_taken() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let marker = v04_fixtures::artifact("sample-lab", "location");
    let prepared = v04_bindings::prepare(
        &store,
        "sample-lab",
        "stamp_marker",
        json!({"image":image(),"marker":marker["record"]}),
    )
    .unwrap();
    let run = results::execute(
        &store,
        &prepared,
        &info(root.path(), "between-inputs"),
        || root.path().join("artifacts").exists(),
    );
    check(&run);
    assert_eq!(run["status"], "cancelled");
    assert_eq!(run["artifacts"]["in"].as_array().unwrap().len(), 1);
    assert_eq!(run["locations"]["used"], json!([]));
    assert!(
        store
            .read(marker["artifact"]["identity"].as_str().unwrap())
            .is_err()
    );
}

#[test]
fn finishing_elapsed_time_preserves_every_terminal_outcome() {
    let root = tempfile::tempdir().unwrap();
    let store = v04_store::Store::new(root.path(), "sample-lab", vec![]).unwrap();
    let completed = execute(&store, root.path(), "clock", "summarize", json!({}));
    let mut cancelled = completed.clone();
    results::settle(&mut cancelled, results::Outcome::Cancelled, Duration::ZERO);
    let mut failed = completed.clone();
    results::settle(
        &mut failed,
        results::Outcome::Error {
            code: "internal",
            reason: "Synthetic failure".into(),
        },
        Duration::ZERO,
    );
    let invalid = results::invalid(
        &info(root.path(), "invalid-clock"),
        "make_token",
        &json!({"source":"bad"}),
        "Invalid image",
    );
    for original in [completed, cancelled, failed, invalid] {
        let mut run = original.clone();
        results::finish(&mut run, Duration::from_millis(1234));
        check(&run);
        assert_eq!(run["finished"], "2026-11-02T12:00:01.234Z");
        assert_eq!(run["duration_ms"], 1234);
        for field in [
            "status",
            "error",
            "invocation",
            "artifacts",
            "images",
            "locations",
        ] {
            assert_eq!(run[field], original[field]);
        }
    }
}

#[test]
fn invalid_input_identity_excludes_delivery_flags() {
    let root = tempfile::tempdir().unwrap();
    let info = info(root.path(), "invalid-delivery");
    let first = results::invalid(
        &info,
        "make_token",
        &json!({"source":"bad"}),
        "Invalid image",
    );
    let second = results::invalid(
        &info,
        "make_token",
        &json!({"source":"bad","return_images":"inline","image_blocks":true}),
        "Invalid image",
    );
    assert_eq!(first["input_identity"], second["input_identity"]);
}
