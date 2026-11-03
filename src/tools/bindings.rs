//! Preflight every image before copying; retain ordered, independently fingerprinted bindings.
use super::{Error, Result, client::Declaration, validator};
use crate::library::{
    assets::{Asset, AssetStore, MAX_BYTES},
    attempts::{InputBinding, Submission},
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

#[path = "port_inputs.rs"]
mod port_inputs;

const MAX_IMAGES: usize = 32;
const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

pub(super) async fn materialize(
    assets: &AssetStore,
    request: &Submission,
    action: &Value,
    declaration: &Declaration,
    directory: &Path,
) -> Result<(Value, Value)> {
    let (mut arguments, slots) = port_inputs::plan(request, action)?;
    if request.bound_images().len() > MAX_IMAGES {
        return Err(Error::Invalid(
            "An attempt supports at most 32 images".into(),
        ));
    }
    let reference_schema = validator(&json!({"$ref":"urn:table-manners:images:image-reference"}))?;
    let mut inputs = serde_json::Map::new();
    let mut copies: Vec<(Asset, PathBuf)> = Vec::new();
    let mut total_bytes = 0u64;
    for (slot_index, slot) in slots.iter().enumerate() {
        let name = &slot.name;
        let many = slot.many;
        if slot.items.is_empty() {
            continue;
        }
        let mut values = Vec::new();
        let mut evidence = Vec::new();
        for (index, item) in slot.items.iter().enumerate() {
            let (id, frame) = match item {
                InputBinding::Image { asset, frame } => (asset, frame),
                InputBinding::Artifact { reference } => {
                    values.push(reference.clone());
                    continue;
                }
                InputBinding::Location { value } => {
                    values.push(value.clone());
                    continue;
                }
            };
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
            let reference = json!({"file_sha256":asset.sha256,"width":asset.width,"height":asset.height,"projection":frame.projection,"topology":frame.topology});
            reference_schema
                .validate(&reference)
                .map_err(|e| Error::Invalid(format!("Invalid image coordinates: {e}")))?;
            values.push(json!({"path":path,"expect":reference}));
            evidence.push(
                json!({"asset":asset,"image":reference,"role":slot.role,"argument_index":index}),
            );
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
        if !evidence.is_empty() {
            inputs.insert(name.into(), json!(evidence));
        }
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

#[cfg(test)]
mod operation_tests {
    use super::*;
    use std::collections::BTreeMap;
    #[tokio::test]
    async fn mixed_image_lists_copy_only_assets_and_preserve_all_occurrences_in_order() {
        let root = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&root.path().join("Library")).unwrap();
        let path = root.path().join("input.png");
        std::fs::write(&path, include_bytes!("../../ui/tests/fixtures/sample.png")).unwrap();
        let asset = assets.import_path(path).await.unwrap();
        let reference = json!({"type":"image","schema_version":1,"tool":"other-tool","identity":format!("sha256:{}",asset.sha256),"frame":null,"resource":format!("other-tool://artifact/sha256:{}",asset.sha256)});
        let request:Submission=serde_json::from_value(json!({"id":"mixed","tool":"peer","action":"mix","revision":"r","params":{},"images":{},"bindings":{"pictures":[
            {"kind":"image","asset":asset.id},{"kind":"artifact","reference":reference},{"kind":"image","asset":asset.id}
        ]}})).unwrap();
        let action = json!({"name":"mix","kind":"operation","ports":{"in":[{"name":"pictures","type":"image","required":true,"many":true}]}});
        let declaration = Declaration {
            document: json!({"profiles":["images","operations"]}),
            revision: "r".into(),
            generation: 0,
            tool_session: Value::Null,
            schemas: BTreeMap::from([(
                "mix".into(),
                json!({"type":"object","required":["pictures"],"properties":{"pictures":{"type":"array","minItems":3,"items":{"anyOf":[{"$ref":"urn:table-manners:images:image-in"},{"$ref":"urn:table-manners:operations:artifact-ref"}]}}}}),
            )]),
        };
        let dir = root.path().join("copy");
        std::fs::create_dir(&dir).unwrap();
        let (args, inputs) = materialize(&assets, &request, &action, &declaration, &dir)
            .await
            .unwrap();
        assert_eq!(args["pictures"][1], reference);
        for index in [0, 2] {
            assert!(Path::new(args["pictures"][index]["path"].as_str().unwrap()).is_file());
        }
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
        assert_eq!(inputs["pictures"].as_array().unwrap().len(), 2);
        assert_eq!(inputs["pictures"][1]["argument_index"], 2);
        assert_eq!(request.bound_images().len(), 2);
        let mut collision = request.clone();
        collision.params = json!({"pictures":[]});
        assert!(
            materialize(&assets, &collision, &action, &declaration, &dir)
                .await
                .is_err()
        );
        assets.database.shutdown().await.unwrap();
    }
}
