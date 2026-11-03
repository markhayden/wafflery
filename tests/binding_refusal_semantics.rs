#[path = "../testkit/v04_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;
#[path = "../src/tools/binding_refusal_semantics.rs"]
mod refusal;
use refusal::Context;
use serde_json::{Value, json};
use wafflery::tools::{Error, Result, validator};

fn operation() -> Value {
    let description = fixtures::description("test-prism");
    description["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "collect_tokens")
        .unwrap()
        .clone()
}
fn arguments() -> Value {
    let token = fixtures::artifact("test-prism", "test-prism:Token")["artifact"].clone();
    let guide = fixtures::artifact("test-prism", "location")["artifact"].clone();
    json!({"tokens":[token.clone(), token], "guides":[guide]})
}

#[test]
fn many_refusals_identify_submitted_items_in_port_then_item_order() {
    let action = operation();
    let arguments = arguments();
    let mut data = fixtures::refusal("unresolved", Some("tokens"), Some(0));
    let second = fixtures::refusal("wrong_frame", Some("tokens"), Some(1))["bindings"][0].clone();
    let third = fixtures::refusal("wrong_type", Some("guides"), Some(0))["bindings"][0].clone();
    data["bindings"]
        .as_array_mut()
        .unwrap()
        .extend([second, third]);
    refusal::validate(
        &data,
        Context::Operation {
            action: &action,
            arguments: &arguments,
        },
    )
    .unwrap();
    for indices in [[1, 0, 2], [0, 0, 2], [2, 0, 1]] {
        let mut invalid = data.clone();
        invalid["bindings"] = json!(indices.map(|index| data["bindings"][index].clone()));
        assert!(
            refusal::validate(
                &invalid,
                Context::Operation {
                    action: &action,
                    arguments: &arguments
                }
            )
            .is_err()
        );
    }
}

#[test]
fn unknown_ports_absent_inputs_and_out_of_range_indices_are_not_binding_evidence() {
    let action = operation();
    let arguments = arguments();
    for (port, index) in [
        (Some("missing"), Some(0)),
        (None, None),
        (Some("tokens"), None),
        (Some("tokens"), Some(2)),
    ] {
        let data = fixtures::refusal("unresolved", port, index);
        assert!(
            refusal::validate(
                &data,
                Context::Operation {
                    action: &action,
                    arguments: &arguments
                }
            )
            .is_err()
        );
    }
    let data = fixtures::refusal("unresolved", Some("guides"), Some(0));
    let mut omitted = arguments.clone();
    omitted.as_object_mut().unwrap().remove("guides");
    assert!(
        refusal::validate(
            &data,
            Context::Operation {
                action: &action,
                arguments: &omitted
            }
        )
        .is_err()
    );
}

#[test]
fn singular_ports_have_no_index_and_tasks_cannot_report_operation_bindings() {
    let description = fixtures::description("test-prism");
    let action = description["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == "soften_token")
        .unwrap();
    let arguments =
        json!({"token":fixtures::artifact("test-prism","test-prism:Token")["artifact"]});
    let data = fixtures::refusal("wrong_version", Some("token"), None);
    refusal::validate(
        &data,
        Context::Operation {
            action,
            arguments: &arguments,
        },
    )
    .unwrap();
    let indexed = fixtures::refusal("wrong_version", Some("token"), Some(0));
    assert!(
        refusal::validate(
            &indexed,
            Context::Operation {
                action,
                arguments: &arguments
            }
        )
        .is_err()
    );
    let mut task = action.clone();
    task["kind"] = json!("task");
    task.as_object_mut().unwrap().remove("ports");
    assert!(
        refusal::validate(
            &data,
            Context::Operation {
                action: &task,
                arguments: &arguments
            }
        )
        .is_err()
    );
}

#[test]
fn artifact_reads_and_exports_only_report_the_single_unresolved_identity() {
    for (context, port) in [
        (Context::ArtifactRead, None),
        (Context::Export, Some("artifact")),
    ] {
        let data = fixtures::refusal("unresolved", port, None);
        refusal::validate(&data, context).unwrap();
        for invalid in [
            fixtures::refusal("wrong_type", port, None),
            fixtures::refusal("unresolved", Some("tokens"), Some(0)),
            json!({"reason":"binding","run":null,"bindings":[data["bindings"][0],data["bindings"][0]]}),
        ] {
            assert!(refusal::validate(&invalid, context).is_err());
        }
    }
}

#[test]
fn integral_json_numbers_are_valid_list_indices() {
    let action = operation();
    let arguments = arguments();
    let mut data = fixtures::refusal("unresolved", Some("tokens"), Some(1));
    data["bindings"][0]["index"] = json!(1.0);
    refusal::validate(
        &data,
        Context::Operation {
            action: &action,
            arguments: &arguments,
        },
    )
    .unwrap();
}

#[test]
fn other_valid_refusal_variants_are_rejected_without_a_panic() {
    let data = json!({"reason":"invalid_arguments", "run":null,
        "error":{"code":"usage","retryable":false,"reason":"Synthetic input refusal."}});
    assert!(refusal::validate(&data, Context::ArtifactRead).is_err());
}

#[test]
fn inline_images_and_locations_cannot_be_reported_as_failed_artifact_references() {
    let mut action = operation();
    for many in [false, true] {
        action["ports"]["in"] = json!([{"name":"image","type":"image","many":many}]);
        let data = fixtures::refusal("wrong_frame", Some("image"), many.then_some(0));
        for direct in [
            json!({"path":"/input.png"}),
            json!({"inline":{"media_type":"image/png","data":"AA=="}}),
            json!({"kind":"line","parts":[]}),
            json!({"identity":"not-a-reference"}),
        ] {
            let arguments = json!({"image":if many {json!([direct])} else {direct}});
            assert!(
                refusal::validate(
                    &data,
                    Context::Operation {
                        action: &action,
                        arguments: &arguments
                    }
                )
                .is_err()
            );
        }
    }
}
