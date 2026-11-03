use super::{
    Error, Result,
    assets::{Asset, AssetStore},
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub attempts: Vec<String>,
    pub examples: Vec<String>,
    #[serde(default)]
    pub unused_assets: bool,
}
#[derive(Serialize)]
pub struct Preview {
    pub token: String,
    pub selection: Selection,
    pub assets: Vec<Asset>,
    pub native_blobs: Vec<super::blobs::Blob>,
    pub total_native_bytes: u64,
    pub total_bundle_bytes: u64,
    pub bundle_count: u64,
    pub reclaimable_bytes: u64,
    pub total_asset_bytes: u64,
    pub baselines_cleared: usize,
}
#[derive(Serialize)]
pub struct Outcome {
    pub reclaimed_bytes: u64,
    pub failures: Vec<String>,
}
#[derive(Clone)]
pub struct Cleanup(pub AssetStore);
impl Cleanup {
    pub async fn preview(&self, selection: Selection) -> Result<Preview> {
        self.0.database.call(move |db| preview(db, selection)).await
    }
    pub async fn confirm(&self, selection: Selection, token: String) -> Result<Outcome> {
        let _storage = self.0.storage.write().await;
        let (assets, native_blobs) = self
            .0
            .database
            .call(move |db| {
                let tx = db.transaction()?;
                let current = preview(&tx, selection)?;
                if token != current.token {
                    return Err(Error::Conflict(
                        "Library references changed. Review a fresh cleanup preview.",
                    ));
                }
                tx.execute(
                    "DELETE FROM attempts WHERE id IN (SELECT value FROM json_each(?1))",
                    [serde_json::to_string(&current.selection.attempts)?],
                )?;
                tx.execute(
                    "DELETE FROM examples WHERE id IN (SELECT value FROM json_each(?1))",
                    [serde_json::to_string(&current.selection.examples)?],
                )?;
                tx.commit()?;
                Ok((current.assets, current.native_blobs))
            })
            .await?;
        let mut outcome = Outcome {
            reclaimed_bytes: 0,
            failures: vec![],
        };
        for asset in assets {
            let id = asset.id.clone();
            let referenced=self.0.database.call(move|db|Ok(db.query_row("SELECT EXISTS(SELECT 1 FROM attempt_assets WHERE asset_id=?1 UNION ALL SELECT 1 FROM example_assets WHERE asset_id=?1 UNION ALL SELECT 1 FROM bundle_assets WHERE asset_id=?1)",[id],|r|r.get::<_,bool>(0))?)).await?;
            if referenced {
                continue;
            }
            let removed = remove_owned(&self.0, &asset);
            match removed {
                Ok(()) => {
                    let id = asset.id.clone();
                    self.0
                        .database
                        .call(move |db| {
                            db.execute("DELETE FROM assets WHERE id=?1", [id])?;
                            Ok(())
                        })
                        .await?;
                    outcome.reclaimed_bytes += u64::from(asset.size);
                }
                Err(error) => outcome
                    .failures
                    .push(format!("Image {} could not be removed: {error}", asset.id)),
            }
        }
        let (bytes, failures) = super::blob_cleanup::reclaim(&self.0, native_blobs).await?;
        outcome.reclaimed_bytes += bytes;
        outcome.failures.extend(failures);
        Ok(outcome)
    }
}
fn preview(db: &Connection, mut selection: Selection) -> Result<Preview> {
    if selection.attempts.len() + selection.examples.len() > 1000 {
        return Err(Error::Conflict("Select up to 1,000 records per cleanup"));
    }
    selection.attempts.sort();
    selection.attempts.dedup();
    selection.examples.sort();
    selection.examples.dedup();
    let attempts = serde_json::to_string(&selection.attempts)?;
    let examples = serde_json::to_string(&selection.examples)?;
    let mut hash = Sha256::new();
    for (table, ids, count) in [
        ("attempts", &attempts, selection.attempts.len()),
        ("examples", &examples, selection.examples.len()),
    ] {
        let mut query = db.prepare(&format!(
            "SELECT record FROM {table} WHERE id IN (SELECT value FROM json_each(?1)) ORDER BY id"
        ))?;
        let mut found = 0;
        for row in query.query_map([ids], |r| r.get::<_, String>(0))? {
            hash.update(row?.as_bytes());
            found += 1;
        }
        if found != count {
            return Err(Error::Conflict(
                "A selected record is no longer available. Refresh cleanup.",
            ));
        }
    }
    let mut query=db.prepare("SELECT id,sha256,size,width,height,media_type FROM assets a WHERE (?3 OR a.id IN (SELECT asset_id FROM attempt_assets WHERE attempt_id IN (SELECT value FROM json_each(?1)) UNION SELECT asset_id FROM example_assets WHERE example_id IN (SELECT value FROM json_each(?2)))) AND NOT EXISTS(SELECT 1 FROM attempt_assets WHERE asset_id=a.id AND attempt_id NOT IN (SELECT value FROM json_each(?1))) AND NOT EXISTS(SELECT 1 FROM example_assets WHERE asset_id=a.id AND example_id NOT IN (SELECT value FROM json_each(?2))) AND NOT EXISTS(SELECT 1 FROM bundle_assets WHERE asset_id=a.id) ORDER BY id LIMIT 1000")?;
    let assets = query
        .query_map(params![attempts, examples, selection.unused_assets], |r| {
            Ok(Asset {
                id: r.get(0)?,
                sha256: r.get(1)?,
                size: r.get(2)?,
                width: r.get(3)?,
                height: r.get(4)?,
                media_type: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let total_asset_bytes: i64 =
        db.query_row("SELECT coalesce(sum(size),0) FROM assets", [], |r| r.get(0))?;
    let baselines_cleared: i64 = db.query_row(
        "SELECT count(*) FROM baselines WHERE attempt_id IN (SELECT value FROM json_each(?1))",
        [&attempts],
        |r| r.get(0),
    )?;
    let native_blobs = super::blob_cleanup::candidates(db, &selection)?;
    let total_native_bytes: i64 = db.query_row(
        "SELECT coalesce(sum(size),0) FROM native_blobs",
        [],
        |row| row.get(0),
    )?;
    let (bundle_count, total_bundle_bytes): (i64, i64) = db.query_row(
        "SELECT count(*),coalesce(sum(json_extract(record,'$.size')),0) FROM invocation_bundles",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let mut preview = Preview {
        bundle_count: bundle_count.max(0) as u64,
        total_bundle_bytes: total_bundle_bytes.max(0) as u64,
        token: String::new(),
        reclaimable_bytes: assets.iter().map(|a| u64::from(a.size)).sum::<u64>()
            + native_blobs
                .iter()
                .map(|blob| u64::from(blob.size))
                .sum::<u64>(),
        native_blobs,
        total_native_bytes: total_native_bytes.max(0) as u64,
        assets,
        selection,
        total_asset_bytes: total_asset_bytes.max(0) as u64,
        baselines_cleared: baselines_cleared as usize,
    };
    hash.update(serde_json::to_vec(&preview)?);
    preview.token = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(preview)
}
fn remove_owned(store: &AssetStore, asset: &Asset) -> Result<()> {
    if asset.sha256.len() != 64 || !asset.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidAsset("Invalid asset fingerprint"));
    }
    let parent = store.root().join("assets/sha256").join(&asset.sha256[..2]);
    if parent.canonicalize()? != parent {
        return Err(Error::InvalidAsset("Asset directory is no longer owned"));
    }
    let path = parent.join(&asset.sha256);
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() || metadata.is_symlink() => std::fs::remove_file(path)?,
        Ok(_) => return Err(Error::InvalidAsset("Asset path is not a file")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
