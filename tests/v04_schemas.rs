#[allow(dead_code)]
#[path = "../testkit/v04_fixtures.rs"]
mod fixtures;
#[path = "../testkit/v04_schemas.rs"]
mod schemas;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};

fn validator(schema: &Value) -> jsonschema::Validator {
    fn no_refs(value: &Value) {
        match value {
            Value::Object(fields) => {
                assert!(!fields.contains_key("$ref"));
                for value in fields.values() {
                    no_refs(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    no_refs(value);
                }
            }
            _ => {}
        }
    }
    no_refs(schema);
    jsonschema::options()
        .should_validate_formats(true)
        .build(schema)
        .unwrap()
}
fn action(name: &str) -> Value {
    fixtures::description("test-prism")["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == name)
        .unwrap()
        .clone()
}
fn image() -> Value {
    json!({"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}})
}

#[test]
fn every_declared_action_has_a_complete_standalone_input_schema() {
    let token = fixtures::artifact("test-prism", "test-prism:Token");
    let marker = fixtures::artifact("test-prism", "location");
    for (name, arguments) in [
        ("locate_marker", json!({"image":image()})),
        (
            "stamp_marker",
            json!({"image":image(),"marker":marker["record"]}),
        ),
        (
            "soften_token",
            json!({"token":token["artifact"],"amount":0.4}),
        ),
        (
            "collect_tokens",
            json!({"tokens":[token["artifact"]],"guides":[marker["artifact"]]}),
        ),
        ("make_token", json!({"source":image()})),
        ("summarize", json!({})),
    ] {
        let schema = schemas::input_schema(&action(name));
        assert_eq!(schema["type"], "object");
        let check = validator(&schema);
        assert!(check.is_valid(&arguments), "{name}");
        let mut invalid = arguments.clone();
        invalid["undeclared"] = json!(true);
        assert!(
            !check.is_valid(&invalid),
            "{name} accepted an undeclared argument"
        );
        let mut delivered = arguments;
        delivered["return_images"] = json!("inline");
        delivered["image_blocks"] = json!(true);
        assert!(check.is_valid(&delivered));
        delivered["return_images"] = json!("invented");
        assert!(!check.is_valid(&delivered));
    }
}

#[test]
fn scalar_many_and_literal_forms_retain_their_schema_constraints() {
    let token = fixtures::artifact("test-prism", "test-prism:Token");
    let scalar = validator(&schemas::input_schema(&action("soften_token")));
    assert!(!scalar.is_valid(&json!({})));
    assert!(!scalar.is_valid(&json!({"token":[token["artifact"]]})));
    assert!(!scalar.is_valid(&json!({"token":token["artifact"],"amount":1.1})));
    let many = validator(&schemas::input_schema(&action("collect_tokens")));
    assert!(!many.is_valid(&json!({"tokens":[]})));
    assert!(!many.is_valid(&json!({"tokens":token["artifact"]})));
    assert!(!many.is_valid(&json!({"tokens":[token["artifact"]],"guides":[]})));
    let mut bad = token["artifact"].clone();
    bad["extra"] = json!(1);
    assert!(!many.is_valid(&json!({"tokens":[bad]})));
    let locations = validator(&schemas::input_schema(&action("stamp_marker")));
    assert!(!locations.is_valid(&json!({"image":image(),"marker":{"kind":"invented"}})));
    assert!(locations.is_valid(&json!({"image":fixtures::artifact("test-prism","image")["artifact"],"marker":fixtures::artifact("test-prism","location")["artifact"]})));
}

#[test]
fn ref_siblings_evaluate_properties_at_the_same_schema_level() {
    let schema = schemas::standalone(&json!({
        "$ref":"urn:table-manners:core:run-ref#/$defs/run_ref",
        "properties":{"label":{"type":"string"}},"required":["label"],
        "unevaluatedProperties":false
    }));
    let check = validator(&schema);
    let mut value = json!({"id":"one","resource":"test-prism://run/one/run.json","session_id":null,"client_run_id":null,"label":"Example"});
    assert!(check.is_valid(&value));
    value["extra"] = json!(true);
    assert!(!check.is_valid(&value));
}

#[test]
fn published_controls_also_expand_without_external_resolution() {
    for name in ["export-request", "import-request"] {
        let id = format!("urn:table-manners:operations:{name}");
        let text = table_manners_v04::SCHEMAS
            .iter()
            .find(|(uri, _)| *uri == id)
            .unwrap()
            .1;
        let schema = schemas::standalone(&serde_json::from_str(text).unwrap());
        let check = validator(&schema);
        let args = if name == "export-request" {
            json!({"artifact":fixtures::artifact("test-prism","test-prism:Token")["artifact"]})
        } else {
            json!({"inline":{"media_type":"application/json","data":"e30="}})
        };
        assert!(check.is_valid(&args), "{name}");
        assert!(!check.is_valid(&json!({})), "{name}");
    }
}
