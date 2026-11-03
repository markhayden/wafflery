//! Single-step invocation bundles from immutable evidence and exact owned inputs.
use super::{Error, Result, validator};
use crate::library::{
    assets::AssetStore,
    attempts::Attempt,
    blob_links::{BlobLink, Owner},
    blobs::BlobStore,
    bundle_archive::{self, CanonicalArchive, MAX_MEMBER, MAX_MEMBERS, MAX_TOTAL},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// Whole slots/ports only: the contract's omission record has no occurrence index.
    #[serde(default)]
    pub omit: BTreeMap<String, String>,
}
pub use crate::library::bundle_archive::{Bundle, Member};
struct Assembly {
    files: BTreeMap<String, Vec<u8>>,
    total: u64,
    assets: BTreeSet<String>,
    links: Vec<BlobLink>,
}
impl Assembly {
    fn insert(&mut self, name: String, bytes: Vec<u8>) -> Result<()> {
        if bytes.len() as u64 > MAX_MEMBER {
            return Err(Error::Conflict("Invocation bundle member exceeds 64 MiB"));
        }
        if let Some(existing) = self.files.get(&name) {
            if existing != &bytes {
                return Err(Error::Invalid(
                    "Conflicting bytes for one invocation bundle member".into(),
                ));
            }
            return Ok(());
        }
        if self.files.len() + 2 > MAX_MEMBERS {
            return Err(Error::Conflict("Invocation bundle exceeds 64 members"));
        }
        self.total += bytes.len() as u64;
        if self.total > MAX_TOTAL {
            return Err(Error::Conflict("Invocation bundle exceeds 256 MiB"));
        }
        self.files.insert(name, bytes);
        Ok(())
    }
}
fn check_schema(kind: &str, value: &Value) -> Result<()> {
    validator(&json!({"$ref":kind}))?
        .validate(value)
        .map_err(|e| Error::Invalid(format!("Cannot bundle invalid saved evidence: {e}")))
}
fn extension(media: &str) -> &'static str {
    match media {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "application/json" => "json",
        "application/x-tar" | "application/tar" => "tar",
        _ => "bin",
    }
}
fn port_name(port: &Value, operation: bool) -> Result<&str> {
    port[if operation { "name" } else { "slot" }]
        .as_str()
        .ok_or_else(|| Error::Invalid("Saved input has no declared name".into()))
}
fn occurrence<'a>(result: &'a Value, name: &str, index: usize) -> Option<(usize, &'a Value)> {
    result["artifacts"]["in"]
        .as_array()?
        .iter()
        .enumerate()
        .filter(|(_, item)| item["port"] == name)
        .nth(index)
}
pub async fn build(assets: &AssetStore, attempt: &Attempt, options: &Options) -> Result<Bundle> {
    let _storage = assets.storage.read().await;
    if attempt.preservation["contract"] == "pending"
        || attempt.preservation["native"] == "preserving"
    {
        return Err(Error::Conflict(
            "Wait for evidence and native preservation before assembling a bundle",
        ));
    }
    let result = attempt
        .result
        .as_ref()
        .ok_or(Error::Missing("No saved result to bundle"))?;
    if result["contract"] != "0.4.0"
        || attempt.declaration["document"]["contract"] != "0.4.0"
        || result["invocation"].is_null()
    {
        return Err(Error::Invalid(
            "A bundle needs a resolved v0.4 invocation".into(),
        ));
    }
    check_schema("urn:table-manners:core:result", result)?;
    if result["action"] != attempt.request.action
        || result["invocation"]["action"] != result["action"]
        || result["tool"]["name"] != attempt.declaration["document"]["tool"]["name"]
        || result["tool"]["version"] != attempt.declaration["document"]["tool"]["version"]
    {
        return Err(Error::Invalid(
            "Saved invocation conflicts with its selected tool or action".into(),
        ));
    }
    let action = attempt.declaration["document"]["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|a| a["name"] == attempt.request.action))
        .ok_or(Error::Missing("Saved action description is unavailable"))?;
    let operation = action["kind"] == "operation";
    let ports = if operation {
        &action["ports"]["in"]
    } else {
        &action["images"]
    };
    let ports = ports
        .as_array()
        .ok_or_else(|| Error::Invalid("Saved input declarations are unavailable".into()))?;
    let names = ports
        .iter()
        .map(|port| port_name(port, operation))
        .collect::<Result<BTreeSet<_>>>()?;
    if options.omit.iter().any(|(name, why)| {
        !names.contains(name.as_str()) || why.trim().is_empty() || why.len() > 1024
    }) {
        return Err(Error::Invalid(
            "Omissions need a declared slot/port and a reason of 1–1,024 bytes".into(),
        ));
    }
    let mut assembly = Assembly {
        files: BTreeMap::new(),
        total: 0,
        assets: BTreeSet::new(),
        links: vec![],
    };
    let blobs = BlobStore(assets.clone());
    let links = blobs.links(Owner::Attempt(attempt.id.clone())).await?;
    let mut inputs = vec![];
    let mut artifacts = vec![];
    let mut unportable = vec![];
    let mut reference_only = vec![];
    let arguments = &result["invocation"]["arguments"];
    for port in ports {
        let name = port_name(port, operation)?;
        let Some(value) = arguments.get(name) else {
            if options.omit.contains_key(name) {
                return Err(Error::Invalid(format!(
                    "No supplied input to omit for {name}"
                )));
            }
            if port["required"] == true {
                return Err(Error::Invalid(format!(
                    "Resolved invocation is missing {name}"
                )));
            }
            continue;
        };
        let values = if port["many"] == true {
            value
                .as_array()
                .ok_or_else(|| Error::Invalid(format!("Resolved input {name} must be a list")))?
                .iter()
                .collect::<Vec<_>>()
        } else {
            vec![value]
        };
        if let Some(why) = options.omit.get(name) {
            if values.is_empty() {
                return Err(Error::Invalid(format!(
                    "No supplied input to omit for {name}"
                )));
            }
            if operation
                && port["type"] == "location"
                && values.iter().any(|value| value.get("resource").is_none())
            {
                return Err(Error::Invalid("Shared location values are part of the frozen invocation and cannot be omitted".into()));
            }
            if !values.is_empty() {
                reference_only.push(if operation {
                    json!({"port":name,"why":why})
                } else {
                    json!({"slot":name,"why":why})
                });
            }
            continue;
        }
        for (index, value) in values.into_iter().enumerate() {
            let is_reference = value.get("resource").is_some();
            if is_reference {
                if !operation {
                    return Err(Error::Invalid(
                        "Image-profile inputs require owned file bytes".into(),
                    ));
                }
                check_schema("urn:table-manners:operations:artifact-ref", value)?;
                if operation && value["type"] != port["type"] {
                    return Err(Error::Invalid(format!(
                        "Artifact type conflicts with port {name}"
                    )));
                }
                let saved = occurrence(result, name, index);
                if saved.is_some_and(|(_, item)| item["artifact"] != *value) {
                    return Err(Error::Invalid(format!(
                        "Artifact occurrence conflicts with {name} item {}",
                        index + 1
                    )));
                }
                let kind = attempt.declaration["document"]["artifact_types"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|kind| kind["type"] == value["type"]);
                let portable = kind
                    .is_some_and(|kind| kind["portable"] == "json" || kind["portable"] == "bundle");
                let link = saved.and_then(|(ordinal, _)| {
                    links.iter().find(|link| {
                        link.direction == "in"
                            && link.occurrence as usize == ordinal
                            && link.artifact == *value
                    })
                });
                let reason: Option<String> = if value["type"] == "image" {
                    Some("Original image bytes were not preserved for this image reference; a preview cannot replace them.".into())
                } else if !portable {
                    Some("The saved type declaration has no portable form.".into())
                } else if let Some(link) = link {
                    if u64::from(blobs.metadata(&link.sha256).await?.size) > MAX_MEMBER {
                        return Err(Error::Conflict("Invocation bundle member exceeds 64 MiB"));
                    }
                    match blobs.read_leased(&link.sha256).await {
                        Ok(bytes) => {
                            let member = format!(
                                "artifacts/{}.{}",
                                value["identity"].as_str().unwrap().replace(':', "-"),
                                extension(&link.media_type)
                            );
                            assembly.insert(member.clone(), bytes)?;
                            artifacts.push(json!({
                                "port": name, "artifact": value,
                                "member": member, "sha256": link.sha256
                            }));
                            assembly.links.push(link.clone());
                            None
                        }
                        Err(_) => Some("Owned native export bytes are missing or failed fingerprint verification.".into())
                    }
                } else {
                    Some("No owned native export is available for this input; preserve it explicitly before export.".into())
                };
                if let Some(reason) = reason {
                    unportable.push(json!({"port":name,"artifact":value,"reason":reason}));
                }
            } else if !operation || port["type"] == "image" {
                let (member, image) =
                    image_input(assets, attempt, name, index, value, &mut assembly).await?;
                inputs.push(if operation {
                    json!({"port":name,"image":image,"member":member})
                } else {
                    json!({"slot":name,"image":image,"member":member})
                });
            } else if port["type"] == "location" {
                // Shared locations passed by value are already carried in invocation.arguments.
                check_schema("urn:table-manners:images:location", value)?;
            } else {
                return Err(Error::Invalid(format!(
                    "Native input {name} must be an artifact reference"
                )));
            }
        }
    }
    let workspace_dependent = !reference_only.is_empty() || !unportable.is_empty();
    let manifest = json!({"bundle_version":1,"steps":[{"contract":result["contract"],"tool":result["tool"],"invocation":result["invocation"],"result":result,"inputs":inputs,"artifacts":artifacts,"reference_only":reference_only,"unportable":unportable}]});
    check_schema("urn:table-manners:core:invocation-bundle", &manifest)?;
    let members = assembly
        .files
        .iter()
        .map(|(name, bytes)| {
            use sha2::{Digest, Sha256};
            Member {
                name: name.clone(),
                sha256: bundle_archive::hex(&Sha256::digest(bytes)),
                bytes: bytes.len() as u64,
            }
        })
        .collect();
    let CanonicalArchive { file, sha256, size } =
        bundle_archive::write(assets.root(), &manifest, assembly.files).await?;
    Ok(Bundle {
        file,
        sha256,
        size,
        manifest,
        members,
        workspace_dependent,
        input_assets: assembly.assets.into_iter().collect(),
        native_links: assembly.links,
    })
}
async fn image_input(
    assets: &AssetStore,
    attempt: &Attempt,
    name: &str,
    index: usize,
    value: &Value,
    assembly: &mut Assembly,
) -> Result<(String, Value)> {
    if !value
        .as_object()
        .is_some_and(|object| object.len() == 1 && object.contains_key("expect"))
    {
        return Err(Error::Invalid(
            "Resolved image input contains locators or unexpected fields".into(),
        ));
    }
    let image = &value["expect"];
    check_schema("urn:table-manners:images:image-reference", image)?;
    let saved = attempt.inputs[name]
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry["argument_index"].as_u64() == Some(index as u64))
        })
        .ok_or(Error::Missing(
            "Owned input image occurrence is unavailable",
        ))?;
    if ["file_sha256", "width", "height", "projection", "topology"]
        .iter()
        .any(|key| image[key] != saved["image"][key])
    {
        return Err(Error::Invalid(
            "Saved input image frame conflicts with the invocation".into(),
        ));
    }
    let id = saved["asset"]["id"]
        .as_str()
        .ok_or(Error::Missing("Owned input image is unavailable"))?;
    if u64::from(assets.metadata(id).await?.size) > MAX_MEMBER {
        return Err(Error::Conflict("Invocation bundle image exceeds 64 MiB"));
    }
    let (asset, bytes) = assets.read(id).await?;
    if image["file_sha256"] != asset.sha256
        || image["width"] != asset.width
        || image["height"] != asset.height
    {
        return Err(Error::Invalid(
            "Owned input image bytes conflict with the invocation".into(),
        ));
    }
    let name = format!("inputs/{}.{}", asset.sha256, extension(&asset.media_type));
    assembly.insert(name.clone(), bytes)?;
    assembly.assets.insert(id.into());
    Ok((name, image.clone()))
}
