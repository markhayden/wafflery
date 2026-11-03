#[path = "../testkit/v04_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;
use serde_json::{Value, json};
use wafflery::tools::validate_contract;

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> std::result::Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let text = table_manners::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or("Unknown schema")?
            .1;
        Ok(serde_json::from_str(text)?)
    }
}
fn shape(value: &Value) {
    let text = table_manners::SCHEMAS
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
    let error = validate_contract("urn:table-manners:core:description", &value)
        .unwrap_err()
        .to_string();
    assert!(error.contains(reason), "{error}");
}

#[test]
fn independent_descriptions_allow_reused_output_names_and_many_optional_ports() {
    for name in ["sample-lab", "test-prism"] {
        let value = fixtures::description(name);
        shape(&value);
        validate_contract("urn:table-manners:core:description", &value).unwrap();
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
    validate_contract("urn:table-manners:core:description", &value).unwrap();
}

#[test]
fn malformed_descriptions_are_rejected_before_relationship_checks() {
    for value in [json!(null), json!({"contract":"0.4.0"})] {
        assert!(validate_contract("urn:table-manners:core:description", &value).is_err());
    }
    let mut value = fixtures::description("test-prism");
    value["actions"][0]["ports"]["in"][0]["name"] = json!(null);
    assert!(validate_contract("urn:table-manners:core:description", &value).is_err());
}

#[test]
fn composed_referenced_and_pattern_parameters_cannot_shadow_profile_inputs() {
    for fragment in [
        json!({"allOf":[{"properties":{"image":{"type":"string"}}}]}),
        json!({"$defs":{"inputs":{"properties":{"image":{}}}},"$ref":"#/$defs/inputs"}),
        json!({"patternProperties":{"^image$":{}}}),
        json!({"dependentRequired":{"gate":["image"]}}),
        json!({"anyOf":[{"required":["image"]},{"properties":{"safe":{}}}]}),
    ] {
        let mut value = fixtures::description("test-prism");
        value["actions"][0]["params_schema"]
            .as_object_mut()
            .unwrap()
            .extend(fragment.as_object().unwrap().clone());
        refused(value, "Port image in locate_marker shadows a parameter");
    }
    let mut value = fixtures::description("test-prism");
    value["actions"][0]["params_schema"]["allOf"] = json!([{"properties":{"return_images":{}}}]);
    refused(
        value,
        "Action locate_marker parameter shadows profile argument return_images",
    );
    let mut value = fixtures::description("test-prism");
    value["actions"][4]["params_schema"]["allOf"] = json!([{"properties":{"source":{}}}]);
    refused(value, "Input source in make_token shadows a parameter");
}

#[test]
fn nested_values_and_unused_definitions_do_not_shadow_profile_inputs() {
    let mut value = fixtures::description("test-prism");
    value["actions"][0]["params_schema"]["properties"] = json!({
        "settings":{"properties":{"image":{},"return_images":{}}}
    });
    value["actions"][0]["params_schema"]["$defs"] = json!({
        "unused":{"properties":{"image":{},"return_images":{}}}
    });
    shape(&value);
    validate_contract("urn:table-manners:core:description", &value).unwrap();
}

#[test]
fn unresolvable_parameter_schema_names_the_action() {
    let mut value = fixtures::description("test-prism");
    value["actions"][0]["params_schema"]["$ref"] =
        json!("https://example.invalid/unavailable.json");
    refused(value, "Action locate_marker parameter schema");
}
