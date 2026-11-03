//! Read only the exact authorized export file; never interpret native contents.
use super::{Error, Result, client::ToolClient};
use crate::library::blobs::MAX_BLOB_BYTES;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
pub(super) async fn call(client: &ToolClient, name: &str, arguments: Value) -> Result<Value> {
    let request = serde_json::from_value(
        json!({"method":"tools/call","params":{"name":name,"arguments":arguments}}),
    )?;
    let response = client.peer()?.send_request(request).await;
    match response {
        Ok(result) => {
            let response = serde_json::to_value(result)?;
            if response["isError"] == true {
                return Err(Error::Invalid(
                    "Artifact controls must report failures before any run".into(),
                ));
            }
            Ok(response.get("structuredContent").cloned().ok_or_else(|| {
                Error::Invalid("Artifact control omitted structuredContent".into())
            })?)
        }
        Err(rmcp::ServiceError::McpError(error)) => {
            if let Some(data) = &error.data {
                if error.code != rmcp::model::ErrorCode::INVALID_PARAMS {
                    return Err(Error::Invalid(
                        "Artifact refusal must use JSON-RPC invalid params".into(),
                    ));
                }
                super::validator(&json!({"$ref":"urn:table-manners:core:refusal"}))?
                    .validate(data)
                    .map_err(|error| {
                        Error::Invalid(format!("Invalid artifact control refusal: {error}"))
                    })?;
                if data["reason"] == "binding" && name == "export" {
                    super::binding_refusal_semantics::validate(
                        data,
                        super::binding_refusal_semantics::Context::Export,
                    )?;
                    return Err(Error::BindingRefusal(data.clone()));
                }
                if data["reason"] == "invalid_arguments" {
                    return Err(Error::ArtifactControlRefusal(data.clone()));
                }
                return Err(Error::Invalid(
                    "Artifact control returned an unrelated refusal".into(),
                ));
            }
            Err(Error::Mcp(rmcp::ServiceError::McpError(error)))
        }
        Err(error) => {
            while !client.has_exited().await.unwrap_or(false) {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(error.into())
        }
    }
}
pub(super) fn read_export(
    file: &Value,
    roots: &[PathBuf],
    run_dir: Option<&Path>,
) -> Result<Vec<u8>> {
    let raw = Path::new(
        file["path"]
            .as_str()
            .ok_or_else(|| Error::Invalid("Export omitted its exact file path".into()))?,
    );
    let size = file["bytes"]
        .as_u64()
        .ok_or_else(|| Error::Invalid("Export omitted its file size".into()))?;
    if !raw.is_absolute() || size > MAX_BLOB_BYTES {
        return Err(Error::Invalid(
            "Export must name an absolute file no larger than 128 MiB".into(),
        ));
    }
    let path = raw.canonicalize()?;
    if !roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| path.starts_with(root))
    {
        return Err(Error::Invalid(
            "Export is outside registered evidence directories".into(),
        ));
    }
    if run_dir
        .and_then(|dir| dir.canonicalize().ok())
        .is_some_and(|dir| path.starts_with(dir))
    {
        return Err(Error::Invalid(
            "Artifact export must be outside the run directory".into(),
        ));
    }
    let input = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(&path)?;
    let before = input.metadata()?;
    if !before.is_file() || before.len() != size {
        return Err(Error::Invalid(
            "Export is not a regular file of the declared size".into(),
        ));
    }
    let mut bytes = Vec::new();
    input.take(size + 1).read_to_end(&mut bytes)?;
    let after = fs::metadata(&path)?;
    let stamp = |metadata: &fs::Metadata| {
        (
            metadata.dev(),
            metadata.ino(),
            metadata.len(),
            metadata.mtime(),
            metadata.mtime_nsec(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        )
    };
    let digest = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if stamp(&before) != stamp(&after) || bytes.len() as u64 != size || file["sha256"] != digest {
        return Err(Error::Invalid(
            "Export file changed or failed its declared size/fingerprint".into(),
        ));
    }
    Ok(bytes)
}
