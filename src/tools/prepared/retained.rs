//! Bounded, session-owned drafts. This map never contains a dispatched attempt.
mod quotes;
use super::{Declaration, Error, PreparedCall, Result, Submission};
pub use quotes::PreparedEstimate;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, serde::Serialize)]
pub struct Preparation {
    pub id: String,
    pub tool: String,
    pub action: String,
    pub revision: String,
    pub expires_ms: u64,
    pub bytes: u64,
}
struct Draft {
    call: Arc<PreparedCall>,
    receipt: Preparation,
    expires: Instant,
    estimate_ticket: Option<quotes::EstimateTicket>,
    quote: Option<crate::tools::estimate::ValidatedEstimate>,
}
struct Tracked {
    call: std::sync::Weak<PreparedCall>,
    bytes: u64,
}
struct State {
    drafts: BTreeMap<String, Draft>,
    tracked: BTreeMap<String, Tracked>,
}
impl State {
    fn prune(&mut self, now: Instant) {
        self.drafts.retain(|_, draft| draft.expires > now);
        self.tracked
            .retain(|_, tracked| tracked.call.strong_count() > 0);
    }
}
pub(crate) struct Retained {
    state: Arc<Mutex<State>>,
    count: usize,
    bytes: u64,
    ttl: Duration,
}
impl Retained {
    pub fn new() -> Self {
        Self::with_limits(16, 1024 * 1024 * 1024, Duration::from_secs(15 * 60))
    }
    fn with_limits(count: usize, bytes: u64, ttl: Duration) -> Self {
        let state = Arc::new(Mutex::new(State {
            drafts: BTreeMap::new(),
            tracked: BTreeMap::new(),
        }));
        let weak = Arc::downgrade(&state);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                let Some(state) = weak.upgrade() else {
                    break;
                };
                state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .prune(Instant::now());
            }
        });
        Self {
            state,
            count,
            bytes,
            ttl,
        }
    }
    fn lock(&self) -> MutexGuard<'_, State> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.prune(Instant::now());
        state
    }
    #[cfg(test)]
    fn expire_at(&self, now: Instant) {
        self.lock().prune(now);
    }
    // The coordinator holds submission across this check and materialization: only one
    // preparation can be in progress, so its count/bytes cannot be consumed by another tab.
    pub fn capacity(&self, bytes: u64) -> Result<()> {
        let state = self.lock();
        if state.tracked.len() >= self.count
            || bytes
                > self
                    .bytes
                    .saturating_sub(state.tracked.values().map(|d| d.bytes).sum())
        {
            return Err(Error::Conflict(
                "Draft preparation limit reached. Discard an unused draft first.",
            ));
        }
        Ok(())
    }
    pub fn insert(&self, call: PreparedCall) -> Result<Preparation> {
        let bytes = call
            .inputs
            .as_object()
            .into_iter()
            .flat_map(|slots| slots.values())
            .flat_map(|items| items.as_array().into_iter().flatten())
            .map(|input| input["asset"]["size"].as_u64().unwrap_or(0))
            .sum();
        self.capacity(bytes)?;
        let mut state = self.lock();
        if state.tracked.contains_key(&call.request.id) {
            return Err(Error::Conflict("Preparation ID is already in use"));
        }
        let receipt = Preparation {
            id: call.request.id.clone(),
            tool: call.request.tool.clone(),
            action: call.request.action.clone(),
            revision: call.request.revision.clone(),
            bytes,
            expires_ms: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                + self.ttl)
                .as_millis() as u64,
        };
        let call = Arc::new(call);
        state.tracked.insert(
            receipt.id.clone(),
            Tracked {
                call: Arc::downgrade(&call),
                bytes,
            },
        );
        state.drafts.insert(
            receipt.id.clone(),
            Draft {
                call,
                receipt: receipt.clone(),
                expires: Instant::now() + self.ttl,
                estimate_ticket: None,
                quote: None,
            },
        );
        Ok(receipt)
    }
    pub fn get(&self, id: &str) -> Result<Preparation> {
        self.lock()
            .drafts
            .get(id)
            .map(|d| d.receipt.clone())
            .ok_or(Error::Missing("Preparation is missing or expired"))
    }
    pub fn existing(
        &self,
        request: &Submission,
        current: &Declaration,
        session: &str,
    ) -> Result<Option<Preparation>> {
        let mut state = self.lock();
        let Some(draft) = state.drafts.get(&request.id) else {
            return Ok(None);
        };
        if !matches(&draft.call, current, session) {
            state.drafts.remove(&request.id);
            return Err(Error::Conflict(
                "Preparation belongs to an outdated tool session or declaration",
            ));
        }
        if draft.call.request != *request {
            return Err(Error::Conflict(
                "Preparation ID already has different inputs",
            ));
        }
        Ok(Some(draft.receipt.clone()))
    }
    pub fn lease(
        &self,
        id: &str,
        current: &Declaration,
        session: &str,
    ) -> Result<Arc<PreparedCall>> {
        let mut state = self.lock();
        let draft = state
            .drafts
            .get(id)
            .ok_or(Error::Missing("Preparation is missing or expired"))?;
        if !matches(&draft.call, current, session) {
            state.drafts.remove(id);
            return Err(Error::Conflict(
                "Preparation belongs to an outdated tool session or declaration",
            ));
        }
        Ok(draft.call.clone())
    }
    pub fn is_current(
        &self,
        call: &Arc<PreparedCall>,
        current: &Declaration,
        session: &str,
    ) -> bool {
        self.lock()
            .drafts
            .get(&call.request.id)
            .is_some_and(|draft| Arc::ptr_eq(&draft.call, call) && matches(call, current, session))
    }
    pub fn take(
        &self,
        request: &Submission,
        current: &Declaration,
        session: &str,
    ) -> Result<PreparedCall> {
        self.existing(request, current, session)?
            .ok_or(Error::Missing("Preparation is missing or expired"))?;
        let mut state = self.lock();
        let draft = state
            .drafts
            .get(&request.id)
            .ok_or(Error::Missing("Preparation is missing or expired"))?;
        if Arc::strong_count(&draft.call) != 1 {
            return Err(Error::Conflict(
                "An estimate is still reading these prepared inputs",
            ));
        }
        let draft = state.drafts.remove(&request.id).unwrap();
        Arc::try_unwrap(draft.call)
            .map_err(|_| Error::Conflict("Prepared inputs still have a reader"))
    }
    pub fn discard(&self, id: &str) {
        self.lock().drafts.remove(id);
    }
    pub fn invalidate_tool(&self, tool: &str) {
        self.lock().drafts.retain(|_, d| d.receipt.tool != tool);
    }
    pub fn reconcile(&self, tool: &str, current: &Declaration, session: &str) {
        self.lock().drafts.retain(|_, draft| {
            draft.receipt.tool != tool || matches(&draft.call, current, session)
        });
    }
    pub fn is_empty(&self) -> bool {
        self.lock().tracked.is_empty()
    }
    pub fn clear(&self) {
        self.lock().drafts.clear();
    }
}
fn matches(call: &PreparedCall, current: &Declaration, session: &str) -> bool {
    call.request.revision == current.revision
        && call.declaration["session"] == session
        && call.declaration["tool_session"]["id"]
            .as_str()
            .is_some_and(|id| !id.is_empty() && current.tool_session["id"].as_str() == Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::attempts::AttemptStore;
    #[tokio::test]
    async fn retention_expiry_and_limits_release_only_unclaimed_copies() {
        let (_dir, assets, request, declaration) = super::super::tests::fixture().await;
        let retained = Retained::with_limits(1, 1024 * 1024, Duration::from_secs(60));
        let first = PreparedCall::new(&assets, request.clone(), &declaration, "session")
            .await
            .unwrap();
        retained.insert(first).unwrap();
        let mut other = request.clone();
        other.id = "other".into();
        let second = PreparedCall::new(&assets, other, &declaration, "session")
            .await
            .unwrap();
        assert!(retained.insert(second).is_err());
        assert!(assets.root().join("execution/prepared").exists());
        assert!(!assets.root().join("execution/other").exists());
        retained.expire_at(Instant::now() + Duration::from_secs(61));
        assert!(!assets.root().join("execution/prepared").exists());
        assert!(retained.take(&request, &declaration, "session").is_err());
        assert!(
            AttemptStore::new(assets.database.clone())
                .history(None, 50, None, None)
                .await
                .unwrap()
                .is_empty()
        );
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn retained_byte_limit_counts_each_copy_and_releases_rejected_owner() {
        let (_dir, assets, request, declaration) = super::super::tests::fixture().await;
        let bytes = u64::from(
            assets
                .metadata(&request.images["source"][0])
                .await
                .unwrap()
                .size,
        );
        let retained = Retained::with_limits(16, bytes - 1, Duration::from_secs(60));
        let first = PreparedCall::new(&assets, request, &declaration, "session")
            .await
            .unwrap();
        assert!(retained.insert(first).is_err());
        assert!(!assets.root().join("execution/prepared").exists());
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn session_or_declaration_changes_invalidate_unclaimed_drafts() {
        for change_session in [true, false] {
            let (_dir, assets, request, mut declaration) = super::super::tests::fixture().await;
            let retained = Retained::new();
            let call = PreparedCall::new(&assets, request, &declaration, "session")
                .await
                .unwrap();
            retained.insert(call).unwrap();
            let session = if change_session {
                "replacement"
            } else {
                declaration.revision = "new".into();
                "session"
            };
            retained.reconcile("fixture", &declaration, session);
            assert!(retained.get("prepared").is_err());
            assert!(!assets.root().join("execution/prepared").exists());
            assets.database.shutdown().await.unwrap();
        }
    }
    #[tokio::test]
    async fn consumed_preparation_cannot_be_deleted_by_expiry_or_discard() {
        let (_dir, assets, request, declaration) = super::super::tests::fixture().await;
        let retained = Retained::new();
        let call = PreparedCall::new(&assets, request.clone(), &declaration, "session")
            .await
            .unwrap();
        retained.insert(call).unwrap();
        let owned = retained.take(&request, &declaration, "session").unwrap();
        retained.discard(&request.id);
        retained.expire_at(Instant::now() + Duration::from_secs(3600));
        assert!(assets.root().join("execution/prepared").exists());
        let saved = owned
            .persist(&AttemptStore::new(assets.database.clone()))
            .await
            .unwrap();
        assert!(assets.root().join("execution/prepared").exists());
        super::super::super::materialized::discard(assets.root(), &saved).unwrap();
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn changed_or_missing_tool_session_identity_invalidates_even_with_the_same_revision() {
        for identity in [
            serde_json::json!("cc8f5520-a822-4cdc-a9eb-f1a72018e000"),
            serde_json::Value::Null,
        ] {
            let (_dir, assets, request, mut declaration) = super::super::tests::fixture().await;
            let retained = Retained::new();
            if identity.is_null() {
                declaration.tool_session["id"] = serde_json::Value::Null;
            }
            let call = PreparedCall::new(&assets, request, &declaration, "process-nonce")
                .await
                .unwrap();
            retained.insert(call).unwrap();
            declaration.tool_session["id"] = identity;
            retained.reconcile("fixture", &declaration, "process-nonce");
            assert!(retained.get("prepared").is_err());
            assert!(!assets.root().join("execution/prepared").exists());
            assets.database.shutdown().await.unwrap();
        }
    }
    #[tokio::test]
    async fn invalidated_leases_keep_files_and_capacity_until_the_reader_finishes() {
        for invalidate in ["discard", "expire", "restart", "reconcile", "clear"] {
            let (_dir, assets, request, mut declaration) = super::super::tests::fixture().await;
            let retained = Retained::with_limits(1, 1024 * 1024, Duration::from_secs(60));
            retained
                .insert(
                    PreparedCall::new(&assets, request.clone(), &declaration, "session")
                        .await
                        .unwrap(),
                )
                .unwrap();
            let lease = retained
                .lease(&request.id, &declaration, "session")
                .unwrap();
            let path =
                std::path::PathBuf::from(lease.arguments()["source"]["path"].as_str().unwrap());
            let bytes = std::fs::read(&path).unwrap();
            match invalidate {
                "discard" => retained.discard(&request.id),
                "expire" => retained.expire_at(Instant::now() + Duration::from_secs(61)),
                "restart" => retained.invalidate_tool(&request.tool),
                "reconcile" => {
                    declaration.revision = "new".into();
                    retained.reconcile(&request.tool, &declaration, "session");
                }
                _ => retained.clear(),
            }
            assert!(retained.get(&request.id).is_err(), "{invalidate}");
            assert!(retained.take(&request, &declaration, "session").is_err());
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
            assert!(
                retained.capacity(0).is_err(),
                "leased bytes/count must still be charged"
            );
            assert!(
                !retained.is_empty(),
                "cleanup must not wait behind an invisible leased read guard"
            );
            drop(lease);
            assert!(!path.exists());
            assert!(retained.capacity(0).is_ok());
            assert!(retained.is_empty());
            assets.database.shutdown().await.unwrap();
        }
    }
    #[tokio::test]
    async fn dispatch_cannot_take_paths_from_an_active_estimate_reader() {
        let (_dir, assets, request, declaration) = super::super::tests::fixture().await;
        let retained = Retained::new();
        retained
            .insert(
                PreparedCall::new(&assets, request.clone(), &declaration, "session")
                    .await
                    .unwrap(),
            )
            .unwrap();
        let lease = retained
            .lease(&request.id, &declaration, "session")
            .unwrap();
        assert!(retained.take(&request, &declaration, "session").is_err());
        assert!(retained.get(&request.id).is_ok());
        drop(lease);
        let call = retained.take(&request, &declaration, "session").unwrap();
        drop(call);
        assert!(!assets.root().join("execution/prepared").exists());
        assets.database.shutdown().await.unwrap();
    }
}
