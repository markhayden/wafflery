//! Derive ordered dataflow only from public manifest inputs and output evidence.
//! Callers validate archive/schema/claims first; this module never reads files or native records.
use super::{Error, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Alias {
    pub profile: String,
    pub port: String,
    pub index: usize,
    pub item: usize,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Producer {
    pub step: usize,
    pub port: String,
    pub item: usize,
    pub aliases: Vec<Alias>,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Edge {
    pub consumer: usize,
    pub name: String,
    pub index: usize,
    pub producer: Producer,
}
struct Output {
    tool: String,
    producer: Producer,
    reference: Option<Value>,
    image: Option<Value>,
    preview: Option<Value>,
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
fn array<'a>(value: &'a Value, field: &str) -> impl Iterator<Item = &'a Value> {
    value[field].as_array().into_iter().flatten()
}
fn artifact_image(row: &Value) -> Option<Value> {
    let r = &row["artifact"];
    if r["type"] != "image" {
        return None;
    }
    let hash = r["identity"].as_str()?.strip_prefix("sha256:")?;
    for image in [&row["preview"]["image"], &r["on"]] {
        if image["file_sha256"] == hash {
            return Some(image.clone());
        }
    }
    None
}
fn dimension(value: &Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 1.0 && *n <= u32::MAX as f64 && n.fract() == 0.0)
            .map(|n| n as u64)
    })
}
pub(crate) fn same_image(left: &Value, right: &Value) -> bool {
    left["file_sha256"].is_string()
        && ["file_sha256", "projection", "topology"]
            .iter()
            .all(|field| left[*field] == right[*field])
        && ["width", "height"].iter().all(|field| {
            dimension(&left[*field])
                .zip(dimension(&right[*field]))
                .is_some_and(|(a, b)| a == b)
        })
}
fn same_picture(left: &Value, right: &Value) -> bool {
    left["role"] == right["role"]
        && same_image(&left["image"], &right["image"])
        && ["path", "inline", "resource"]
            .iter()
            .all(|field| left.get(*field) == right.get(*field))
}
struct SearchBudget(usize);
impl SearchBudget {
    fn scan(&mut self, candidates: usize) -> Result<()> {
        self.0 = self.0.checked_sub(candidates).ok_or_else(|| invalid("Chain connection search exceeds this host's limit. Import individual steps or export a single-step bundle instead."))?;
        Ok(())
    }
}
fn outputs(step: usize, result: &Value, budget: &mut SearchBudget) -> Result<Vec<Output>> {
    let mut outputs = vec![];
    let mut counts = BTreeMap::<String, usize>::new();
    for (index, row) in array(&result["artifacts"], "out").enumerate() {
        let port = row["port"]
            .as_str()
            .ok_or_else(|| invalid("Output artifact has no port"))?
            .to_owned();
        let item = counts.entry(port.clone()).or_default();
        let alias = Alias {
            profile: "artifacts".into(),
            port: port.clone(),
            index,
            item: *item,
        };
        *item += 1;
        outputs.push(Output {
            tool: result["tool"]["name"].as_str().unwrap_or("").into(),
            producer: Producer {
                step,
                port,
                item: alias.item,
                aliases: vec![alias],
            },
            reference: Some(row["artifact"].clone()),
            image: artifact_image(row),
            preview: row.get("preview").filter(|v| !v.is_null()).cloned(),
        });
    }
    counts.clear();
    for (index, row) in array(&result["images"], "out").enumerate() {
        let port = row["role"]
            .as_str()
            .ok_or_else(|| invalid("Output image has no role"))?
            .to_owned();
        let item = counts.entry(port.clone()).or_default();
        let alias = Alias {
            profile: "images".into(),
            port: port.clone(),
            index,
            item: *item,
        };
        *item += 1;
        budget.scan(outputs.len())?;
        let mirrored: Vec<_> = outputs
            .iter()
            .enumerate()
            .filter(|(_, out)| {
                out.reference.as_ref().is_some_and(|r| r["type"] == "image")
                    && out
                        .image
                        .as_ref()
                        .is_some_and(|image| same_image(image, &row["image"]))
                    && out
                        .preview
                        .as_ref()
                        .is_some_and(|preview| same_picture(preview, row))
            })
            .map(|(index, _)| index)
            .take(2)
            .collect();
        if mirrored.len() == 1
            && !outputs[mirrored[0]]
                .producer
                .aliases
                .iter()
                .any(|a| a.profile == "images")
        {
            outputs[mirrored[0]].producer.aliases.push(alias);
        } else {
            outputs.push(Output {
                tool: result["tool"]["name"].as_str().unwrap_or("").into(),
                producer: Producer {
                    step,
                    port,
                    item: alias.item,
                    aliases: vec![alias],
                },
                reference: None,
                image: Some(row["image"].clone()),
                preview: None,
            });
        }
    }
    Ok(outputs)
}
fn names(step: &Value) -> BTreeSet<&str> {
    let mut names = BTreeSet::new();
    for field in ["inputs", "artifacts", "unportable", "reference_only"] {
        for input in array(step, field) {
            if let Some(name) = input["port"].as_str().or_else(|| input["slot"].as_str()) {
                names.insert(name);
            }
        }
    }
    for (profile, key) in [("artifacts", "port"), ("images", "slot")] {
        for input in array(&step["result"][profile], "in") {
            if let Some(name) = input[key].as_str() {
                names.insert(name);
            }
        }
    }
    names
}
fn matches_input(out: &Output, reference: Option<&Value>, image: Option<&Value>) -> bool {
    if let Some(reference) = reference {
        if let Some(saved) = &out.reference {
            // `on` is an optional root hint, not a distinct native artifact identity.
            return [
                "type",
                "schema_version",
                "tool",
                "identity",
                "frame",
                "resource",
            ]
            .iter()
            .all(|field| saved[*field] == reference[*field])
                && match (saved.get("on"), reference.get("on")) {
                    (Some(a), Some(b)) => same_image(a, b),
                    _ => true,
                };
        }
        // The image profile has no native frame. Match bytes/owner and public coordinates
        // when `on` names those same bytes; never interpret an opaque native grid string.
        return reference["type"] == "image"
            && reference["tool"] == out.tool
            && out.image.as_ref().is_some_and(|image| {
                reference["identity"]
                    == format!("sha256:{}", image["file_sha256"].as_str().unwrap_or(""))
                    && (reference["on"]["file_sha256"] != image["file_sha256"]
                        || same_image(&reference["on"], image))
            });
    }
    image.is_some_and(|image| match &out.image {
        Some(saved) => same_image(saved, image),
        None => out.reference.as_ref().is_some_and(|reference| {
            reference["type"] == "image"
                && reference["identity"]
                    == format!("sha256:{}", image["file_sha256"].as_str().unwrap_or(""))
        }),
    })
}
/// Zero matching earlier outputs is an external input; multiple matches cannot be replayed safely.
pub fn resolve(manifest: &Value) -> Result<Vec<Edge>> {
    let steps = manifest["steps"]
        .as_array()
        .ok_or_else(|| invalid("A chain needs ordered steps"))?;
    let mut earlier = Vec::<Output>::new();
    let mut budget = SearchBudget(1_000_000);
    let mut edges = vec![];
    for (consumer, step) in steps.iter().enumerate() {
        let arguments = step["invocation"]["arguments"]
            .as_object()
            .ok_or_else(|| invalid("Chain step has no argument object"))?;
        for name in names(step) {
            let value = arguments
                .get(name)
                .ok_or_else(|| invalid("Chain input is absent from its invocation"))?;
            let values: Vec<_> = value
                .as_array()
                .map(|v| v.iter().collect())
                .unwrap_or_else(|| vec![value]);
            for (index, input) in values.into_iter().enumerate() {
                let reference = input
                    .get("resource")
                    .filter(|v| v.is_string())
                    .map(|_| input);
                let image = input.get("expect");
                if reference.is_none() && image.is_none() {
                    continue;
                }
                budget.scan(earlier.len())?;
                let matches: Vec<_> = earlier
                    .iter()
                    .filter(|out| matches_input(out, reference, image))
                    .take(2)
                    .collect();
                match matches.as_slice() {
                    [] => (),
                    [output] => edges.push(Edge {
                        consumer,
                        name: name.into(),
                        index,
                        producer: output.producer.clone(),
                    }),
                    _ => {
                        return Err(invalid(format!(
                            "Chain input {} item {} in step {} is ambiguous across earlier outputs. Export a single-step bundle or choose distinct producer outputs.",
                            name,
                            index + 1,
                            consumer + 1
                        )));
                    }
                }
            }
        }
        earlier.extend(outputs(consumer, &step["result"], &mut budget)?);
    }
    Ok(edges)
}

