#[path = "../testkit/task_invocation.rs"]
mod invocation;
use serde_json::{Value, json};

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let text = table_manners_v04::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or("Unknown schema")?
            .1;
        Ok(serde_json::from_str(text)?)
    }
}
fn action() -> Value {
    json!({"name":"mark_band","params_schema":{"properties":{
        "tone":{"default":"blue"},"strength":{"default":0.5},"filler":{"default":false}
    }},"images":[{"slot":"source","required":true},{"slot":"references","many":true}]})
}
fn reference(digit: &str) -> Value {
    json!({"file_sha256":digit.repeat(64),"width":160,"height":100,"projection":"flat","topology":"bounded"})
}
fn effective() -> Value {
    json!({"model":null,"config_identity":format!("sha256:{}","1".repeat(64)),"x-sample-lab":{"constants":"band-v1"}})
}
fn check(value: &Value) {
    jsonschema::options()
        .with_retriever(Embedded)
        .build(&json!({"$ref":"urn:table-manners:core:result#/$defs/invocation"}))
        .unwrap()
        .validate(value)
        .unwrap();
}

#[test]
fn invocation_preserves_resolved_parameters_and_uses_only_portable_images() {
    let arguments = json!({"label":"Test","strength":0.0,"max_spend_usd":0.25,
        "source":{"path":"/synthetic/source.png"},
        "references":[{"inline":{"media_type":"image/png","data":"synthetic"}},{"path":"/synthetic/reference.png"}],
        "return_images":"inline","image_blocks":true,
        "opaque":{"source":{"path":"symbolic-data"},"image_blocks":true},
        "location":{"space":"image","x":0.5}});
    let before = arguments.clone();
    let images = json!([{"slot":"references","image":reference("b")},
        {"slot":"source","image":reference("a")},{"slot":"references","image":reference("c")}]);
    let value = invocation::build(
        &action(),
        &arguments,
        &images,
        &effective(),
        &["sample.toml", "SAMPLE_KEY"],
    )
    .unwrap();
    check(&value);
    assert_eq!(value["action"], "mark_band");
    assert_eq!(
        value["arguments"],
        json!({"label":"Test","tone":"blue","strength":0.0,"filler":false,"max_spend_usd":0.25,
        "source":{"expect":reference("a")},"references":[{"expect":reference("b")},{"expect":reference("c")}],
        "opaque":before["opaque"],"location":before["location"]})
    );
    assert_eq!(value["effective"], effective());
    assert_eq!(value["requirements"], json!(["sample.toml", "SAMPLE_KEY"]));
    assert_eq!(arguments, before);
}

#[test]
fn optional_images_stay_absent_and_missing_defaults_are_applied() {
    let value = invocation::build(
        &action(),
        &json!({"label":"Test","source":{"path":"synthetic"}}),
        &json!([{"slot":"source","image":reference("a")}]),
        &effective(),
        &[],
    )
    .unwrap();
    check(&value);
    assert_eq!(value["arguments"]["strength"], 0.5);
    assert!(value["arguments"].get("references").is_none());
    assert_eq!(value["requirements"], json!([]));
}

#[test]
fn unresolved_or_mismatched_images_cannot_be_recorded_as_a_resolved_invocation() {
    let args = json!({"source":{"path":"synthetic"},"references":[{},{}]});
    for images in [
        json!([]),
        json!([{"slot":"source","image":reference("a")}]),
        json!([{"slot":"source","image":reference("a")},{"slot":"source","image":reference("b")}]),
    ] {
        assert!(invocation::build(&action(), &args, &images, &effective(), &[]).is_err());
    }
    assert!(invocation::build(&action(), &json!({}), &json!([]), &effective(), &[]).is_err());
}

#[test]
fn requirement_names_never_include_file_paths_and_are_unique() {
    let args = json!({"source":{}});
    let images = json!([{"slot":"source","image":reference("a")}]);
    for names in [
        vec!["/synthetic/sample.toml"],
        vec!["dir\\sample.toml"],
        vec![""],
        vec!["SAMPLE_KEY", "SAMPLE_KEY"],
    ] {
        assert!(invocation::build(&action(), &args, &images, &effective(), &names).is_err());
    }
}
