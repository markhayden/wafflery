//! Check a verified archive against the receiver's current public input declaration.
pub use super::bundle_reader::{Input, InputClaim};
use super::{Error, Result, bundle_reader::ValidatedBundle, validate_contract, validator};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct Plan {
    pub action: String,
    pub params: Value,
    pub inputs: Vec<InputClaim>,
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
/// No tool calls or file publication. Original results are not authoritative input schemas.
pub fn prepare(bundle: &ValidatedBundle, index: usize, description: &Value) -> Result<Plan> {
    validate_contract("urn:table-manners:core:description", description)?;
    let step = bundle.manifest()["steps"]
        .get(index)
        .ok_or(Error::Missing("Bundle step is unavailable"))?;
    if step["tool"]["name"] != description["tool"]["name"] {
        return Err(invalid("Bundle step belongs to a different tool"));
    }
    let action_name = step["invocation"]["action"].as_str().unwrap();
    let action = description["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["name"] == action_name)
        .ok_or(Error::Missing("The bundle action is not installed"))?;
    let operation = action["kind"] == "operation";
    let mut definitions = if operation {
        action["ports"]["in"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        ["images", "locations"]
            .into_iter()
            .flat_map(|field| {
                action[field]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(move |slot| {
                        let mut definition = slot.clone();
                        definition["name"] = slot["slot"].clone();
                        definition["type"] = json!(if field == "images" {
                            "image"
                        } else {
                            "location"
                        });
                        definition
                    })
            })
            .collect()
    };
    let mut params = step["invocation"]["arguments"].as_object().unwrap().clone();
    if description["profiles"]
        .as_array()
        .is_some_and(|profiles| profiles.iter().any(|p| p == "images"))
    {
        params.remove("return_images");
        params.remove("image_blocks");
    }
    let mut claims: BTreeMap<_, _> = bundle
        .input_claims(index)?
        .into_iter()
        .map(|claim| ((claim.name.clone(), claim.index), claim))
        .collect();
    let mut inputs = vec![];
    for definition in definitions.drain(..) {
        let name = definition["name"].as_str().unwrap();
        let value = params.remove(name);
        let Some(value) = value else {
            if definition["required"] == true {
                return Err(invalid(format!("Bundle is missing required input {name}")));
            }
            continue;
        };
        let values = if definition["many"] == true {
            let values = value
                .as_array()
                .ok_or_else(|| invalid(format!("Input {name} requires a list")))?
                .clone();
            if values.is_empty() {
                return Err(invalid(format!("Input {name} cannot be empty")));
            }
            values
        } else {
            if value.is_array() {
                return Err(invalid(format!("Input {name} requires one value")));
            }
            vec![value]
        };
        for (index, value) in values.into_iter().enumerate() {
            let claim = if let Some(claim) = claims.remove(&(name.to_owned(), index)) {
                claim
            } else if definition["type"] == "location" && value.get("resource").is_none() {
                validator(&json!({"$ref":"urn:table-manners:images:location"}))?
                    .validate(&value)
                    .map_err(|_| invalid(format!("Invalid shared location for {name}")))?;
                InputClaim {
                    name: name.into(),
                    index,
                    input: Input::Location { value },
                }
            } else {
                return Err(invalid(format!(
                    "Input {name} occurrence {index} has no supplied file or explicit dependency"
                )));
            };
            match &claim.input {
                Input::Image { .. } if definition["type"] == "image" => (),
                Input::Location { .. } if definition["type"] == "location" => (),
                Input::Artifact { reference, .. }
                    if operation && reference["type"] == definition["type"] =>
                {
                    super::coordinator::artifacts::validate_reference(reference, description)?;
                    if !description["artifact_types"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|kind| {
                            kind["type"] == reference["type"]
                                && matches!(kind["portable"].as_str(), Some("json" | "bundle"))
                        })
                    {
                        return Err(invalid(format!(
                            "Input {name} has no supported import format"
                        )));
                    }
                }
                Input::Unresolved { .. } => (),
                _ => {
                    return Err(invalid(format!(
                        "Bundle input {name} conflicts with its current declared type"
                    )));
                }
            }
            inputs.push(claim);
        }
    }
    if !claims.is_empty() {
        return Err(invalid(
            "Bundle names an input not declared by the installed action",
        ));
    }
    let params = Value::Object(params);
    validator(&action["params_schema"])?
        .validate(&params)
        .map_err(|error| {
            invalid(format!(
                "Recorded parameters do not match the installed action: {error}"
            ))
        })?;
    Ok(Plan {
        action: action_name.into(),
        params,
        inputs,
    })
}
