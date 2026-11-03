//! Normalize declared inputs; never resolve native identities or accept caller file paths.
use super::{Error, Result, validator};
use crate::library::attempts::{ImageFrame, InputBinding, Submission};
use serde_json::{Map, Value, json};

pub(super) struct Slot {
    pub name: String,
    pub role: Value,
    pub many: bool,
    pub items: Vec<InputBinding>,
}
pub(super) fn plan(
    request: &Submission,
    action: &Value,
) -> Result<(Map<String, Value>, Vec<Slot>)> {
    let arguments = request
        .params
        .as_object()
        .cloned()
        .ok_or_else(|| Error::Invalid("Parameters must be an object".into()))?;
    let operation = action["kind"] == "operation";
    if operation && (!request.images.is_empty() || !request.frames.is_empty()) {
        return Err(Error::Invalid(
            "Operations take images through their declared ports".into(),
        ));
    }
    let mut definitions = if operation {
        action["ports"]["in"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    } else {
        action["images"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|slot| {
                let mut slot = slot.clone();
                slot["name"] = slot["slot"].clone();
                slot["type"] = json!("image");
                slot
            })
            .collect::<Vec<_>>()
    };
    if !operation {
        definitions.extend(
            action["locations"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|slot| {
                    let mut slot = slot.clone();
                    slot["name"] = slot["slot"].clone();
                    slot["type"] = json!("location");
                    slot
                }),
        );
    }
    if request.images.keys().any(|name| {
        !definitions
            .iter()
            .any(|s| s["name"] == *name && s["type"] == "image")
    }) || request.bindings.keys().any(|name| {
        !definitions
            .iter()
            .any(|s| s["name"] == *name && (operation || s["type"] == "location"))
    }) {
        return Err(Error::Invalid("Undeclared input slot or port".into()));
    }
    if request.frames.iter().any(|(name, frames)| {
        request
            .images
            .get(name)
            .is_none_or(|ids| ids.len() != frames.len())
    }) {
        return Err(Error::Invalid(
            "Image coordinate metadata must match its image bindings".into(),
        ));
    }
    let artifact = validator(&json!({"$ref":"urn:table-manners:operations:artifact-ref"}))?;
    let location = validator(&json!({"$ref":"urn:table-manners:images:location"}))?;
    let mut slots = Vec::new();
    for definition in definitions {
        let name = definition["name"]
            .as_str()
            .ok_or_else(|| Error::Invalid("Missing input name".into()))?;
        if arguments.contains_key(name) {
            return Err(Error::Invalid(
                "Input slots cannot be supplied as raw parameters".into(),
            ));
        }
        let many = definition["many"] == true;
        let (supplied, items) = if !operation && definition["type"] == "image" {
            (
                request.images.contains_key(name),
                request
                    .images
                    .get(name)
                    .into_iter()
                    .flatten()
                    .enumerate()
                    .map(|(i, id)| InputBinding::Image {
                        asset: id.clone(),
                        frame: request
                            .frames
                            .get(name)
                            .and_then(|frames| frames.get(i))
                            .cloned()
                            .unwrap_or_default(),
                    })
                    .collect(),
            )
        } else {
            (
                request.bindings.contains_key(name),
                request.bindings.get(name).cloned().unwrap_or_default(),
            )
        };
        if (!many && items.len() > 1)
            || (items.is_empty() && (definition["required"] == true || (many && supplied)))
        {
            return Err(Error::Invalid(format!("Invalid input count for {name}")));
        }
        for item in &items {
            match item {
                InputBinding::Image {
                    frame: ImageFrame { .. },
                    ..
                } if definition["type"] == "image" => (),
                InputBinding::Artifact { reference } if operation => {
                    artifact.validate(reference).map_err(|e| {
                        Error::Invalid(format!("Invalid artifact reference for {name}: {e}"))
                    })?
                }
                InputBinding::Location { value } if definition["type"] == "location" => location
                    .validate(value)
                    .map_err(|e| Error::Invalid(format!("Invalid location for {name}: {e}")))?,
                _ => {
                    return Err(Error::Invalid(format!(
                        "Unsupported input binding for {name}"
                    )));
                }
            }
        }
        slots.push(Slot {
            name: name.into(),
            role: if operation {
                json!("source")
            } else {
                definition["role"].clone()
            },
            many,
            items,
        });
    }
    Ok((arguments, slots))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(bindings: Value) -> Submission {
        serde_json::from_value(json!({"id":"shape","tool":"test","action":"act","revision":"r","params":{},"images":{},"bindings":bindings})).unwrap()
    }
    #[test]
    fn optional_empty_scalar_is_omitted_but_a_supplied_many_port_cannot_be_empty() {
        let mut action = json!({"kind":"operation","ports":{"in":[{"name":"items","type":"test:Item","required":false,"many":false}]}});
        assert!(
            plan(&request(json!({"items":[]})), &action).unwrap().1[0]
                .items
                .is_empty()
        );
        action["ports"]["in"][0]["many"] = json!(true);
        assert!(plan(&request(json!({"items":[]})), &action).is_err());
        assert!(
            plan(&request(json!({})), &action).unwrap().1[0]
                .items
                .is_empty()
        );
    }
    #[test]
    fn task_locations_accept_shared_values_and_never_artifact_references() {
        let action =
            json!({"kind":"task","images":[],"locations":[{"slot":"selection","required":true}]});
        assert!(plan(&request(json!({})), &action).is_err());
        let value = json!({"image":{"file_sha256":"a".repeat(64),"width":10,"height":10,"projection":"flat","topology":"bounded"},"kind":"line","parts":[{"column":0.5,"width_px":1}],"straddles_wrap":false});
        let (_, slots) = plan(
            &request(json!({"selection":[{"kind":"location","value":value}]})),
            &action,
        )
        .unwrap();
        assert_eq!(slots[0].name, "selection");
        assert!(matches!(&slots[0].items[0],InputBinding::Location{value:saved} if *saved==value));
        assert!(
            plan(
                &request(json!({"selection":[{"kind":"artifact","reference":{}}]})),
                &action
            )
            .is_err()
        );
    }
}
