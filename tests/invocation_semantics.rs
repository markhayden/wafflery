#[path = "../testkit/v04_fixtures.rs"]
#[allow(dead_code)]
mod fixtures;
#[path = "../src/tools/invocation_semantics.rs"]
mod invocation;
use serde_json::json;
use wafflery::tools::{Error, Result};

#[test]
fn resolved_invocations_belong_to_the_result_action_on_every_outcome() {
    for status in ["completed", "cancelled", "error", "invalid"] {
        let mut result = fixtures::result("test-prism", status);
        invocation::validate(&result, true).unwrap();
        if status == "invalid" {
            // Schema permits unresolved inputs only for this outcome.
            assert!(result["invocation"].is_null());
            continue;
        }
        result["invocation"]["action"] = json!("another_action");
        assert!(invocation::validate(&result, true).is_err(), "{status}");
    }
}

#[test]
fn image_delivery_options_cannot_be_recorded_as_replay_arguments() {
    for option in ["return_images", "image_blocks"] {
        let mut result = fixtures::result("test-prism", "completed");
        result["invocation"]["arguments"][option] = if option == "return_images" {
            json!("path")
        } else {
            json!(false)
        };
        assert!(invocation::validate(&result, true).is_err());
        // A core-only tool can own these names; they are reserved by the images profile.
        invocation::validate(&result, false).unwrap();
    }
}

#[test]
fn opaque_arguments_and_resolved_defaults_are_preserved_without_interpretation() {
    let mut result = fixtures::result("test-prism", "completed");
    result["invocation"]["arguments"]["amount"] = json!(0.25);
    result["invocation"]["arguments"]["label"] =
        json!("return_images and image_blocks are ordinary text here");
    result["invocation"]["arguments"]["options"] =
        json!({"return_images":"a tool-owned nested value"});
    let before = result.clone();
    invocation::validate(&result, true).unwrap();
    assert_eq!(result, before);
}
