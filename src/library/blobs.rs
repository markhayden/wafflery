//! Opaque, content-addressed native bytes. Format interpretation belongs to the tool.
use super::{
    Error, Result,
    assets::{AssetStore, private_directory},
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
pub const MAX_BLOB_BYTES: u64 = 128 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Blob {
    pub sha256: String,
    pub size: u32,
}
#[derive(Clone)]
pub struct BlobStore(pub AssetStore);
fn valid_digest(hash: &str) -> Result<()> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::InvalidAsset("Invalid native blob fingerprint"));
    }
    Ok(())
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn path(root: &Path, digest: &str) -> Result<PathBuf> {
    valid_digest(digest)?;
    Ok(root.join("blobs/sha256").join(&digest[..2]).join(digest))
}
pub(super) fn read_verified(root: &Path, digest: &str) -> Result<Vec<u8>> {
    let path = path(root, digest)?;
    let parent = path.parent().unwrap();
    if parent.canonicalize()? != parent {
        return Err(Error::InvalidAsset("Native blob directory is not owned"));
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::InvalidAsset("Native blob is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BLOB_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BLOB_BYTES || hash(&bytes) != digest {
        return Err(Error::InvalidAsset(
            "Native blob failed fingerprint verification",
        ));
    }
    Ok(bytes)
}
impl BlobStore {
    pub async fn publish(&self, bytes: Vec<u8>, expected: String) -> Result<Blob> {
        valid_digest(&expected)?;
        if bytes.len() as u64 > MAX_BLOB_BYTES {
            return Err(Error::InvalidAsset("Native blob exceeds 128 MiB"));
        }
        let _storage = self.0.storage.read().await;
        let root = self.0.root().to_owned();
        let blob = tokio::task::spawn_blocking(move || -> Result<Blob> {
            if hash(&bytes) != expected {
                return Err(Error::InvalidAsset(
                    "Native blob fingerprint differs from the export",
                ));
            }
            let destination = path(&root, &expected)?;
            let directory = destination.parent().unwrap();
            private_directory(directory)?;
            if directory.canonicalize()? != directory {
                return Err(Error::InvalidAsset("Native blob directory is not owned"));
            }
            let mut staged = tempfile::NamedTempFile::new_in(root.join("staging"))?;
            staged.write_all(&bytes)?;
            staged.as_file().sync_all()?;
            staged
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o444))?;
            match staged.persist_noclobber(&destination) {
                Ok(_) => File::open(directory)?.sync_all()?,
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                    read_verified(&root, &expected)?;
                }
                Err(error) => return Err(error.error.into()),
            }
            Ok(Blob {
                sha256: expected,
                size: bytes.len() as u32,
            })
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        self.0.database.call(move|db|{db.execute("INSERT INTO native_blobs(sha256,size) VALUES(?1,?2) ON CONFLICT(sha256) DO NOTHING",params![blob.sha256,blob.size])?;let saved= db.query_row("SELECT sha256,size FROM native_blobs WHERE sha256=?1",[&blob.sha256],row)?;if blob!=saved{return Err(Error::InvalidAsset("Native blob metadata conflicts with its bytes"));}Ok(saved)}).await
    }
    pub async fn read(&self, digest: &str) -> Result<Vec<u8>> {
        let _storage = self.0.storage.read().await;
        let digest = digest.to_owned();
        valid_digest(&digest)?;
        let expected = self.metadata(&digest).await?;
        let root = self.0.root().to_owned();
        let bytes = tokio::task::spawn_blocking(move || read_verified(&root, &digest))
            .await
            .map_err(|_| Error::Unavailable)??;
        if bytes.len() as u64 != u64::from(expected.size) {
            return Err(Error::InvalidAsset(
                "Native blob size conflicts with metadata",
            ));
        }
        Ok(bytes)
    }
    pub async fn metadata(&self, digest: &str) -> Result<Blob> {
        let digest = digest.to_owned();
        valid_digest(&digest)?;
        self.0
            .database
            .call(move |db| {
                Ok(db.query_row(
                    "SELECT sha256,size FROM native_blobs WHERE sha256=?1",
                    [digest],
                    row,
                )?)
            })
            .await
    }
    pub async fn list(&self) -> Result<Vec<Blob>> {
        self.0
            .database
            .call(|db| {
                let mut query =
                    db.prepare("SELECT sha256,size FROM native_blobs ORDER BY sha256 LIMIT 1000")?;
                Ok(query
                    .query_map([], row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?)
            })
            .await
    }
}
fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Blob> {
    Ok(Blob {
        sha256: row.get(0)?,
        size: row.get(1)?,
    })
}
