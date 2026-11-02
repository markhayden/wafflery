//! Exact input copies belong to a preparation until its attempt is durably recorded.
pub(super) mod retained;
use super::{Error, Result, client::Declaration, validator};
use crate::library::{
    assets::AssetStore,
    attempts::{Attempt, AttemptStore, Submission},
};
use serde_json::Value;
use std::{os::unix::fs::DirBuilderExt, path::PathBuf};

pub(super) struct PreparedCall {
    request: Submission,
    arguments: Value,
    inputs: Value,
    declaration: Value,
    directory: Option<PathBuf>,
    // Prevent cleanup from removing asset references before the attempt transaction commits.
    _storage: tokio::sync::OwnedRwLockReadGuard<()>,
}
impl PreparedCall {
    // Called by an owned coordinator task: requester cancellation cannot interrupt a copy
    // or drop this owner while its database transaction is still queued.
    pub async fn new(
        assets: &AssetStore,
        request: Submission,
        current: &Declaration,
        session: &str,
    ) -> Result<Self> {
        validate_id(&request.id)?;
        if request.revision != current.revision {
            return Err(Error::Conflict(
                "Tool description changed. Reload its inputs before running.",
            ));
        }
        let action = current.document["actions"]
            .as_array()
            .and_then(|actions| actions.iter().find(|a| a["name"] == request.action))
            .ok_or(Error::Missing("Action not declared"))?;
        validator(&action["params_schema"])?
            .validate(&request.params)
            .map_err(|e| Error::Invalid(format!("Invalid parameters: {e}")))?;
        let storage = assets.storage.clone().read_owned().await;
        let directory = assets.root().join("execution").join(&request.id);
        std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
        let mut prepared = Self {
            request,
            arguments: Value::Null,
            inputs: Value::Null,
            declaration: Value::Null,
            directory: Some(directory.clone()),
            _storage: storage,
        };
        prepared.declaration = serde_json::to_value(current)?;
        prepared.declaration["session"] = Value::String(session.to_owned());
        (prepared.arguments, prepared.inputs) =
            super::bindings::materialize(assets, &prepared.request, action, current, &directory)
                .await?;
        Ok(prepared)
    }
    pub fn arguments(&self) -> &Value {
        &self.arguments
    }
    pub async fn persist(mut self, attempts: &AttemptStore) -> Result<Attempt> {
        let attempt = attempts
            .create(
                self.request.clone(),
                self.arguments.clone(),
                self.declaration.clone(),
                self.inputs.clone(),
            )
            .await?;
        // The durable attempt now owns these same paths; never copy or move them again.
        self.directory.take();
        Ok(attempt)
    }
}
impl Drop for PreparedCall {
    fn drop(&mut self) {
        if let Some(directory) = &self.directory
            && directory
                .canonicalize()
                .is_ok_and(|actual| actual == *directory)
        {
            // No tool has seen this unclaimed directory; it contains only our temporary copies.
            let _ = std::fs::remove_dir_all(directory);
        }
    }
}
pub(super) fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(Error::Invalid("Invalid submission ID".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::cleanup::{Cleanup, Selection};
    use serde_json::json;

    pub(super) async fn fixture() -> (tempfile::TempDir, AssetStore, Submission, Declaration) {
        let dir = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&dir.path().join("library")).unwrap();
        std::fs::create_dir(assets.root().join("execution")).unwrap();
        let path = dir.path().join("source.png");
        std::fs::write(&path, include_bytes!("../../ui/tests/fixtures/sample.png")).unwrap();
        let asset = assets.import_path(path).await.unwrap();
        let request = serde_json::from_value(json!({"id":"prepared", "tool":"fixture", "action":"mark", "revision":"r", "params":{"label":"before"}, "images":{"source":[asset.id]}})).unwrap();
        let declaration = Declaration {
            document: json!({"profiles":["images"],"actions":[{"name":"mark", "params_schema":{"type":"object"}, "images":[{"slot":"source","role":"source","required":true}]}]}),
            revision: "r".into(),
            generation: 0,
            tool_session: json!({"id":"bb8f5520-a822-4cdc-a9eb-f1a72018e000"}),
            schemas: [("mark".into(), json!({"type":"object"}))].into(),
        };
        (dir, assets, request, declaration)
    }
    #[tokio::test]
    async fn prepared_bytes_and_arguments_survive_source_changes_without_a_run() {
        let (dir, assets, mut request, declaration) = fixture().await;
        let attempts = AttemptStore::new(assets.database.clone());
        let prepared = PreparedCall::new(&assets, request.clone(), &declaration, "session")
            .await
            .unwrap();
        assert!(
            attempts
                .history(None, 50, None, None)
                .await
                .unwrap()
                .is_empty()
        );
        let arguments = prepared.arguments.clone();
        let path = std::path::PathBuf::from(arguments["source"]["path"].as_str().unwrap());
        let before = std::fs::read(&path).unwrap();
        let asset = assets.metadata(&request.images["source"][0]).await.unwrap();
        let library_path = assets
            .root()
            .join("assets/sha256")
            .join(&asset.sha256[..2])
            .join(asset.sha256);
        std::fs::remove_file(&library_path).unwrap();
        std::fs::write(library_path, b"changed bytes").unwrap();
        std::fs::write(dir.path().join("source.png"), b"changed original").unwrap();
        request.params["label"] = json!("after");
        let saved = prepared.persist(&attempts).await.unwrap();
        assert_eq!(saved.arguments, arguments);
        assert_eq!(saved.request.params["label"], "before");
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(saved.declaration["session"], "session");
        assert_eq!(
            attempts.history(None, 50, None, None).await.unwrap().len(),
            1
        );
        super::super::materialized::discard(assets.root(), &saved).unwrap();
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn abandoned_failed_and_duplicate_preparations_do_not_touch_other_inputs() {
        let (_dir, assets, request, declaration) = fixture().await;
        let first = PreparedCall::new(&assets, request.clone(), &declaration, "session")
            .await
            .unwrap();
        let path = assets.root().join("execution/prepared");
        assert!(
            PreparedCall::new(&assets, request.clone(), &declaration, "session")
                .await
                .is_err()
        );
        assert!(path.exists());
        let mut second = request.clone();
        second.id = "other".into();
        let other = PreparedCall::new(&assets, second, &declaration, "session")
            .await
            .unwrap();
        drop(first);
        assert!(!path.exists());
        assert!(assets.root().join("execution/other").exists());
        drop(other);
        let mut invalid = request;
        invalid.images.insert("unknown".into(), vec![]);
        assert!(
            PreparedCall::new(&assets, invalid, &declaration, "session")
                .await
                .is_err()
        );
        assert!(!path.exists());
        assert!(assets.list().await.unwrap().len() == 1);
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn cleanup_cannot_remove_references_during_transfer_to_an_attempt() {
        let (_dir, assets, request, declaration) = fixture().await;
        let prepared = PreparedCall::new(&assets, request, &declaration, "session")
            .await
            .unwrap();
        let cleanup = Cleanup(assets.clone());
        let selection = Selection {
            unused_assets: true,
            ..Default::default()
        };
        let preview = cleanup.preview(selection.clone()).await.unwrap();
        let mut cleaning = Box::pin(cleanup.confirm(selection, preview.token));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), &mut cleaning)
                .await
                .is_err()
        );
        let attempts = AttemptStore::new(assets.database.clone());
        let saved = prepared.persist(&attempts).await.unwrap();
        assert!(
            cleaning.await.is_err(),
            "the old cleanup preview must be stale after transfer"
        );
        assert!(
            assets
                .read(&saved.request.images["source"][0])
                .await
                .is_ok()
        );
        super::super::materialized::discard(assets.root(), &saved).unwrap();
        assets.database.shutdown().await.unwrap();
    }
}
