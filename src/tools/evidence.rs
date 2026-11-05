use super::{Error, Result, client::ToolClient};
use crate::library::{
    assets::{AssetStore, MAX_BYTES},
    attempts::{Attempt, InputBinding},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

pub fn check_association(attempt: &Attempt, result: &Value) -> Result<Vec<String>> {
    let tool = &attempt.declaration["document"]["tool"];
    if result["tool"]["name"] != tool["name"]
        || result["tool"]["version"] != tool["version"]
        || result["action"] != attempt.request.action
    {
        return Err(Error::Invalid(
            "Result tool or action conflicts with this attempt".into(),
        ));
    }
    let name = tool["name"]
        .as_str()
        .ok_or_else(|| Error::Invalid("Missing tool name".into()))?;
    let id = result["run"]["id"]
        .as_str()
        .ok_or_else(|| Error::Invalid("Missing run identity".into()))?;
    if result["run"]["resource"] != format!("{name}://run/{id}/run.json") {
        return Err(Error::Invalid(
            "Result run identity conflicts with its resource address".into(),
        ));
    }
    super::association::check_reference(attempt, &result["run"])?;
    let images_profile = attempt.declaration["document"]["profiles"]
        .as_array()
        .is_some_and(|profiles| profiles.iter().any(|profile| profile == "images"));
    super::invocation_semantics::validate(result, images_profile)?;
    if images_profile {
        for (container, fields) in [("images", ["in", "out"]), ("locations", ["used", "found"])] {
            if fields
                .iter()
                .any(|field| !result[container][field].is_array())
            {
                return Err(Error::Invalid(format!(
                    "Images-profile result is missing {container} containers"
                )));
            }
        }
    }
    let action = attempt.declaration["document"]["actions"]
        .as_array()
        .and_then(|actions| {
            actions
                .iter()
                .find(|action| action["name"] == attempt.request.action)
        });
    super::coordinator::artifacts::validate_occurrences(
        &attempt.declaration["document"],
        &attempt.request.action,
        result,
        &attempt.request.bindings,
    )?;
    let operation = action.is_some_and(|action| action["kind"] == "operation");
    let mut notices = Vec::new();
    for direction in ["in", "out"] {
        for occurrence in result["artifacts"][direction]
            .as_array()
            .into_iter()
            .flatten()
        {
            let preview = &occurrence["preview"];
            if !preview.is_null()
                && !result["images"]["out"]
                    .as_array()
                    .is_some_and(|outputs| outputs.iter().any(|output| same_image(preview, output)))
            {
                notices.push("An artifact preview was not reported among the output images; preview preservation is incomplete.".into());
            }
        }
    }

    let mut expected_inputs = attempt.inputs.clone();
    if let Some(action) = action.filter(|_| operation) {
        for port in action["ports"]["in"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|port| port["type"] == "image")
        {
            let name = port["name"].as_str().unwrap();
            let mut expected = Vec::new();
            for (index, binding) in attempt
                .request
                .bindings
                .get(name)
                .into_iter()
                .flatten()
                .enumerate()
            {
                match binding {
                    InputBinding::Image { .. } => {
                        let image = attempt.inputs[name]
                            .as_array()
                            .and_then(|entries| {
                                entries.iter().find(|entry| {
                                    entry["argument_index"].as_u64() == Some(index as u64)
                                })
                            })
                            .ok_or_else(|| {
                                Error::Invalid("Missing owned operation image evidence".into())
                            })?;
                        expected.push(image.clone());
                    }
                    InputBinding::Artifact { reference } => {
                        notices.push(format!("Input image {name}[{index}] was passed by reference; its image bytes were not preserved by this library."));
                        expected.push(json!({"artifact":reference}));
                    }
                    _ => return Err(Error::Invalid("Invalid operation image binding".into())),
                }
            }
            if !expected.is_empty() {
                expected_inputs[name] = json!(expected);
            }
        }
    }
    let mut positions = std::collections::BTreeMap::<&str, usize>::new();
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    let completed = result["status"] == "completed";
    for input in result["images"]["in"].as_array().into_iter().flatten() {
        let slot = input["slot"]
            .as_str()
            .ok_or_else(|| Error::Invalid("Missing input slot".into()))?;
        let expected = expected_inputs[slot]
            .as_array()
            .ok_or_else(|| Error::Invalid("Result reports an unsubmitted image slot".into()))?;
        let position = positions.entry(slot).or_default();
        let matches = |entry: &Value| {
            if let Some(identity) = entry["artifact"]["identity"].as_str() {
                // Image identities are their file hash; `on` can name a different root frame.
                entry["artifact"]["type"] == "image"
                    && identity.strip_prefix("sha256:") == input["image"]["file_sha256"].as_str()
            } else {
                (operation || entry["role"] == input["role"])
                    && same_frame(&entry["image"], &input["image"])
            }
        };
        let offset = if completed {
            expected
                .get(*position)
                .filter(|entry| matches(entry))
                .map(|_| 0)
        } else {
            expected[*position..].iter().position(matches)
        }
        .ok_or_else(|| {
            Error::Invalid(
                "Result input role, fingerprint, frame or order conflicts with submitted images"
                    .into(),
            )
        })?;
        *position += offset + 1;
        *counts.entry(slot).or_default() += 1;
    }
    for (slot, expected) in expected_inputs
        .as_object()
        .into_iter()
        .flat_map(|inputs| inputs.iter())
    {
        let total = expected.as_array().map_or(0, Vec::len);
        let reported = counts.get(slot.as_str()).copied().unwrap_or(0);
        if reported < total {
            if completed {
                return Err(Error::Invalid(
                    "Completed result omits submitted image evidence".into(),
                ));
            }
            notices.push(format!("Result reports {reported} of {total} submitted images for slot {slot}; input evidence is incomplete."));
        }
    }
    Ok(notices)
}

fn same_frame(a: &Value, b: &Value) -> bool {
    ["file_sha256", "width", "height", "projection", "topology"]
        .iter()
        .all(|key| !a[key].is_null() && a[key] == b[key])
}
pub(super) fn resource_content(
    response: &Value,
    uri: &str,
    max: usize,
) -> Result<(Value, Vec<u8>)> {
    let contents = response["contents"]
        .as_array()
        .ok_or_else(|| Error::Invalid("Missing resource contents".into()))?;
    if contents.len() != 1 || contents[0]["uri"] != uri {
        return Err(Error::Invalid(
            "Resource response has a conflicting address or multiple contents".into(),
        ));
    }
    let item = &contents[0];
    let bytes = if let Some(text) = item["text"].as_str() {
        text.as_bytes().to_vec()
    } else if let Some(blob) = item["blob"].as_str() {
        STANDARD
            .decode(blob)
            .map_err(|_| Error::Invalid("Invalid resource base64".into()))?
    } else {
        return Err(Error::Invalid("Resource has no data".into()));
    };
    if bytes.len() > max {
        return Err(Error::Invalid(
            "Resource exceeds the preservation limit".into(),
        ));
    }
    Ok((item.clone(), bytes))
}
pub async fn capture(
    assets: &AssetStore,
    client: &ToolClient,
    attempt: &Attempt,
) -> (Vec<Value>, Option<Value>, Vec<String>) {
    let Some(result) = &attempt.result else {
        return (vec![], None, vec!["No terminal result to preserve".into()]);
    };
    let mut outputs = vec![];
    let mut notices = vec![];
    for output in result["images"]["out"].as_array().into_iter().flatten() {
        let saved = capture_image(assets, client, output).await;
        match saved {
            Ok(asset) => outputs.push(saved_output(output, asset)),
            Err(error) => notices.push(format!("Output could not be preserved: {error}")),
        }
    }
    let uri = format!(
        "{}://run/{}/journal",
        result["tool"]["name"].as_str().unwrap_or(""),
        result["run"]["id"].as_str().unwrap_or("")
    );
    let journal = match client
        .read_resource(&uri)
        .await
        .and_then(|response| resource_content(&response, &uri, 16 * 1024 * 1024))
    {
        Ok((content, _)) => Some(content),
        Err(error) => {
            notices.push(format!("Journal could not be preserved: {error}"));
            None
        }
    };
    let mut frames: Vec<&Value> = attempt
        .inputs
        .as_object()
        .into_iter()
        .flat_map(|slots| slots.values())
        .flat_map(|values| values.as_array().into_iter().flatten())
        .map(|value| &value["image"])
        .collect();
    frames.extend(outputs.iter().map(|value| &value["image"]));
    for location in ["used", "found"]
        .iter()
        .flat_map(|key| result["locations"][key].as_array().into_iter().flatten())
    {
        for reference in [&location["image"], &location["mask"]["image"]] {
            if !reference.is_null() && !frames.iter().any(|frame| same_frame(frame, reference)) {
                notices.push("A location references image bytes that were not preserved.".into());
            }
        }
    }
    (outputs, journal, notices)
}
/// A preserved output: its role, its reference, the saved asset and, when the tool gave one, the
/// label a person sees in place of the role (table-manners 10.2).
fn saved_output(output: &Value, asset: crate::library::assets::Asset) -> Value {
    let mut saved = json!({"role":output["role"],"image":output["image"],"asset":asset});
    if let Some(label) = output["label"].as_str() {
        saved["label"] = json!(label);
    }
    saved
}
async fn capture_image(
    assets: &AssetStore,
    client: &ToolClient,
    output: &Value,
) -> Result<crate::library::assets::Asset> {
    let bytes = if let Some(path) = output["path"].as_str() {
        let path = Path::new(path).to_owned();
        let roots = client.descriptor.evidence_dirs.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let path = path.canonicalize()?;
            if !roots.iter().any(|root| path.starts_with(root)) {
                return Err(Error::Invalid(
                    "Output path is outside registered folders".into(),
                ));
            }
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
                .open(path)?;
            if !file.metadata()?.is_file() {
                return Err(Error::Invalid("Output is not a regular file".into()));
            }
            let mut bytes = vec![];
            file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
            Ok(bytes)
        })
        .await
        .map_err(|_| Error::Invalid("Image capture worker stopped".into()))??
    } else if let Some(data) = output["inline"]["data"].as_str() {
        STANDARD
            .decode(data)
            .map_err(|_| Error::Invalid("Invalid inline image".into()))?
    } else if let Some(uri) = output["resource"].as_str() {
        resource_content(&client.read_resource(uri).await?, uri, MAX_BYTES as usize)?.1
    } else {
        return Err(Error::Invalid("Output has no locator".into()));
    };
    if bytes.len() as u64 > MAX_BYTES {
        return Err(Error::Invalid("Output exceeds 128 MiB".into()));
    }
    let hash: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if output["image"]["file_sha256"] != hash {
        return Err(Error::Invalid(
            "Output fingerprint does not match its bytes".into(),
        ));
    }
    let directory = assets.root().join("staging");
    let staged =
        tokio::task::spawn_blocking(move || -> std::io::Result<tempfile::NamedTempFile> {
            let mut staged = tempfile::NamedTempFile::new_in(directory)?;
            staged.write_all(&bytes)?;
            staged.as_file().sync_all()?;
            Ok(staged)
        })
        .await
        .map_err(|_| Error::Invalid("Image staging worker stopped".into()))??;
    let asset = assets.import_path(staged.path().to_owned()).await?;
    if output["image"]["width"] != asset.width || output["image"]["height"] != asset.height {
        return Err(Error::Invalid(
            "Output dimensions do not match its bytes".into(),
        ));
    }
    Ok(asset)
}

