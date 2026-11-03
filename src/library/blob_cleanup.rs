use super::{Error, Result, assets::AssetStore, blobs::Blob, cleanup::Selection};
use rusqlite::{Connection, params};
pub(super) fn candidates(db: &Connection, selection: &Selection) -> Result<Vec<Blob>> {
    let attempts = serde_json::to_string(&selection.attempts)?;
    let examples = serde_json::to_string(&selection.examples)?;
    let mut query=db.prepare("SELECT sha256,size FROM native_blobs b WHERE (?3 OR EXISTS(SELECT 1 FROM native_links l WHERE l.sha256=b.sha256 AND ((l.owner_kind='attempt' AND l.owner_id IN (SELECT value FROM json_each(?1))) OR (l.owner_kind='example' AND l.owner_id IN (SELECT value FROM json_each(?2)))))) AND NOT EXISTS(SELECT 1 FROM native_links l WHERE l.sha256=b.sha256 AND (l.owner_kind='bundle' OR (l.owner_kind='attempt' AND l.owner_id NOT IN (SELECT value FROM json_each(?1))) OR (l.owner_kind='example' AND l.owner_id NOT IN (SELECT value FROM json_each(?2))))) ORDER BY sha256 LIMIT 1000")?;
    Ok(query
        .query_map(
            params![attempts, examples, selection.unused_assets],
            |row| {
                Ok(Blob {
                    sha256: row.get(0)?,
                    size: row.get(1)?,
                })
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
/// Caller holds the store's write lock across reference deletion and filesystem reclamation.
pub(super) async fn reclaim(store: &AssetStore, blobs: Vec<Blob>) -> Result<(u64, Vec<String>)> {
    let mut reclaimed = 0;
    let mut failures = vec![];
    for blob in blobs {
        let digest = blob.sha256.clone();
        let referenced = store
            .database
            .call(move |db| {
                Ok(db.query_row(
                    "SELECT EXISTS(SELECT 1 FROM native_links WHERE sha256=?1)",
                    [digest],
                    |row| row.get::<_, bool>(0),
                )?)
            })
            .await?;
        if referenced {
            continue;
        }
        let root = store.root().to_owned();
        let digest = blob.sha256.clone();
        let removed = tokio::task::spawn_blocking(move || remove_owned(&root, &digest))
            .await
            .map_err(|_| Error::Unavailable)?;
        match removed {
            Ok(()) => {
                let digest = blob.sha256.clone();
                store
                    .database
                    .call(move |db| {
                        db.execute("DELETE FROM native_blobs WHERE sha256=?1", [digest])?;
                        Ok(())
                    })
                    .await?;
                reclaimed += u64::from(blob.size);
            }
            Err(error) => failures.push(format!(
                "Native blob {} could not be removed: {error}",
                blob.sha256
            )),
        }
    }
    Ok((reclaimed, failures))
}
fn remove_owned(root: &std::path::Path, digest: &str) -> Result<()> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(Error::InvalidAsset("Invalid native blob fingerprint"));
    }
    let parent = root.join("blobs/sha256").join(&digest[..2]);
    if parent.canonicalize()? != parent {
        return Err(Error::InvalidAsset("Native blob directory is not owned"));
    }
    let path = parent.join(digest);
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() || metadata.is_symlink() => std::fs::remove_file(path)?,
        Ok(_) => return Err(Error::InvalidAsset("Native blob path is not a file")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
