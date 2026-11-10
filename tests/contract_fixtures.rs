#[path = "../testkit/fixtures.rs"]
mod fixtures;
use serde_json::{Value, json};

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let (_, text) = table_manners::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or_else(|| format!("unknown embedded schema {uri}"))?;
        Ok(serde_json::from_str(text)?)
    }
}

fn validator(name: &str) -> jsonschema::Validator {
    let (_, text) = table_manners::SCHEMAS
        .iter()
        .find(|(id, _)| *id == name)
        .unwrap();
    jsonschema::options()
        .with_retriever(Embedded)
        .should_validate_formats(true)
        .build(&serde_json::from_str::<Value>(text).unwrap())
        .unwrap()
}

#[test]
fn owned_description_and_terminal_fixtures_match_the_published_contract() {
    let description = fixtures::description("0.6.0");
    validator("urn:table-manners:core:description")
        .validate(&description)
        .unwrap();
    let _: table_manners::Description = serde_json::from_value(description).unwrap();
    for status in ["completed", "cancelled", "invalid", "error"] {
        let mut result = fixtures::result(status);
        if matches!(status, "invalid" | "error") {
            result["error"] =
                json!({"code": "input", "retryable": false, "reason": "Fixture input error."});
        }
        validator("urn:table-manners:core:result")
            .validate(&result)
            .unwrap();
        let _: table_manners::RunResult = serde_json::from_value(result).unwrap();
    }
}

#[test]
fn schema_validity_does_not_authorize_an_unapproved_contract_version() {
    let schema = validator("urn:table-manners:core:description");
    for version in [
        json!("0.1.0"),
        json!("0.2.0"),
        json!("1.0.0"),
        json!(null),
        json!(1),
        json!("0.1"),
    ] {
        let mut document = fixtures::description("0.6.0");
        document["contract"] = version;
        let executable =
            schema.is_valid(&document) && document["contract"] == table_manners::CONTRACT_VERSION;
        assert!(!executable);
    }
    let mut missing = fixtures::description("0.6.0");
    missing.as_object_mut().unwrap().remove("contract");
    assert!(!schema.is_valid(&missing));
}

#[test]
fn missing_cost_and_malformed_list_slots_fail_schema_validation() {
    let schema = validator("urn:table-manners:core:description");
    let mut document = fixtures::description("0.6.0");
    document["actions"][0]["cost"]
        .as_object_mut()
        .unwrap()
        .remove("may_spend");
    assert!(!schema.is_valid(&document));
    let mut document = fixtures::description("0.6.0");
    document["actions"][0]["images"][0]["many"] = json!(true);
    assert!(schema.is_valid(&document));
    document["actions"][0]["images"][0]["many"] = json!("true");
    assert!(!schema.is_valid(&document));
}

#[test]
fn live_contract_gate_accepts_only_the_v05_version() {
    assert_eq!(table_manners::CONTRACT_VERSION, "0.6.0");
    let document = fixtures::description("0.6.0");
    wafflery::tools::validate_contract("urn:table-manners:core:description", &document).unwrap();
    for version in ["0.1.0", "0.2.0", "0.3.0", "0.4.0", "0.5.0", "0.7.0"] {
        let mut old = document.clone();
        old["contract"] = json!(version);
        let error = wafflery::tools::validate_contract("urn:table-manners:core:description", &old)
            .unwrap_err();
        assert!(error.to_string().contains("exactly 0.6.0"), "{error}");
    }
}
