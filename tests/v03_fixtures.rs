#[path = "../testkit/v03_fixtures.rs"]
mod fixtures;
use serde_json::{Value, json};

#[path = "support/v03_schema.rs"]
mod schema;
use schema::validator;
#[test]
fn schema_dependency_is_the_approved_contract_version() {
    assert_eq!(table_manners::CONTRACT_VERSION, "0.3.0");
    for (id, text) in table_manners::SCHEMAS {
        assert_eq!(serde_json::from_str::<Value>(text).unwrap()["$id"], *id);
    }
}
#[test]
fn independent_v03_fixtures_cover_readiness_estimates_and_refusals() {
    for tool in ["sample-lab", "test-lens"] {
        for state in ["ready", "needs_setup", "unavailable"] {
            let document = fixtures::description(tool, state);
            validator("description").validate(&document).unwrap();
            assert_eq!(document["tool"]["name"], tool);
            assert_eq!(document["contract"], "0.3.0");
            assert_eq!(document["actions"][0]["readiness"]["state"], "ready");
            assert_eq!(document["actions"][1]["readiness"]["state"], state);
            assert!(
                wafflery::tools::validate_contract("urn:table-manners:core:description", &document)
                    .is_ok()
            );
        }
        let session = fixtures::session(tool);
        validator("session").validate(&session).unwrap();
        for refused in [None, Some("cap"), Some("budget"), Some("readiness")] {
            let estimate = fixtures::estimate(tool, &session, refused);
            validator("estimate").validate(&estimate).unwrap();
            assert_eq!(estimate["session_id"], estimate["session"]["id"]);
            assert_eq!(estimate["tool"]["name"], tool);
            assert_eq!(estimate["allowed"], refused.is_none());
            assert!(estimate.get("run").is_none());
        }
    }
    for reason in ["client_run_id_reused", "not_ready", "invalid_arguments"] {
        validator("refusal")
            .validate(&fixtures::refusal(reason))
            .unwrap();
    }
}
#[test]
fn invalid_readiness_cap_and_estimate_fixtures_are_rejected() {
    let schema = validator("description");
    let valid = fixtures::description("sample-lab", "ready");
    let mut invalid = valid.clone();
    invalid["actions"][1]["readiness"]["state"] = json!("maybe");
    assert!(!schema.is_valid(&invalid));
    let mut invalid = valid.clone();
    invalid["actions"][1]["readiness"]["requirements"][0]["state"] = json!("missing");
    assert!(!schema.is_valid(&invalid));
    let mut invalid = valid.clone();
    invalid["actions"][1]["params_schema"]["properties"]
        .as_object_mut()
        .unwrap()
        .remove("max_spend_usd");
    assert!(!schema.is_valid(&invalid));
    let mut invalid = valid;
    invalid["actions"][1]["params_schema"]["properties"]["max_spend_usd"] =
        json!({"type":"number","minimum":0});
    assert!(!schema.is_valid(&invalid));
    let mut estimate = fixtures::estimate("sample-lab", &fixtures::session("sample-lab"), None);
    estimate["binding"] = json!("not-a-binding");
    assert!(!validator("estimate").is_valid(&estimate));
    let mut estimate = fixtures::estimate("sample-lab", &fixtures::session("sample-lab"), None);
    estimate["worst_case_usd"] = Value::Null;
    assert!(!validator("estimate").is_valid(&estimate));
    let mut refusal = fixtures::refusal("not_ready");
    refusal["requirements"] = json!([]);
    assert!(!validator("refusal").is_valid(&refusal));
}
