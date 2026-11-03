//! Evidence only: explicit bytes and fixed entry names, never a walk of tool storage.
use super::{Error, Result, assets::AssetStore, attempts::Attempt};
use flate2::{Compression, write::GzEncoder};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, io::Write};

const MAX_TOTAL: usize = 256 * 1024 * 1024;
const MAX_METADATA: usize = 32 * 1024 * 1024;
pub struct EvidenceArchive {
    pub file: tempfile::NamedTempFile,
    pub size: u64,
}
fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub async fn archive(
    assets: &AssetStore,
    attempt: Attempt,
    include_images: bool,
) -> Result<EvidenceArchive> {
    let _storage = assets.storage.read().await;
    if attempt.preservation["contract"] == "pending" {
        return Err(Error::Conflict(
            "Wait for evidence preservation to finish before exporting",
        ));
    }
    let metadata = serde_json::to_vec_pretty(&attempt)?;
    if metadata.len() > MAX_METADATA {
        return Err(Error::Conflict(
            "This run exceeds the 32 MiB evidence metadata limit",
        ));
    }
    let mut entries = vec![("evidence/attempt.json".to_owned(), metadata)];
    let mut ids: BTreeSet<String> = attempt
        .request
        .bound_images()
        .into_iter()
        .map(|(_, _, id)| id.to_owned())
        .collect();
    for output in &attempt.outputs {
        if let Some(id) = output["asset"]["id"].as_str() {
            ids.insert(id.into());
        }
    }
    if ids.len() > 128 {
        return Err(Error::Conflict(
            "An evidence archive supports up to 128 images",
        ));
    }
    let mut references = vec![];
    let mut total = entries[0].1.len();
    for id in ids {
        if include_images {
            let (asset, bytes) = assets.read(&id).await?;
            total = total
                .checked_add(bytes.len())
                .ok_or(Error::Conflict("Evidence archive is too large"))?;
            if total > MAX_TOTAL {
                return Err(Error::Conflict(
                    "Evidence archive exceeds 256 MiB; use references only",
                ));
            }
            let name = format!(
                "images/{}.{}",
                asset.sha256,
                if asset.media_type == "image/png" {
                    "png"
                } else {
                    "jpg"
                }
            );
            references.push(json!({"asset_id":id,"file":name,"sha256":asset.sha256}));
            entries.push((name, bytes));
        } else {
            references.push(json!({"asset_id":id,"file":null}));
        }
    }
    let files: Vec<Value> = entries
        .iter()
        .map(|(name, bytes)| json!({"path":name,"bytes":bytes.len(),"sha256":fingerprint(bytes)}))
        .collect();
    let manifest = json!({"format":"wafflery-evidence","version":1,"attempt_id":attempt.id,"images_included":include_images,
        "tool":attempt.declaration["document"]["tool"],"contract":attempt.declaration["document"]["contract"],"mcp_protocol":"2025-11-25",
        "files":files,"assets":references,"limitations":["Evidence package, not a portable native replay bundle.","Recorded paths refer to the original host and may no longer exist.","Provider credentials and host browser sessions are not captured.","Native bytes, tool report files and CLI handoff files are not included in this evidence package."]});
    entries.push((
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest)?,
    ));
    let file = tempfile::Builder::new()
        .prefix("evidence-")
        .suffix(".tar.gz")
        .tempfile_in(assets.root().join("staging"))?;
    tokio::task::spawn_blocking(move || write_archive(file, entries))
        .await
        .map_err(|_| Error::Unavailable)?
}
fn write_archive(
    mut file: tempfile::NamedTempFile,
    entries: Vec<(String, Vec<u8>)>,
) -> Result<EvidenceArchive> {
    let writer = LimitedWriter {
        inner: file.as_file_mut(),
        remaining: MAX_TOTAL + MAX_METADATA,
    };
    let gzip = GzEncoder::new(writer, Compression::fast());
    let mut tar = tar::Builder::new(gzip);
    tar.follow_symlinks(false);
    for (name, bytes) in entries {
        if std::path::Path::new(&name)
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(Error::Conflict("Unsafe evidence entry name"));
        }
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o600);
        header.set_mtime(0);
        header.set_uid(0);
        header.set_gid(0);
        tar.append_data(&mut header, &name, bytes.as_slice())?;
    }
    let mut writer = tar.into_inner()?.finish()?;
    writer.flush()?;
    let size = file.as_file().metadata()?.len();
    Ok(EvidenceArchive { file, size })
}
struct LimitedWriter<W> {
    inner: W,
    remaining: usize,
}
impl<W: Write> Write for LimitedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other(
                "Compressed archive exceeds its size limit",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
