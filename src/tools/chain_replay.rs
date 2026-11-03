//! Fresh edge planning uses saved public evidence and owned bytes, never original run paths.
use super::{Error, Result, chain_edges, validate_contract};
use crate::library::{
    assets::AssetStore,
    attempts::{Attempt, BindingSource, ImageFrame, InputBinding},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize)]
pub struct FreshInput {
    pub name: String,
    pub index: usize,
    pub binding: InputBinding,
    pub source: BindingSource,
    pub changed: bool,
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
/// The manifest has passed the bounded archive reader. Completed attempts come from this library.
/// Nothing is imported, prepared, estimated or dispatched here.
pub async fn fresh_inputs(
    assets: &AssetStore,
    manifest: &Value,
    step: usize,
    completed: &BTreeMap<usize, Attempt>,
) -> Result<Vec<FreshInput>> {
    let consumer = manifest["steps"]
        .get(step)
        .ok_or(Error::Missing("Replay step is unavailable"))?;
    let proof_manifest = manifest.clone();
    let mut proofs = completed
        .iter()
        .map(|(step, saved)| {
            (
                *step,
                (
                    saved.execution.clone(),
                    saved.preservation["contract"].clone(),
                    saved.result.clone(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let (edges,tables)=tokio::task::spawn_blocking(move||->Result<_>{
        let edges=chain_edges::resolve(&proof_manifest)?.into_iter().filter(|e|e.consumer==step).collect::<Vec<_>>();
        let mut results=BTreeMap::new();
        for producer in edges.iter().map(|e|e.producer.step).collect::<std::collections::BTreeSet<_>>() {
            let (execution,contract,result)=proofs.remove(&producer).ok_or_else(||invalid("A required earlier replay step has no completed saved output"))?;
            let result=result.ok_or_else(||invalid("The producer has no verified result"))?;
            if execution!="completed" || result["status"]!="completed" || contract!="complete" {return Err(invalid("The producer did not finish with complete contract evidence. The consumer was not prepared."));}
            let original=&proof_manifest["steps"][producer];
            if result["tool"]["name"]!=original["tool"]["name"] || result["action"]!=original["invocation"]["action"] {return Err(invalid("The saved producer belongs to a different tool or action"));}
            validate_contract("urn:table-manners:core:result",&result)?;
            results.insert(producer,result);
        }
        let tables=chain_edges::fresh_tables(&results)?;
        Ok((edges,tables))
    }).await.map_err(|_|invalid("Fresh output review worker stopped"))??;
    let mut inputs = vec![];
    for edge in edges {
        let saved = completed
            .get(&edge.producer.step)
            .ok_or_else(|| invalid("A completed source is unavailable"))?;
        let output = tables
            .get(&edge.producer.step)
            .ok_or_else(|| invalid("Fresh source output review is unavailable"))?
            .locate(&edge.producer)?;
        let argument = &consumer["invocation"]["arguments"][&edge.name];
        let old = if let Some(items) = argument.as_array() {
            items
                .get(edge.index)
                .ok_or_else(|| invalid("Replay input occurrence is unavailable"))?
        } else {
            argument
        };
        let binding = if output.image.is_some()
            || output
                .reference
                .as_ref()
                .is_some_and(|r| r["type"] == "image")
        {
            let picture = output.image.as_ref().ok_or_else(|| {
                invalid("The fresh image output has no verifiable image coordinates or bytes")
            })?;
            let owned=saved.outputs.iter().find(|image| {
                image["asset"]["sha256"]==picture["file_sha256"] &&
                chain_edges::same_image(&image["image"],picture)
            }).ok_or_else(||invalid("The fresh image output has no preserved owned image. The consumer was not prepared; no preview was substituted."))?;
            let id = owned["asset"]["id"]
                .as_str()
                .ok_or_else(|| invalid("Fresh image ownership is unavailable"))?;
            let (asset, _) = assets.read(id).await?;
            if asset.sha256 != picture["file_sha256"]
                || picture["width"].as_f64() != Some(f64::from(asset.width))
                || picture["height"].as_f64() != Some(f64::from(asset.height))
            {
                return Err(invalid(
                    "Fresh owned image bytes conflict with the producer's result",
                ));
            }
            let frame: ImageFrame = serde_json::from_value(
                json!({"projection":picture["projection"],"topology":picture["topology"]}),
            )?;
            InputBinding::Image {
                asset: asset.id,
                frame,
            }
        } else {
            let reference = output
                .reference
                .clone()
                .ok_or_else(|| invalid("The fresh output has no bindable artifact reference"))?;
            if reference["tool"] != consumer["tool"]["name"] {
                return Err(invalid(
                    "A native reference cannot cross tools. Choose owned image bytes or an explicit shared location value.",
                ));
            }
            InputBinding::Artifact { reference }
        };
        let changed = match &binding {
            InputBinding::Artifact { reference } => reference["identity"] != old["identity"],
            InputBinding::Image { asset, .. } => {
                let image = assets.metadata(asset).await?;
                let hash = old["expect"]["file_sha256"]
                    .as_str()
                    .map(str::to_owned)
                    .or_else(|| {
                        old["identity"]
                            .as_str()
                            .and_then(|id| id.strip_prefix("sha256:"))
                            .map(str::to_owned)
                    });
                hash.as_deref() != Some(&image.sha256)
            }
            InputBinding::Location { .. } => false,
        };
        let alias = output
            .producer
            .aliases
            .first()
            .ok_or_else(|| invalid("Fresh output has no public occurrence"))?;
        inputs.push(FreshInput {
            name: edge.name,
            index: edge.index,
            binding,
            source: BindingSource::Attempt {
                attempt: saved.id.clone(),
                port: alias.port.clone(),
                occurrence: alias.index,
            },
            changed,
        });
    }
    Ok(inputs)
}

#[derive(Clone, Debug, Serialize)]
pub struct ReplayRun {
    pub step: usize,
    pub attempt: String,
    pub outcome: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct ReplayStatus {
    pub settling: bool,
    pub id: String,
    pub bundle: String,
    pub tools: Vec<String>,
    pub step: usize,
    pub total: usize,
    pub generation: u64,
    pub phase: String,
    pub attempts: Vec<ReplayRun>,
    pub import: Option<super::bundle_import::Progress>,
    pub error: Option<String>,
}
