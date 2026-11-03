use super::*;
use crate::tools::fixtures;
use serde_json::json;
use time::{Duration, format_description::well_known::Rfc3339};

fn now() -> OffsetDateTime {
    OffsetDateTime::parse("2026-11-01T00:00:00Z", &Rfc3339).unwrap()
}
fn validate(value: Value, session: &Value, cap: f64) -> Result<ValidatedEstimate> {
    ValidatedEstimate::validate(value, &session["tool"], "price_test", cap, session, now())
}
#[test]
fn accepts_ready_and_refused_independent_quotes_without_inventing_unknown_cost() {
    for refused in [None, Some("cap"), Some("budget"), Some("readiness")] {
        let session = fixtures::session("sample-lab");
        let value = fixtures::estimate("sample-lab", &session, refused);
        let session = value["session"].clone();
        let quote = validate(value.clone(), &session, value["cap_usd"].as_f64().unwrap()).unwrap();
        assert_eq!(quote.observation(), &value);
        if refused == Some("readiness") {
            assert!(quote.observation()["worst_case_usd"].is_null());
            assert!(quote.observation()["effective"]["config_identity"].is_null());
            let mut uncapped = value;
            uncapped["cap_usd"] = Value::Null;
            assert!(validate(uncapped, &session, 0.5).is_ok());
        }
    }
}
#[test]
fn rejects_schema_valid_identity_ledger_and_authorization_contradictions() {
    let session = fixtures::session("sample-lab");
    let original = fixtures::estimate("sample-lab", &session, None);
    let changes = [
        ("/action", json!("other_action")),
        ("/tool/name", json!("other-tool")),
        ("/tool/version", json!("changed")),
        ("/session_id", json!("33333333-3333-4333-8333-333333333333")),
        ("/session/id", json!("33333333-3333-4333-8333-333333333333")),
        ("/session/tool/name", json!("other-tool")),
        ("/session/tool/version", json!("changed")),
        ("/session/started", json!("2026-10-31T00:00:00Z")),
        ("/session/budget_usd", json!(3)),
        ("/session/remaining_usd", json!(1)),
        ("/session/unresolved_usd", json!(1)),
        ("/cap_usd", Value::Null),
        ("/cap_usd", json!(0)),
        ("/cap_usd", json!(0.25)),
        ("/cap_usd", json!(0.75)),
        ("/worst_case_usd", json!(0.6)),
        ("/readiness", fixtures::readiness("needs_setup")),
        ("/expires", json!("2026-10-31T23:59:59Z")),
        ("/expires", json!("2026-11-01T00:00:00Z")),
        ("/expires", json!("2026-11-02T00:00:01Z")),
    ];
    for (path, value) in changes {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = value;
        super::super::validate_contract("urn:table-manners:core:estimate", &changed).unwrap();
        assert!(validate(changed, &session, 0.5).is_err(), "{path}");
    }
    let mut cli = original.clone();
    cli["session_id"] = Value::Null;
    cli["session"] = Value::Null;
    super::super::validate_contract("urn:table-manners:core:estimate", &cli).unwrap();
    assert!(validate(cli, &session, 0.5).is_err());
    let mut over_budget = original;
    let mut low_budget = session;
    low_budget["budget_usd"] = json!(0.05);
    low_budget["remaining_usd"] = json!(0.05);
    over_budget["session"] = low_budget.clone();
    assert!(validate(over_budget, &low_budget, 0.5).is_err());
}
#[test]
fn refusals_must_match_readiness_and_the_exceeded_limit() {
    let session = fixtures::session("sample-lab");
    for (reason, cap) in [("cap", 0.5), ("budget", 0.5), ("readiness", 0.5)] {
        let mut value = fixtures::estimate("sample-lab", &session, None);
        value["allowed"] = json!(false);
        value["refused"] = json!(reason);
        super::super::validate_contract("urn:table-manners:core:estimate", &value).unwrap();
        assert!(validate(value, &session, cap).is_err(), "{reason}");
    }
    let mut value = fixtures::estimate("sample-lab", &session, Some("cap"));
    value["readiness"] = fixtures::readiness("unavailable");
    assert!(validate(value, &session, 0.01).is_err());
}
#[test]
fn honors_positive_caps_tolerance_expiry_and_opaque_defaults_identity() {
    let session = fixtures::session("sample-lab");
    let original = fixtures::estimate("sample-lab", &session, None);
    for cap in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(validate(original.clone(), &session, cap).is_err());
    }
    let mut value = original;
    value["cap_usd"] = json!(0.5000005);
    value["worst_case_usd"] = json!(0.5000005);
    value["effective"]["model"] = Value::Null; // No model is valid for an action that needs none.
    value["binding"] = json!(format!("sha256:{}", "9".repeat(64)));
    value["expires"] = json!((now() + Duration::hours(24)).format(&Rfc3339).unwrap());
    let quote = validate(value.clone(), &session, 0.5).unwrap();
    assert_eq!(quote.observation(), &value);
}
#[test]
fn malformed_estimates_never_become_quotes() {
    let session = fixtures::session("sample-lab");
    let mut value = fixtures::estimate("sample-lab", &session, None);
    value["effective"]["config_identity"] = Value::Null;
    assert!(validate(value, &session, 0.5).is_err());
    assert!(validate(json!({}), &session, 0.5).is_err());
}

