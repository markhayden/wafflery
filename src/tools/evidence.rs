use super::{Error, Result, client::ToolClient};
use crate::library::{
    assets::{AssetStore, MAX_BYTES},
    attempts::Attempt,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

pub fn check_association(attempt: &Attempt, result: &Value) -> Result<()> {
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
    for input in result["images"]["in"].as_array().into_iter().flatten() {
        let slot = input["slot"]
            .as_str()
            .ok_or_else(|| Error::Invalid("Missing input slot".into()))?;
        let expected = &attempt.inputs[slot][0]["image"];
        if !same_frame(expected, &input["image"]) {
            return Err(Error::Invalid(
                "Result input fingerprint conflicts with the submitted image".into(),
            ));
        }
    }
    Ok(())
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
            Ok(asset) => {
                outputs.push(json!({"role":output["role"],"image":output["image"],"asset":asset}))
            }
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
async fn capture_image(
    assets: &AssetStore,
    client: &ToolClient,
    output: &Value,
) -> Result<crate::library::assets::Asset> {
    let bytes = if let Some(path) = output["path"].as_str() {
        let path = Path::new(path).to_owned();
        let roots = client.descriptor.roots.clone();
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
