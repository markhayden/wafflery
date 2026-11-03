//! Cross-check declared action inputs against the MCP callable namespace.
use crate::tools::{Error, Result, parameter_names::ParameterNames};
use serde_json::Value;
use std::collections::BTreeMap;

fn entries(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}
/// The description has passed the published schema and cross-field checks.
pub(super) fn validate(document: &Value, schemas: &BTreeMap<String, Value>) -> Result<()> {
    if entries(&document["artifact_types"])
        .iter()
        .any(|kind| kind["portable"] != "none")
    {
        for control in ["export", "import"] {
            if !schemas.contains_key(control) {
                return Err(Error::Invalid(format!(
                    "Portable artifact types require the {control} MCP control"
                )));
            }
        }
    }
    let images = entries(&document["profiles"])
        .iter()
        .any(|profile| profile == "images");
    for action in entries(&document["actions"]) {
        let name = action["name"].as_str().unwrap();
        if action["cost"]["may_spend"] == true && !schemas.contains_key("estimate") {
            return Err(Error::Invalid(
                "Paid actions must expose the estimate MCP control".into(),
            ));
        }
        let schema = schemas
            .get(name)
            .ok_or_else(|| Error::Invalid(format!("Declared action {name} has no MCP tool")))?;
        let inspect = |schema| {
            ParameterNames::new(schema)
                .map_err(|error| Error::Invalid(format!("Action {name} input schema: {error}")))
        };
        let params = inspect(&action["params_schema"])?;
        let callable = inspect(schema)?;
        let mut profile_names = std::collections::BTreeSet::new();
        if images {
            profile_names.extend(["return_images", "image_blocks"]);
            if action["kind"] == "operation" {
                for port in entries(&action["ports"]["in"]) {
                    profile_names.insert(port["name"].as_str().unwrap());
                }
            } else {
                for slot in entries(&action["images"])
                    .iter()
                    .chain(entries(&action["locations"]))
                {
                    profile_names.insert(slot["slot"].as_str().unwrap());
                }
            }
        }
        for argument in params.explicit_names().chain(profile_names.iter().copied()) {
            if !callable.contains(argument) {
                return Err(Error::Invalid(format!(
                    "Action {name} MCP input schema is missing declared argument {argument}"
                )));
            }
        }
        for argument in callable.explicit_names() {
            if !params.contains(argument) && !profile_names.contains(argument) {
                return Err(Error::Invalid(format!(
                    "Action {name} MCP input schema adds undeclared argument {argument}"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../testkit/v04_schemas.rs"]
mod fixture_schemas;
#[cfg(test)]
#[allow(dead_code)]
#[path = "../../testkit/v04_fixtures.rs"]
mod fixtures;
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn peer() -> (Value, BTreeMap<String, Value>) {
        let document = fixtures::description("test-prism");
        let mut schemas: BTreeMap<_, _> = document["actions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|action| {
                (
                    action["name"].as_str().unwrap().into(),
                    fixture_schemas::input_schema(action),
                )
            })
            .collect();
        for control in ["describe", "export", "import"] {
            schemas.insert(control.into(), json!({"type":"object"}));
        }
        (document, schemas)
    }
    #[test]
    fn independent_operation_task_and_utility_callables_match() {
        let (document, schemas) = peer();
        super::super::validate_contract("urn:table-manners:core:description", &document).unwrap();
        validate(&document, &schemas).unwrap();
    }
    #[test]
    fn missing_controls_or_actions_are_refused() {
        let (document, schemas) = peer();
        for name in ["locate_marker", "export", "import"] {
            let mut missing = schemas.clone();
            missing.remove(name);
            assert!(validate(&document, &missing).is_err(), "{name}");
        }
        let mut paid = document;
        paid["actions"][0]["cost"]["may_spend"] = json!(true);
        assert!(validate(&paid, &schemas).is_err());
    }
    #[test]
    fn missing_profile_inputs_and_parameters_name_the_action_and_argument() {
        let (document, schemas) = peer();
        for (action, field) in [
            ("locate_marker", "image"),
            ("collect_tokens", "tokens"),
            ("make_token", "source"),
            ("soften_token", "amount"),
            ("summarize", "image_blocks"),
        ] {
            let mut missing = schemas.clone();
            missing.get_mut(action).unwrap()["properties"]
                .as_object_mut()
                .unwrap()
                .remove(field);
            if let Some(required) = missing.get_mut(action).unwrap()["required"].as_array_mut() {
                required.retain(|name| name != field);
            }
            let error = validate(&document, &missing).unwrap_err().to_string();
            assert!(error.contains(action) && error.contains(field), "{error}");
        }
    }
    #[test]
    fn injected_root_context_is_refused_but_nested_values_are_not_arguments() {
        let (mut document, mut schemas) = peer();
        schemas.get_mut("locate_marker").unwrap()["allOf"] =
            json!([{"required":["hidden_context"]}]);
        let error = validate(&document, &schemas).unwrap_err().to_string();
        assert!(
            error.contains("locate_marker") && error.contains("hidden_context"),
            "{error}"
        );
        schemas
            .get_mut("locate_marker")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("allOf");
        let options = json!({"type":"object","properties":{"hidden_context":{}}});
        document["actions"][0]["params_schema"]["properties"]["options"] = options.clone();
        schemas.get_mut("locate_marker").unwrap()["properties"]["options"] = options;
        validate(&document, &schemas).unwrap();
    }
    #[test]
    fn composed_callable_names_and_core_only_arguments_remain_valid() {
        let (document, mut schemas) = peer();
        let schema = schemas.get_mut("locate_marker").unwrap();
        let properties = schema
            .as_object_mut()
            .unwrap()
            .remove("properties")
            .unwrap();
        schema["$defs"] = json!({"inputs":{"properties":properties}});
        schema["$ref"] = json!("#/$defs/inputs");
        validate(&document, &schemas).unwrap();
        let document = json!({"profiles":[],"actions":[{"name":"core_action","kind":"task",
            "cost":{"may_spend":false},"params_schema":{"type":"object","properties":{"image_blocks":{"type":"boolean"}}}}]});
        let schemas = BTreeMap::from([(
            "core_action".into(),
            document["actions"][0]["params_schema"].clone(),
        )]);
        validate(&document, &schemas).unwrap();
    }
}
