//! Ownership metadata is independent of artifact identity and native byte format.
use super::{Error, Result, blobs::BlobStore};
use rusqlite::params;
use serde_json::Value;
#[derive(Clone, Debug)]
pub enum Owner {
    Attempt(String),
    Example(String),
    Bundle(String),
}
impl Owner {
    fn parts(&self) -> (&str, &str) {
        match self {
            Self::Attempt(id) => ("attempt", id),
            Self::Example(id) => ("example", id),
            Self::Bundle(id) => ("bundle", id),
        }
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct BlobLink {
    pub direction: String,
    pub occurrence: u32,
    pub sha256: String,
    pub media_type: String,
    pub artifact: Value,
}
impl BlobStore {
    pub async fn attach(&self, owner: Owner, link: BlobLink) -> Result<()> {
        let _storage = self.0.storage.read().await;
        let (kind, id) = owner.parts();
        if id.is_empty()
            || id.len() > 128
            || link.media_type.trim().is_empty()
            || link.media_type.len() > 256
            || !link
                .media_type
                .bytes()
                .all(|byte| (b' '..=b'~').contains(&byte))
            || serde_json::to_vec(&link.artifact)?.len() > 64 * 1024
            || !link.artifact.is_object()
        {
            return Err(Error::InvalidAsset(
                "Invalid native blob ownership metadata",
            ));
        }
        let metadata = self.metadata(&link.sha256).await?;
        let root = self.0.root().to_owned();
        let digest = link.sha256.clone();
        let size = tokio::task::spawn_blocking(move || {
            super::blobs::read_verified(&root, &digest).map(|bytes| bytes.len() as u64)
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        if size != u64::from(metadata.size) {
            return Err(Error::InvalidAsset(
                "Native blob size conflicts with metadata",
            ));
        }
        let kind = kind.to_owned();
        let id = id.to_owned();
        self.0.database.call(move|db|{
            let tx=db.transaction()?;
            let full:bool=tx.query_row("SELECT count(*)>=1000 AND NOT EXISTS(SELECT 1 FROM native_links WHERE owner_kind=?1 AND owner_id=?2 AND direction=?3 AND occurrence=?4) FROM native_links WHERE owner_kind=?1 AND owner_id=?2",params![kind,id,link.direction,link.occurrence],|row|row.get(0))?;
            if full{return Err(Error::Conflict("A native blob owner supports up to 1,000 occurrences"));}
            tx.execute("INSERT INTO native_links(owner_kind,owner_id,direction,occurrence,sha256,media_type,artifact) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(owner_kind,owner_id,direction,occurrence) DO NOTHING",params![kind,id,link.direction,link.occurrence,link.sha256,link.media_type,serde_json::to_string(&link.artifact)?])?;
            let saved=tx.query_row("SELECT direction,occurrence,sha256,media_type,artifact FROM native_links WHERE owner_kind=?1 AND owner_id=?2 AND direction=?3 AND occurrence=?4",params![kind,id,link.direction,link.occurrence],from_row)?;
            if saved!=link{return Err(Error::Conflict("Native artifact ownership is already recorded differently"));}tx.commit()?;Ok(())
        }).await
    }
    pub async fn links(&self, owner: Owner) -> Result<Vec<BlobLink>> {
        let (kind, id) = owner.parts();
        let kind = kind.to_owned();
        let id = id.to_owned();
        self.0.database.call(move|db|{let mut query=db.prepare("SELECT direction,occurrence,sha256,media_type,artifact FROM native_links WHERE owner_kind=?1 AND owner_id=?2 ORDER BY direction,occurrence LIMIT 1000")?;Ok(query.query_map(params![kind,id],from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)}).await
    }
    /// Future bundle records release only their own links. File reclamation remains previewed cleanup.
    pub async fn release_bundle(&self, id: String) -> Result<()> {
        let _storage = self.0.storage.read().await;
        self.0
            .database
            .call(move |db| {
                db.execute(
                    "DELETE FROM native_links WHERE owner_kind='bundle' AND owner_id=?1",
                    [id],
                )?;
                Ok(())
            })
            .await
    }
}
fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BlobLink> {
    let raw: String = row.get(4)?;
    Ok(BlobLink {
        direction: row.get(0)?,
        occurrence: row.get(1)?,
        sha256: row.get(2)?,
        media_type: row.get(3)?,
        artifact: serde_json::from_str(&raw).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
    })
}