pub const MAX_STEPS: usize = 16;
/// Export has host provenance as well as public evidence. Imported manifests deliberately use
/// `resolve` alone, because host-private lineage is not part of the portable contract.
pub fn check_selection(
    manifest: &Value,
    attempts: &[crate::library::attempts::Attempt],
) -> Result<Vec<Edge>> {
    use crate::library::attempts::BindingSource;
    let steps = manifest["steps"]
        .as_array()
        .ok_or_else(|| invalid("A chain needs ordered steps"))?;
    if attempts.is_empty() || attempts.len() > MAX_STEPS || steps.len() != attempts.len() {
        return Err(invalid("Select between 1 and 16 matching saved steps"));
    }
    let selected: BTreeMap<_, _> = attempts
        .iter()
        .enumerate()
        .map(|(i, a)| (a.id.as_str(), i))
        .collect();
    if selected.len() != attempts.len() {
        return Err(invalid("A selected chain cannot repeat a saved attempt"));
    }
    for (attempt, step) in attempts.iter().zip(steps) {
        if attempt.result.as_ref() != Some(&step["result"]) {
            return Err(invalid(
                "Selected chain order conflicts with its immutable results",
            ));
        }
    }
    let edges = resolve(manifest)?;
    {
        let edge_map: BTreeMap<_, _> = edges
            .iter()
            .map(|edge| {
                (
                    (edge.consumer, edge.name.as_str(), edge.index),
                    &edge.producer,
                )
            })
            .collect();
        for (consumer, (attempt, step)) in attempts.iter().zip(steps).enumerate() {
            for name in names(step) {
                let value = &step["invocation"]["arguments"][name];
                let values: Vec<_> = value
                    .as_array()
                    .map(|v| v.iter().collect())
                    .unwrap_or_else(|| vec![value]);
                for (index, value) in values.into_iter().enumerate() {
                    if value.get("resource").is_none() && value.get("expect").is_none() {
                        continue;
                    }
                    let source = attempt
                        .request
                        .lineage
                        .get(name)
                        .and_then(|entries| entries.get(index))
                        .and_then(Option::as_ref);
                    match (edge_map.get(&(consumer, name, index)), source) {
                        (
                            Some(producer),
                            Some(BindingSource::Attempt {
                                attempt,
                                port,
                                occurrence,
                            }),
                        ) if attempts[producer.step].id == *attempt
                            && producer
                                .aliases
                                .iter()
                                .any(|alias| alias.port == *port && alias.index == *occurrence) => {
                        }
                        (Some(_), _) => {
                            return Err(invalid(format!(
                                "Step {} input {} item {} matches a selected output but its recorded source differs or is missing. Export a single-step bundle instead of guessing a producer.",
                                consumer + 1,
                                name,
                                index + 1
                            )));
                        }
                        (None, Some(BindingSource::Attempt { attempt, .. }))
                            if selected.contains_key(attempt.as_str()) =>
                        {
                            return Err(invalid(format!(
                                "Step {} input {} item {} has selected-source lineage that cannot be recovered in this order from public output references. Reorder the selection or export a single-step bundle.",
                                consumer + 1,
                                name,
                                index + 1
                            )));
                        }
                        _ => (),
                    }
                }
            }
        }
    }
    Ok(edges)
}