#[test]
fn distinguishes_plain_invalid_params_from_the_exact_named_estimate_refusal() {
    let validate_refusal = |error: &rmcp::model::ErrorData| {
        super::validate_refusal(error, &json!({"kind":"task"}), &json!({}))
    };
    use rmcp::model::{ErrorCode, ErrorData};
    let refusal = fixtures::refusal("invalid_arguments");
    for data in [None, Some(refusal.clone())] {
        validate_refusal(&ErrorData::new(
            ErrorCode::INVALID_PARAMS,
            "bad inputs",
            data,
        ))
        .unwrap();
    }
    for data in [
        fixtures::refusal("not_ready"),
        fixtures::refusal("client_run_id_reused"),
        json!({"reason":"invalid_arguments","run":null}),
        json!({"reason":"invalid_arguments","run":null,"error":{"code":"input","retryable":true,"reason":"wrong"}}),
        json!({"reason":"invalid_arguments","run":null,"error":{"code":"credential","retryable":false,"reason":"wrong"}}),
    ] {
        assert!(
            validate_refusal(&ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "bad inputs",
                Some(data)
            ))
            .is_err()
        );
    }
    assert!(
        validate_refusal(&ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            "bad inputs",
            Some(refusal)
        ))
        .is_err()
    );
}

#[test]
fn requires_successful_structured_estimates_instead_of_text_or_run_results() {
    let session = fixtures::session("sample-lab");
    let value = fixtures::estimate("sample-lab", &session, None);
    let response = json!({"structuredContent":value,"content":[]});
    assert!(
        ValidatedEstimate::from_response(
            &response,
            &session["tool"],
            "price_test",
            0.5,
            &session,
            now()
        )
        .is_ok()
    );
    for response in [
        json!({"structuredContent":value,"isError":true,"content":[]}),
        json!({"content":[{"type":"text","text":value.to_string()}]}),
        json!({"structuredContent":{"status":"completed","run":{}},"content":[]}),
    ] {
        assert!(
            ValidatedEstimate::from_response(
                &response,
                &session["tool"],
                "price_test",
                0.5,
                &session,
                now()
            )
            .is_err()
        );
    }
}

#[test]
fn estimate_binding_refusals_match_the_requested_operation_and_exact_list_item() {
    let action = json!({"kind":"operation","ports":{"in":[{"name":"masks","many":true}]}});
    let artifact = json!({"type":"sample-lab:Mask","schema_version":1,"tool":"sample-lab",
        "identity":format!("sha256:{}","1".repeat(64)), "frame":null,
        "resource":"sample-lab://artifact/example"});
    let arguments = json!({"masks":[artifact.clone(),artifact]});
    let data = json!({"reason":"binding","run":null,"bindings":[{"port":"masks","index":1,
        "problem":"unresolved","expected":"sample-lab:Mask v1","got":"missing", "remedy":"Import its bundle."}]});
    let error = |data| rmcp::ErrorData::invalid_params("Cannot bind", Some(data));
    validate_refusal(&error(data.clone()), &action, &arguments).unwrap();
    assert!(validate_refusal(&error(data.clone()), &json!({"kind":"task"}), &arguments).is_err());
    for supplied in [
        json!({}),
        json!({"masks":[arguments["masks"][0]]}),
        json!({"masks":[arguments["masks"][0],{"path":"/image.png"}]}),
    ] {
        assert!(validate_refusal(&error(data.clone()), &action, &supplied).is_err());
    }
    for (field, value) in [
        ("port", json!("unknown")),
        ("index", json!(null)),
        ("index", json!(2)),
    ] {
        let mut invalid = data.clone();
        invalid["bindings"][0][field] = value;
        assert!(validate_refusal(&error(invalid), &action, &arguments).is_err());
    }
    let mut known_run = data;
    known_run["run"] = json!({"id":"allocated"});
    assert!(validate_refusal(&error(known_run), &action, &arguments).is_err());
}
