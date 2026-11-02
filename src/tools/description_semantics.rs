//! Cross-field relationships checked after the published description schema.
use super::{Error, Result};
use serde_json::Value;
use std::collections::BTreeSet;

fn entries(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}
fn parameter(schema: &Value, name: &str) -> bool {
    schema["properties"].get(name).is_some()
        || entries(&schema["required"]).iter().any(|item| item == name)
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

/// The caller must first validate the document with the exact contract schema.
pub fn description(document: &Value) -> Result<()> {
    let owner = document["tool"]["name"].as_str().unwrap();
    let images = entries(&document["profiles"]).iter().any(|p| p == "images");
    let mut types = BTreeSet::new();
    for item in entries(&document["artifact_types"]) {
        let name = item["type"].as_str().unwrap();
        if name != "location" && name.split_once(':').map(|(prefix, _)| prefix) != Some(owner) {
            return Err(invalid(format!(
                "Artifact type owner must be {owner}: {name}"
            )));
        }
        if !types.insert(name) {
            return Err(invalid(format!("Duplicate artifact type: {name}")));
        }
    }
    let groups = entries(&document["vocabulary"]["group"]);
    let mut actions = BTreeSet::new();
    for action in entries(&document["actions"]) {
        let name = action["name"].as_str().unwrap();
        if !actions.insert(name) {
            return Err(invalid(format!("Duplicate action name: {name}")));
        }
        if let Some(group) = action.get("group")
            && !groups.contains(group)
        {
            return Err(invalid(format!("Action {name} names an undeclared group")));
        }
        let params = &action["params_schema"];
        if images {
            for option in ["return_images", "image_blocks"] {
                if parameter(params, option) {
                    return Err(invalid(format!(
                        "Action {name} parameter shadows profile argument {option}"
                    )));
                }
            }
        }
        if action["kind"] == "operation" {
            for direction in ["in", "out"] {
                let mut ports = BTreeSet::new();
                for port in entries(&action["ports"][direction]) {
                    let port_name = port["name"].as_str().unwrap();
                    let kind = port["type"].as_str().unwrap();
                    if !ports.insert(port_name) {
                        return Err(invalid(format!(
                            "Duplicate {direction} port name in {name}: {port_name}"
                        )));
                    }
                    if kind != "image" && !types.contains(kind) {
                        return Err(invalid(format!("Undeclared port type in {name}: {kind}")));
                    }
                    if parameter(params, port_name) {
                        return Err(invalid(format!(
                            "Port {port_name} in {name} shadows a parameter"
                        )));
                    }
                    if direction == "in" && matches!(port_name, "return_images" | "image_blocks") {
                        return Err(invalid(format!(
                            "Port {port_name} in {name} shadows a profile argument"
                        )));
                    }
                }
            }
        } else {
            let mut inputs = BTreeSet::new();
            for slot in entries(&action["images"])
                .iter()
                .chain(entries(&action["locations"]))
            {
                let slot_name = slot["slot"].as_str().unwrap();
                if !inputs.insert(slot_name) {
                    return Err(invalid(format!(
                        "Duplicate input name in {name}: {slot_name}"
                    )));
                }
                if parameter(params, slot_name) {
                    return Err(invalid(format!(
                        "Input {slot_name} in {name} shadows a parameter"
                    )));
                }
                if matches!(slot_name, "return_images" | "image_blocks") {
                    return Err(invalid(format!(
                        "Input {slot_name} in {name} shadows a profile argument"
                    )));
                }
            }
        }
    }
    Ok(())
}
