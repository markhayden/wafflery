//! Completed bundles survive their source attempts. Publication and ownership are one commit.
use super::{
    Error, Result,
    assets::{AssetStore, private_directory},
    blobs::BlobStore,
    bundle_archive::{Bundle, MAX_TOTAL, Member, hex},
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub sha256: String,
    pub size: u64,
    pub manifest: Value,
    pub members: Vec<Member>,
    pub workspace_dependent: bool,
}
#[derive(Serialize)]
pub struct Summary {
    pub id: String,
    pub sequence: i64,
    pub size: u64,
    pub workspace_dependent: bool,
    pub tool: Value,
    pub action: String,
}
#[derive(Serialize)]
pub struct Page {
    pub items: Vec<Summary>,
    pub cursor: Option<i64>,
}
#[derive(Clone)]
pub struct BundleStore(pub AssetStore);
fn valid(id: &str) -> Result<()> {
    if id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Missing("Saved invocation bundle is unavailable"));
    }
    Ok(())
}
fn path(root: &Path, id: &str) -> Result<PathBuf> {
    valid(id)?;
    Ok(root.join("bundles/sha256").join(&id[..2]).join(id))
}
fn verified_file(root: &Path, id: &str, size: u64) -> Result<File> {
    let path = path(root, id)?;
    if path.parent().unwrap().canonicalize()? != path.parent().unwrap() {
        return Err(Error::InvalidAsset("Bundle directory is not owned"));
    }
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)?;
    verify(file, id, size)
}
fn verify(mut file: File, id: &str, size: u64) -> Result<File> {
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() != size || size > MAX_TOTAL {
        return Err(Error::InvalidAsset(
            "Saved invocation bundle size is invalid",
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    let mut digest = Sha256::new();
    let mut block = [0; 65536];
    let mut total = 0;
    loop {
        let n = file.read(&mut block)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_TOTAL {
            return Err(Error::InvalidAsset(
                "Saved invocation bundle exceeds 256 MiB",
            ));
        }
        digest.update(&block[..n]);
    }
    if total != size || hex(&digest.finalize()) != id {
        return Err(Error::InvalidAsset(
            "Saved invocation bundle fingerprint failed verification",
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}
impl BundleStore {
    pub async fn save(&self, attempt: &str, bundle: Bundle) -> Result<Record> {
        // Revalidate under the write lease: cleanup may have run after assembly's read lease ended.
        let _storage = self.0.storage.write().await;
        let source = attempt.to_owned();
        let exists = self
            .0
            .database
            .call(move |db| {
                Ok(db.query_row(
                    "SELECT EXISTS(SELECT 1 FROM attempts WHERE id=?1)",
                    [source],
                    |row| row.get::<_, bool>(0),
                )?)
            })
            .await?;
        if !exists {
            return Err(Error::Conflict(
                "The source run was deleted before this bundle was saved. Reopen a retained run.",
            ));
        }
        valid(&bundle.sha256)?;
        for id in &bundle.input_assets {
            self.0.read(id).await?;
        }
        let blobs = BlobStore(self.0.clone());
        for link in &bundle.native_links {
            blobs.read_leased(&link.sha256).await?;
        }
        let record = Record {
            id: bundle.sha256.clone(),
            sha256: bundle.sha256,
            size: bundle.size,
            manifest: bundle.manifest,
            members: bundle.members,
            workspace_dependent: bundle.workspace_dependent,
        };
        let root = self.0.root().to_owned();
        let id = record.id.clone();
        let size = record.size;
        let created = tokio::task::spawn_blocking(move || -> Result<bool> {
            verify(bundle.file.reopen()?, &id, size)?;
            let destination = path(&root, &id)?;
            let parent = destination.parent().unwrap();
            private_directory(parent)?;
            if parent.canonicalize()? != parent {
                return Err(Error::InvalidAsset("Bundle directory is not owned"));
            }
            bundle.file.as_file().sync_all()?;
            bundle
                .file
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o444))?;
            match bundle.file.persist_noclobber(&destination) {
                Ok(_) => {
                    File::open(parent)?.sync_all()?;
                    Ok(true)
                }
                Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                    verified_file(&root, &id, size)?;
                    Ok(false)
                }
                Err(error) => Err(error.error.into()),
            }
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let saved = record.clone();
        let attempt = attempt.to_owned();
        let result=self.0.database.call(move|db|{
            let tx=db.transaction()?;
            tx.execute("INSERT INTO invocation_bundles(id,record) VALUES(?1,?2) ON CONFLICT(id) DO NOTHING",params![saved.id,serde_json::to_string(&saved)?])?;
            let existing:String=tx.query_row("SELECT record FROM invocation_bundles WHERE id=?1",[&saved.id],|row|row.get(0))?;
            if serde_json::from_str::<Record>(&existing)?!=saved{return Err(Error::Conflict("Saved invocation bundle metadata conflicts with its contents"));}
            tx.execute("INSERT OR IGNORE INTO bundle_attempts(bundle_id,attempt_id) VALUES(?1,?2)",params![saved.id,attempt])?;
            for id in bundle.input_assets {tx.execute("INSERT OR IGNORE INTO bundle_assets(bundle_id,asset_id) VALUES(?1,?2)",params![saved.id,id])?;}
            for (index,link) in bundle.native_links.into_iter().enumerate(){
                tx.execute("INSERT OR IGNORE INTO native_links(owner_kind,owner_id,direction,occurrence,sha256,media_type,artifact) VALUES('bundle',?1,'bundle',?2,?3,?4,?5)",params![saved.id,index as u32,link.sha256,link.media_type,serde_json::to_string(&link.artifact)?])?;
            }
            tx.commit()?;Ok(saved)
        }).await;
        if result.is_err() && created {
            // The same write lease excludes another save adopting this newly published file.
            let destination = path(self.0.root(), &record.id)?;
            tokio::task::spawn_blocking(move || fs::remove_file(destination))
                .await
                .map_err(|_| Error::Unavailable)??;
        }
        result
    }
    pub async fn get(&self, id: &str) -> Result<Record> {
        valid(id)?;
        let id = id.to_owned();
        self.0
            .database
            .call(move |db| {
                let raw: Option<String> = db
                    .query_row(
                        "SELECT record FROM invocation_bundles WHERE id=?1",
                        [id],
                        |row| row.get(0),
                    )
                    .optional()?;
                serde_json::from_str(
                    &raw.ok_or(Error::Missing("Saved invocation bundle is unavailable"))?,
                )
                .map_err(Into::into)
            })
            .await
    }
    pub async fn open(&self, id: &str) -> Result<(File, u64)> {
        let _storage = self.0.storage.read().await;
        let saved = self.get(id).await?;
        let root = self.0.root().to_owned();
        tokio::task::spawn_blocking(move || {
            Ok((verified_file(&root, &saved.id, saved.size)?, saved.size))
        })
        .await
        .map_err(|_| Error::Unavailable)?
    }
    pub async fn list(&self, cursor: Option<i64>, attempt: Option<String>) -> Result<Page> {
        self.0.database.call(move |db| {
            let mut query=db.prepare("SELECT rowid,id,json_extract(record,'$.size'),json_extract(record,'$.workspace_dependent'),json_extract(record,'$.manifest.steps[0].tool'),json_extract(record,'$.manifest.steps[0].invocation.action') FROM invocation_bundles b WHERE rowid<?1 AND (?2 IS NULL OR EXISTS(SELECT 1 FROM bundle_attempts a WHERE a.bundle_id=b.id AND a.attempt_id=?2)) ORDER BY rowid DESC LIMIT 21")?;
            let mut items=query.query_map(params![cursor.unwrap_or(i64::MAX),attempt],|row| {
                let tool:String=row.get(4)?;
                Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,u32>(2)?,row.get::<_,bool>(3)?,tool,row.get::<_,String>(5)?))
            })?.map(|row| {
                let (sequence,id,size,workspace_dependent,tool,action)=row?;
                Ok(Summary{id,sequence,size:u64::from(size),workspace_dependent,tool:serde_json::from_str(&tool)?,action})
            }).collect::<Result<Vec<_>>>()?;
            let cursor=if items.len()>20 {items.pop();items.last().map(|item|item.sequence)} else {None};
            Ok(Page{items,cursor})
        }).await
    }
}
