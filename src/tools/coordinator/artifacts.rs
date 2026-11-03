use super::{Coordinator, Error, Result};
use crate::tools::{evidence::resource_content, validate_contract};
use serde_json::Value;
impl Coordinator {
    /// Resolve only an occurrence already saved in this library; callers cannot supply URIs.
    pub async fn inspect_artifact(
        &self,
        attempt: &str,
        direction: &str,
        index: usize,
    ) -> Result<Value> {
        self.inspect_artifact_at(attempt, direction, index, None)
            .await
    }
    pub async fn inspect_artifact_at(
        &self,
        attempt: &str,
        direction: &str,
        index: usize,
        target: Option<&str>,
    ) -> Result<Value> {
        if !matches!(direction, "in" | "out") {
            return Err(Error::Invalid("Unknown artifact direction".into()));
        }
        let saved = self.attempts.get(attempt).await?;
        let reference = saved
            .result
            .as_ref()
            .and_then(|result| result["artifacts"][direction].get(index))
            .map(|entry| &entry["artifact"])
            .ok_or_else(|| Error::Missing("Saved artifact occurrence is unavailable"))?;
        self.inspect_reference_at(reference, target.unwrap_or(&saved.request.tool))
            .await
    }
    pub(super) async fn inspect_reference_at(
        &self,
        reference: &Value,
        target: &str,
    ) -> Result<Value> {
        let entry = self
            .tools
            .get(target)
            .ok_or_else(|| Error::Missing("The artifact's tool is not installed"))?;
        let client = entry
            .client()
            .ok_or_else(|| Error::Missing("The artifact's tool session is unavailable"))?;
        let declaration = entry
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .declaration
            .clone()
            .ok_or_else(|| Error::Missing("The artifact's declaration is unavailable"))?;
        if reference["tool"] != declaration.document["tool"]["name"] {
            return Err(Error::Invalid(
                "This reference belongs to another tool. Choose owned image bytes or a shared location value for the receiving tool."
                    .into(),
            ));
        }
        validate_reference(reference, &declaration.document)?;
        let uri = reference["resource"].as_str().unwrap();
        let response = match client.read_resource(uri).await {
            Ok(response) => response,
            Err(Error::Mcp(rmcp::ServiceError::McpError(error))) => {
                if let Some(data) = &error.data {
                    let (_, schema) = table_manners::SCHEMAS
                        .iter()
                        .find(|(id, _)| *id == "urn:table-manners:core:refusal")
                        .unwrap();
                    let valid =
                        crate::tools::validator(&serde_json::from_str(schema)?)?.is_valid(data);
                    if valid
                        && data["reason"] == "binding"
                        && data["run"].is_null()
                        && data["bindings"].as_array().is_some_and(|items| {
                            !items.is_empty()
                                && items.iter().all(|item| {
                                    item["port"].is_null() && item["problem"] == "unresolved"
                                })
                        })
                    {
                        let remedies = data["bindings"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .filter_map(|item| item["remedy"].as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        return Err(Error::Invalid(format!(
                            "Artifact is unavailable in the current tool store. {remedies}"
                        )));
                    }
                }
                return Err(Error::Mcp(rmcp::ServiceError::McpError(error)));
            }
            Err(error) => return Err(error),
        };
        if entry
            .client()
            .is_none_or(|current| current.session != client.session)
        {
            return Err(Error::Conflict(
                "The artifact tool session changed; inspect again explicitly",
            ));
        }
        let (_, bytes) = resource_content(&response, uri, 4 * 1024 * 1024)?;
        let document: Value = serde_json::from_slice(&bytes)?;
        validate_contract("urn:table-manners:operations:artifact", &document)?;
        if document["artifact"] != *reference
            || document["tool"]["name"] != declaration.document["tool"]["name"]
            || document["tool"]["version"] != declaration.document["tool"]["version"]
        {
            return Err(Error::Invalid(
                "Artifact resource conflicts with its saved reference or tool".into(),
            ));
        }
        // Native record remains opaque. The host only validates the shared envelope.
        Ok(document)
    }
}
pub(in crate::tools) fn validate_reference(reference: &Value, description: &Value) -> Result<()> {
    validate_reference_for_input(reference, description, false)
}
fn validate_reference_for_input(
    reference: &Value,
    description: &Value,
    shared_input: bool,
) -> Result<()> {
    let (_, schema) = table_manners::SCHEMAS
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:operations:artifact-ref")
        .unwrap();
    crate::tools::validator(&serde_json::from_str(schema)?)?
        .validate(reference)
        .map_err(|error| Error::Invalid(format!("Invalid artifact reference: {error}")))?;
    let tool = reference["tool"].as_str().unwrap();
    let identity = reference["identity"].as_str().unwrap();
    if reference["resource"] != format!("{tool}://artifact/{identity}")
        || (reference["tool"] != description["tool"]["name"]
            && !(shared_input && matches!(reference["type"].as_str(), Some("image" | "location"))))
    {
        return Err(Error::Invalid(
            "Artifact resource address conflicts with its identity or registered tool".into(),
        ));
    }
    if reference["type"] != "image"
        && !description["artifact_types"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|kind| {
                kind["type"] == reference["type"]
                    && kind["schema_versions"]
                        .as_array()
                        .is_some_and(|versions| versions.contains(&reference["schema_version"]))
            })
    {
        return Err(Error::Invalid(
            "Artifact type or version is not advertised by this tool".into(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_occurrences(
    description: &Value,
    action: &str,
    result: &Value,
    bindings: &std::collections::BTreeMap<String, Vec<crate::library::attempts::InputBinding>>,
) -> Result<()> {
    let Some(action) = description["actions"]
        .as_array()
        .and_then(|actions| actions.iter().find(|item| item["name"] == action))
    else {
        return Err(Error::Invalid("Artifact action is unavailable".into()));
    };
    if action["kind"] != "operation" {
        return Ok(());
    }
    for direction in ["in", "out"] {
        let mut indices = std::collections::BTreeMap::<String, usize>::new();
        for occurrence in result["artifacts"][direction]
            .as_array()
            .into_iter()
            .flatten()
        {
            let reference = &occurrence["artifact"];
            validate_reference_for_input(reference, description, direction == "in")?;
            let port = occurrence["port"]
                .as_str()
                .ok_or_else(|| Error::Invalid("Missing artifact port".into()))?;
            let index = indices.entry(port.to_owned()).or_default();
            if direction == "in"
                && let Some(crate::library::attempts::InputBinding::Artifact {
                    reference: expected,
                }) = bindings.get(port).and_then(|values| values.get(*index))
                && expected != reference
            {
                return Err(Error::Invalid(
                    "Artifact input identity or order differs from the submitted binding".into(),
                ));
            }
            if direction == "in"
                && reference["tool"] != description["tool"]["name"]
                && !matches!(bindings.get(port).and_then(|values|values.get(*index)),Some(crate::library::attempts::InputBinding::Artifact{reference:expected}) if expected==reference)
            {
                return Err(Error::Invalid(
                    "Foreign shared artifact input has no exact submitted reference".into(),
                ));
            }
            *index += 1;

            if !action["ports"][direction]
                .as_array()
                .into_iter()
                .flatten()
                .any(|port| port["name"] == occurrence["port"] && port["type"] == reference["type"])
            {
                return Err(Error::Invalid(
                    "Artifact occurrence conflicts with its declared port".into(),
                ));
            }
        }
    }
    Ok(())
}

impl Coordinator {
    pub(super) async fn validate_lineage(
        &self,
        request: &crate::library::attempts::Submission,
    ) -> Result<()> {
        use crate::library::attempts::InputBinding;
        for (port, sources) in &request.lineage {
            let image_bindings = request.images.get(port).map(|images| {
                images
                    .iter()
                    .enumerate()
                    .map(|(index, asset)| InputBinding::Image {
                        asset: asset.clone(),
                        frame: request
                            .frames
                            .get(port)
                            .and_then(|frames| frames.get(index))
                            .cloned()
                            .unwrap_or_default(),
                    })
                    .collect::<Vec<_>>()
            });
            let bindings = request
                .bindings
                .get(port)
                .or(image_bindings.as_ref())
                .ok_or_else(|| {
                    Error::Invalid("Lineage has no receiving port or image slot".into())
                })?;
            if sources.len() != bindings.len() {
                return Err(Error::Invalid(
                    "Lineage occurrence count differs from bindings".into(),
                ));
            }
            for (binding, source) in bindings.iter().zip(sources) {
                let Some(source) = source else {
                    continue;
                };
                if let crate::library::attempts::BindingSource::Example {
                    example,
                    port: source_port,
                    occurrence,
                } = source
                {
                    let saved =
                        crate::library::examples::ExampleStore(self.assets.database.clone())
                            .get(example.clone())
                            .await?;
                    let expected = saved
                        .request
                        .bindings
                        .get(source_port)
                        .and_then(|items| items.get(*occurrence));
                    let same = match (expected, binding) {
                        (
                            Some(InputBinding::Image { asset: left, .. }),
                            InputBinding::Image { asset: right, .. },
                        ) => left == right,
                        (None, InputBinding::Image { asset, .. }) => {
                            saved
                                .request
                                .images
                                .get(source_port)
                                .and_then(|images| images.get(*occurrence))
                                == Some(asset)
                        }
                        _ => expected == Some(binding),
                    };
                    if !same {
                        return Err(Error::Invalid(
                            "Example lineage does not identify the submitted input".into(),
                        ));
                    }
                    continue;
                }
                let crate::library::attempts::BindingSource::Attempt {
                    attempt,
                    port: source_port,
                    occurrence,
                } = source
                else {
                    unreachable!()
                };
                let saved = self.attempts.get(attempt).await.map_err(|_|Error::Invalid("The lineage source attempt is unavailable. Remove this binding and select a current saved input.".into()))?;
                let result = saved
                    .result
                    .as_ref()
                    .ok_or_else(|| Error::Invalid("Lineage source has no result".into()))?;
                let output = result["artifacts"]["out"]
                    .get(*occurrence)
                    .filter(|output| output["port"] == *source_port);
                let picture = result["images"]["out"]
                    .get(*occurrence)
                    .filter(|output| output["role"] == *source_port);
                let valid = match binding {
                    InputBinding::Artifact { reference } => {
                        output.is_some_and(|output| *reference == output["artifact"])
                    }
                    InputBinding::Image { asset, .. } => {
                        let asset = self.assets.metadata(asset).await?;
                        output.is_some_and(|output| {
                            output["artifact"]["type"] == "image"
                                && output["artifact"]["identity"]
                                    == format!("sha256:{}", asset.sha256)
                        }) || picture.is_some_and(|output| {
                            output["image"]["file_sha256"] == asset.sha256
                                && output["image"]["width"] == asset.width
                                && output["image"]["height"] == asset.height
                        })
                    }
                    InputBinding::Location { value } => {
                        output.is_some_and(|output| unique_shared_location(output, result, value))
                    }
                };
                if !valid {
                    return Err(Error::Invalid(
                        "Lineage does not identify the submitted value".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn unique_shared_location(output: &Value, result: &Value, value: &Value) -> bool {
    output["artifact"]["type"] == "location"
        && result["artifacts"]["out"].as_array().is_some_and(|items| {
            items
                .iter()
                .filter(|item| item["artifact"]["type"] == "location")
                .count()
                == 1
        })
        && result["locations"]["found"]
            .as_array()
            .is_some_and(|values| values.len() == 1 && values.contains(value))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn shared_location_lineage_requires_one_unambiguous_occurrence() {
        let output = json!({"artifact":{"type":"location"}});
        let value = json!({"kind":"line"});
        let mut result = json!({"artifacts":{"out":[output]},"locations":{"found":[value]}});
        assert!(unique_shared_location(&output, &result, &value));
        result["artifacts"]["out"] = json!([output, output]);
        assert!(!unique_shared_location(&output, &result, &value));
        result["artifacts"]["out"] = json!([output]);
        result["locations"]["found"] = json!([value, value]);
        assert!(!unique_shared_location(&output, &result, &value));
    }
    #[test]
    fn foreign_shared_occurrences_require_an_exact_submitted_reference() {
        let identity = format!("sha256:{}", "a".repeat(64));
        let reference = json!({"type":"image","schema_version":1,"tool":"source","identity":identity,"frame":null,"resource":format!("source://artifact/{identity}")});
        let description = json!({"tool":{"name":"receiving"},"actions":[{"name":"mix","kind":"operation","ports":{"in":[{"name":"pictures","type":"image"}],"out":[]}}]});
        let result =
            json!({"artifacts":{"in":[{"port":"pictures","artifact":reference}],"out":[]}});
        for value in [
            json!({}),
            json!({"pictures":[{"kind":"image","asset":"owned"}]}),
        ] {
            let bindings = serde_json::from_value(value).unwrap();
            assert!(validate_occurrences(&description, "mix", &result, &bindings).is_err());
        }
        let bindings =
            serde_json::from_value(json!({"pictures":[{"kind":"artifact","reference":reference}]}))
                .unwrap();
        validate_occurrences(&description, "mix", &result, &bindings).unwrap();
    }
    #[test]
    fn shared_input_references_keep_their_owner_but_outputs_cannot_impersonate_it() {
        let description = json!({"tool":{"name":"receiving"}});
        let identity = format!("sha256:{}", "a".repeat(64));
        let reference = json!({"type":"image","schema_version":1,"tool":"source","identity":identity,"frame":null,"resource":format!("source://artifact/{identity}")});
        validate_reference_for_input(&reference, &description, true).unwrap();
        assert!(validate_reference(&reference, &description).is_err());
        let mut forged = reference.clone();
        forged["resource"] = json!(format!("receiving://artifact/{identity}"));
        assert!(validate_reference_for_input(&forged, &description, true).is_err());
    }
    #[test]
    fn artifact_addresses_types_and_versions_are_checked_without_reading_native_records() {
        let description = json!({"tool":{"name":"study"},"artifact_types":[{"type":"study:Mask","schema_versions":[1]}]});
        let identity = format!("sha256:{}", "a".repeat(64));
        let reference = json!({"type":"study:Mask","schema_version":1,"tool":"study","identity":identity,"frame":null,"resource":format!("study://artifact/{identity}")});
        validate_reference(&reference, &description).unwrap();
        for (key, value) in [
            ("resource", json!("other://artifact/forged")),
            ("identity", json!(format!("sha256:{}", "b".repeat(64)))),
            ("schema_version", json!(2)),
            ("tool", json!("other")),
        ] {
            let mut forged = reference.clone();
            forged[key] = value;
            assert!(validate_reference(&forged, &description).is_err());
        }
    }
}
