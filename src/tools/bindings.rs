//! Preflight every image before copying; retain ordered, independently fingerprinted bindings.
use super::{Error, Result, client::Declaration, validator};
use crate::library::{
    assets::{Asset, AssetStore, MAX_BYTES},
    attempts::Submission,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const MAX_IMAGES: usize = 32;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

pub(super) async fn materialize(
    assets: &AssetStore,
    request: &Submission,
    action: &Value,
    declaration: &Declaration,
    directory: &Path,
) -> Result<(Value, Value)> {
    let mut arguments = request
        .params
        .as_object()
        .cloned()
        .ok_or_else(|| Error::Invalid("Parameters must be an object".into()))?;
    if action["locations"]
        .as_array()
        .is_some_and(|slots| slots.iter().any(|s| s["required"] == true))
    {
        return Err(Error::Invalid(
            "Required location bindings need the operations interface".into(),
        ));
    }
    let slots = action["images"].as_array().cloned().unwrap_or_default();
    if request
        .images
        .keys()
        .any(|name| !slots.iter().any(|s| s["slot"] == *name))
    {
        return Err(Error::Invalid("Undeclared image slot".into()));
    }
    if request.frames.iter().any(|(slot, frames)| {
        request
            .images
            .get(slot)
            .is_none_or(|images| images.len() != frames.len())
    }) {
        return Err(Error::Invalid(
            "Image coordinate metadata must match its image bindings".into(),
        ));
    }
    if request.images.values().map(Vec::len).sum::<usize>() > MAX_IMAGES {
        return Err(Error::Invalid(
            "An attempt supports at most 32 images".into(),
        ));
    }
    let reference_schema = validator(&json!({"$ref":"urn:table-manners:images:image-reference"}))?;
    let mut inputs = serde_json::Map::new();
    let mut copies: Vec<(Asset, PathBuf)> = Vec::new();
    let mut total_bytes = 0u64;
    for (slot_index, slot) in slots.iter().enumerate() {
        let name = slot["slot"]
            .as_str()
            .ok_or_else(|| Error::Invalid("Invalid image slot".into()))?;
        if arguments.contains_key(name) {
            return Err(Error::Invalid(
                "Image slots cannot be supplied as raw parameters".into(),
            ));
        }
        let supplied = request.images.get(name);
        let ids = supplied.map(Vec::as_slice).unwrap_or_default();
        let many = slot["many"] == true;
        if (!many && ids.len() > 1)
            || (ids.is_empty() && (slot["required"] == true || (many && supplied.is_some())))
        {
            return Err(Error::Invalid(format!(
                "Invalid image count for slot {name}"
            )));
        }
        if ids.is_empty() {
            continue;
        }
        let mut values = Vec::new();
        let mut evidence = Vec::new();
        for (index, id) in ids.iter().enumerate() {
            let asset = assets.metadata(id).await?;
            if u64::from(asset.size) > MAX_BYTES {
                return Err(Error::Invalid("Each image must be at most 128 MiB".into()));
            }
            total_bytes += u64::from(asset.size);
            if total_bytes > MAX_TOTAL_BYTES {
                return Err(Error::Invalid(
                    "Attempt images must total at most 512 MiB".into(),
                ));
            }
            let extension = if asset.media_type == "image/png" {
                "png"
            } else {
                "jpg"
            };
            let filename = if many {
                format!("input-{slot_index}-{index}.{extension}")
            } else {
                format!("input-{slot_index}.{extension}")
            };
            let path = directory.join(filename);
            let frame = request
                .frames
                .get(name)
                .and_then(|frames| frames.get(index))
                .cloned()
                .unwrap_or_default();
            let reference = json!({"file_sha256":asset.sha256,"width":asset.width,"height":asset.height,"projection":frame.projection,"topology":frame.topology});
            reference_schema
                .validate(&reference)
                .map_err(|e| Error::Invalid(format!("Invalid image coordinates: {e}")))?;
            values.push(json!({"path":path,"expect":reference}));
            evidence.push(json!({"asset":asset,"image":reference,"role":slot["role"]}));
            copies.push((asset, path));
        }
        arguments.insert(
            name.into(),
            if many {
                json!(values)
            } else {
                values.remove(0)
            },
        );
        inputs.insert(name.into(), json!(evidence));
    }
    if declaration.document["profiles"]
        .as_array()
        .is_some_and(|profiles| profiles.iter().any(|profile| profile == "images"))
    {
        arguments.insert("return_images".into(), json!("path"));
        arguments.insert("image_blocks".into(), json!(false));
    }
    let arguments = Value::Object(arguments);
    let schema = declaration
        .schemas
        .get(&request.action)
        .ok_or(Error::Missing("No MCP action schema"))?;
    validator(schema)?
        .validate(&arguments)
        .map_err(|e| Error::Invalid(format!("Invalid action inputs: {e}")))?;
    for (expected, path) in copies {
        let (asset, bytes) = assets.read(&expected.id).await?;
        if serde_json::to_value(&asset)? != serde_json::to_value(&expected)? {
            return Err(Error::Invalid(
                "Stored image metadata changed during binding".into(),
            ));
        }
        tokio::fs::write(path, bytes).await?;
    }
    Ok((arguments, Value::Object(inputs)))
}