pub(crate) struct FreshOutput {
    pub producer: Producer,
    pub reference: Option<Value>,
    pub image: Option<Value>,
}
pub(crate) struct FreshOutputs {
    items: Vec<FreshOutput>,
    aliases: BTreeMap<(String, String, usize), usize>,
}
impl FreshOutputs {
    /// Port/item is stable across reordering; a formerly mirrored pair must still be one output.
    pub fn locate(&self, producer: &Producer) -> Result<&FreshOutput> {
        let matches = producer
            .aliases
            .iter()
            .filter_map(|alias| {
                self.aliases
                    .get(&(alias.profile.clone(), alias.port.clone(), alias.item))
                    .copied()
            })
            .collect::<BTreeSet<_>>();
        if matches.len() > 1 {
            return Err(invalid(
                "The fresh producer's formerly mirrored outputs now differ. The consumer was not prepared.",
            ));
        }
        let index=matches.first().ok_or_else(||invalid("The fresh producer is missing the required output port/item. The consumer was not prepared."))?;
        Ok(&self.items[*index])
    }
}
/// Normalize each producer once, sharing the total work budget across the whole staged step.
pub(crate) fn fresh_tables(
    results: &BTreeMap<usize, Value>,
) -> Result<BTreeMap<usize, FreshOutputs>> {
    let mut budget = SearchBudget(1_000_000);
    let mut tables = BTreeMap::new();
    for (step, result) in results {
        let mut items = vec![];
        let mut aliases = BTreeMap::new();
        for output in outputs(*step, result, &mut budget)? {
            for alias in &output.producer.aliases {
                aliases.insert(
                    (alias.profile.clone(), alias.port.clone(), alias.item),
                    items.len(),
                );
            }
            items.push(FreshOutput {
                producer: output.producer,
                reference: output.reference,
                image: output.image,
            });
        }
        tables.insert(*step, FreshOutputs { items, aliases });
    }
    Ok(tables)
}
