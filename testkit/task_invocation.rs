//! Portable invocations for the owned task fake, independent of host types.
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Arguments have passed the fake's callable schema; image references come from its
/// resolved inputs, not the caller's unverified `expect`. This only applies defaults
/// from the owned fake's flat parameter schemas, not arbitrary JSON Schema programs.
/// The caller supplies a validated action and effective settings, references in
/// submitted order within each slot, and only the requirement names actually used.
/// Artifact operations use the separate operations peer, never this task helper.
pub fn build(
    action: &Value,
    arguments: &Value,
    images: &Value,
    effective: &Value,
    requirements: &[&str],
) -> Result<Value, String> {
    let mut resolved = arguments
        .as_object()
        .ok_or("Task arguments must be an object")?
        .clone();
    for (name, schema) in action["params_schema"]["properties"]
        .as_object()
        .into_iter()
        .flatten()
    {
        if let Some(default) = schema.get("default") {
            resolved
                .entry(name.clone())
                .or_insert_with(|| default.clone());
        }
    }
    resolved.remove("return_images");
    resolved.remove("image_blocks");
    let images = images.as_array().ok_or("Missing resolved images")?;
    let slots: Vec<_> = action["images"].as_array().into_iter().flatten().collect();
    if images
        .iter()
        .any(|image| !slots.iter().any(|slot| slot["slot"] == image["slot"]))
    {
        return Err("Resolved image belongs to an undeclared slot".into());
    }
    for slot in slots {
        let name = slot["slot"].as_str().ok_or("Missing image slot name")?;
        let references: Vec<_> = images
            .iter()
            .filter(|image| image["slot"] == name)
            .collect();
        let Some(submitted) = arguments.get(name) else {
            if slot["required"] == true || !references.is_empty() {
                return Err(format!("Missing submitted image slot {name}"));
            }
            continue;
        };
        let many = slot["many"] == true;
        let count = if many {
            submitted
                .as_array()
                .ok_or("List image slot is not an array")?
                .len()
        } else {
            1
        };
        if references.len() != count || references.iter().any(|image| !image["image"].is_object()) {
            return Err(format!("Incomplete resolved image slot {name}"));
        }
        let portable: Vec<_> = references
            .into_iter()
            .map(|image| json!({"expect":image["image"]}))
            .collect();
        resolved.insert(
            name.into(),
            if many {
                json!(portable)
            } else {
                portable[0].clone()
            },
        );
    }
    let mut names = BTreeSet::new();
    for name in requirements {
        if name.is_empty()
            || name.chars().count() > 256
            || name.contains(['/', '\\'])
            || !names.insert(name)
        {
            return Err("Requirements must be unique names, never file paths".into());
        }
    }
    Ok(json!({"action":action["name"],"arguments":resolved,
        "effective":effective,"requirements":requirements}))
}