/// Whether an artifact's preview is this image out: the same image at the same place (its
/// reference and its locator). Role and label are how an out is named and shown, so a preview
/// listed under a try's own role (`preview_try_2`) is still the one image (table-manners 4.7).
fn same_image(preview: &Value, output: &Value) -> bool {
    preview["image"] == output["image"]
        && ["path", "inline", "resource"]
            .iter()
            .any(|locator| !preview[*locator].is_null() && preview[*locator] == output[*locator])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_preview_is_its_image_out_whatever_its_role_or_label() {
        let image = json!({"file_sha256":"a".repeat(64),"width":8,"height":4,"projection":"flat","topology":"bounded"});
        let preview = json!({"role":"preview","label":"Try 1 · preview","image":image,"path":"/run/tries/1/preview-mask.png"});
        // Listed under the try's own role: the same image at the same place.
        assert!(same_image(
            &preview,
            &json!({"role":"preview_try_1","label":"Try 1 · preview","image":image,"path":"/run/tries/1/preview-mask.png"})
        ));
        assert!(same_image(
            &preview,
            &json!({"role":"preview","image":image,"path":"/run/tries/1/preview-mask.png"})
        ));
        // Another place, or another image, is not it.
        assert!(!same_image(
            &preview,
            &json!({"role":"preview","image":image,"path":"/run/preview-mask.png"})
        ));
        assert!(!same_image(
            &preview,
            &json!({"role":"preview","image":{"file_sha256":"b".repeat(64)},"path":"/run/tries/1/preview-mask.png"})
        ));
        assert!(
            !same_image(
                &json!({"role":"preview","image":image}),
                &json!({"role":"preview","image":image})
            ),
            "no locator to match"
        );
    }
    fn frame(name: &str) -> Value {
        json!({"file_sha256":name.repeat(64),"width":8,"height":4,"projection":"flat","topology":"bounded"})
    }
    fn fixture() -> (Attempt, Value) {
        let inputs = json!({"source":[{"role":"source","image":frame("a")}], "references":[
            {"role":"reference","image":frame("b")}, {"role":"reference","image":frame("a")}, {"role":"reference","image":frame("b")}
        ]});
        let attempt = serde_json::from_value(json!({
            "id":"request", "sequence":1, "created_ms":0,
            "request":{"id":"request","tool":"fixture","action":"mark","revision":"r","params":{},"images":{}},
            "arguments":{},"declaration":{"document":{"tool":{"name":"fixture","version":"1"},"profiles":["images"],"actions":[{"name":"mark","kind":"task"}]},"tool_session":{"id":"session"}},"inputs":inputs,"wire":null,
            "execution":"running","result":null,"preservation":{},"outputs":[],"journal":null,"notices":[]
        })).unwrap();
        let result = json!({"tool":{"name":"fixture","version":"1"},"action":"mark","run":{"id":"run","resource":"fixture://run/run/run.json","session_id":"session","client_run_id":"request"},"status":"completed","images":{"in":[
            {"slot":"references","role":"reference","image":frame("b")},
            {"slot":"source","role":"source","image":frame("a")},
            {"slot":"references","role":"reference","image":frame("a")},
            {"slot":"references","role":"reference","image":frame("b")}
        ],"out":[]},"locations":{"used":[],"found":[]}});
        (attempt, result)
    }

    #[test]
    fn complete_lists_match_per_slot_in_order_including_duplicates() {
        let (attempt, result) = fixture();
        assert!(check_association(&attempt, &result).is_ok());
        for case in [
            "order",
            "role",
            "missing",
            "extra",
            "unknown",
            "frame",
            "fingerprint",
        ] {
            let mut changed = result.clone();
            let inputs = changed["images"]["in"].as_array_mut().unwrap();
            match case {
                "order" => inputs.swap(0, 2),
                "role" => inputs[0]["role"] = json!("source"),
                "missing" => {
                    inputs.pop();
                }
                "extra" => inputs.push(inputs[0].clone()),
                "unknown" => inputs[0]["slot"] = json!("other"),
                "frame" => inputs[0]["image"]["topology"] = json!("wrap"),
                _ => inputs[0]["image"]["file_sha256"] = json!("c".repeat(64)),
            }
            assert!(check_association(&attempt, &changed).is_err(), "{case}");
        }
    }

    #[test]
    fn noncompleted_lists_accept_only_ordered_subsequences_with_duplicate_counts() {
        let (attempt, result) = fixture();
        for status in ["error", "invalid", "cancelled"] {
            for selected in [vec![], vec![2], vec![0, 3], vec![0, 2, 3]] {
                let mut partial = result.clone();
                partial["status"] = json!(status);
                partial["images"]["in"] = json!(
                    selected
                        .iter()
                        .map(|index| result["images"]["in"][index].clone())
                        .collect::<Vec<_>>()
                );
                let notices = check_association(&attempt, &partial).unwrap();
                assert_eq!(notices.len(), if selected.len() == 3 { 1 } else { 2 });
                assert!(notices.iter().all(|notice| notice.contains("incomplete")));
            }
            for selected in [vec![2, 0, 3], vec![0, 0, 0]] {
                let mut wrong = result.clone();
                wrong["status"] = json!(status);
                wrong["images"]["in"] = json!(
                    selected
                        .iter()
                        .map(|index| result["images"]["in"][index].clone())
                        .collect::<Vec<_>>()
                );
                assert!(
                    check_association(&attempt, &wrong).is_err(),
                    "{status} {selected:?}"
                );
            }
            let mut wrong = result.clone();
            wrong["status"] = json!(status);
            wrong["images"]["in"][0]["role"] = json!("source");
            assert!(check_association(&attempt, &wrong).is_err());
        }
    }

    #[test]
    fn a_saved_output_keeps_the_tools_label_and_omits_an_absent_one() {
        let asset = crate::library::assets::Asset {
            id: "asset".into(),
            sha256: "a".repeat(64),
            size: 1,
            width: 8,
            height: 4,
            media_type: "image/png".into(),
        };
        let labelled = json!({"role":"result_try_2","label":"Try 2 · result","image":frame("a"),"path":"/run/2.png"});
        let saved = saved_output(&labelled, asset.clone());
        assert_eq!(saved["label"], "Try 2 · result");
        assert_eq!(saved["role"], "result_try_2");
        assert!(saved.get("path").is_none());
        let plain = saved_output(&json!({"role":"result","image":frame("a")}), asset);
        assert!(plain.get("label").is_none());
    }

    #[test]
    fn image_profile_requires_containers_even_without_completed_inputs() {
        let (attempt, result) = fixture();
        for field in ["images", "locations"] {
            let mut missing = result.clone();
            missing["status"] = json!("cancelled");
            missing.as_object_mut().unwrap().remove(field);
            assert!(check_association(&attempt, &missing).is_err(), "{field}");
        }
    }
}
