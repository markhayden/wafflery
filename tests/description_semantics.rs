#[path = "../testkit/v04_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;
// Compile the next live validator without changing the current wire-version gate.
#[path = "../src/tools/description_semantics.rs"]
mod semantics;
use serde_json::{Value, json};
use wafflery::tools::{Error, Result};

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let text = table_manners_v04::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or("Unknown schema")?
            .1;
        Ok(serde_json::from_str(text)?)
    }
}
fn shape(value: &Value) {
    let text = table_manners_v04::SCHEMAS
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:core:description")
        .unwrap()
        .1;
    jsonschema::options()
        .with_retriever(Embedded)
        .build(&serde_json::from_str::<Value>(text).unwrap())
        .unwrap()
        .validate(value)
        .unwrap();
}
fn refused(value: Value, reason: &str) {
    // These examples pass the published structural schema but violate contract relationships.
    shape(&value);
    let error = semantics::description(&value).unwrap_err().to_string();
    assert!(error.contains(reason), "{error}");
}

#[test]
fn independent_descriptions_allow_reused_output_names_and_many_optional_ports() {
    for name in ["sample-lab", "test-prism"] {
        let value = fixtures::description(name);
        shape(&value);
        semantics::description(&value).unwrap();
    }
}

#[test]
fn groups_and_action_names_cannot_ambiguously_select_an_action() {
    let mut value = fixtures::description("test-prism");
    value["actions"][0]["group"] = json!("undeclared");
    refused(value, "group");
    let mut value = fixtures::description("test-prism");
    value["actions"][1]["name"] = value["actions"][0]["name"].clone();
    refused(value, "action name");
}

#[test]
fn native_types_must_be_owned_declared_and_unambiguous() {
    let mut value = fixtures::description("test-prism");
    value["artifact_types"][1]["type"] = json!("another-tool:Token");
    refused(value, "type owner");
    let mut value = fixtures::description("test-prism");
    value["artifact_types"][2]["type"] = value["artifact_types"][1]["type"].clone();
    refused(value, "artifact type");
    for direction in ["in", "out"] {
        let mut value = fixtures::description("test-prism");
        value["actions"][0]["ports"][direction][0]["type"] = json!("test-prism:Missing");
        refused(value, "Undeclared port type");
    }
}

#[test]
fn port_names_are_unique_per_direction_and_do_not_replace_parameters() {
    for direction in ["in", "out"] {
        let mut value = fixtures::description("test-prism");
        let duplicate = value["actions"][0]["ports"][direction][0].clone();
        value["actions"][0]["ports"][direction]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        refused(value, "port name");
        let mut value = fixtures::description("test-prism");
        let name = value["actions"][0]["ports"][direction][0]["name"]
            .as_str()
            .unwrap()
            .to_owned();
        value["actions"][0]["params_schema"]["properties"][name] = json!({"type":"string"});
        refused(value, "parameter");
    }
}

#[test]
fn image_location_and_control_inputs_cannot_shadow_each_other() {
    let base = fixtures::description("test-prism");
    let task = base["actions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|a| a["kind"] == "task")
        .unwrap();
    let mut value = base.clone();
    let slot = value["actions"][task]["images"][0].clone();
    value["actions"][task]["images"]
        .as_array_mut()
        .unwrap()
        .push(slot);
    refused(value, "input name");
    let mut value = base.clone();
    value["actions"][task]["locations"] =
        json!([{"slot":"source","kinds":["line"],"required":false}]);
    refused(value, "input name");
    let mut value = base.clone();
    value["actions"][task]["params_schema"]["properties"]["source"] = json!({"type":"string"});
    refused(value, "parameter");
    for name in ["return_images", "image_blocks"] {
        let mut value = base.clone();
        value["actions"][0]["ports"]["in"][0]["name"] = json!(name);
        refused(value, "profile argument");
        let mut value = base.clone();
        value["actions"][task]["params_schema"]["properties"][name] = json!({"type":"string"});
        refused(value, "profile argument");
    }
}

#[test]
fn core_only_tasks_need_no_image_or_artifact_metadata() {
    let mut value = fixtures::description("test-prism");
    value["profiles"] = json!([]);
    value["actions"] = json!([value["actions"][5]]);
    value.as_object_mut().unwrap().remove("artifact_types");
    value.as_object_mut().unwrap().remove("vocabulary");
    value.as_object_mut().unwrap().remove("limits");
    for field in ["images", "locations", "outputs"] {
        value["actions"][0].as_object_mut().unwrap().remove(field);
    }
    // A core-only action may own this name; the images profile is what reserves it.
    value["actions"][0]["params_schema"]["properties"]["image_blocks"] = json!({"type":"boolean"});
    shape(&value);
    semantics::description(&value).unwrap();
}
