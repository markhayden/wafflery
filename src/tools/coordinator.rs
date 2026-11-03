mod artifact_controls;
pub(super) mod artifacts;
mod bundle_imports;
mod estimates;
mod native_preservation;
mod preparation;
mod readiness;
mod renders;
mod replays;
pub use super::prepared::retained::{Preparation, PreparedEstimate};
mod recovery;
mod restart;
mod shutdown;
use super::{
    Error, Result,
    client::{Declaration, ToolClient, structured_result},
    dispatch_transport::Permit,
    registration::Descriptor,
    validate_contract, validator,
};
use crate::library::{
    assets::AssetStore,
    attempts::{Attempt, AttemptStore, Submission},
};
use serde_json::{Value, json};
pub use shutdown::ShutdownStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::BTreeMap, os::unix::fs::DirBuilderExt, sync::Arc};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

struct Entry {
    descriptor: Descriptor,
    state: std::sync::RwLock<DeclarationState>,
    reservation: Arc<Semaphore>,
    discovery: Arc<Mutex<()>>,
}
impl Entry {
    fn client(&self) -> Option<Arc<ToolClient>> {
        self.state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .client
            .clone()
    }
}
struct DeclarationState {
    client: Option<Arc<ToolClient>>,
    declaration: Option<Declaration>,
    error: Option<String>,
}
struct Active {
    client: Arc<ToolClient>,
    finished: Arc<AtomicBool>,
    _permit: OwnedSemaphorePermit,
    written: tokio::sync::watch::Receiver<bool>,
    cancel_requested: bool,
}
// Declare before the local permit so rejected submissions publish after releasing it.
struct SubmissionChange(crate::events::Events);
impl Drop for SubmissionChange {
    fn drop(&mut self) {
        self.0.changed();
    }
}
pub struct Options {
    pub preserve_native: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            preserve_native: true,
        }
    }
}
pub struct Coordinator {
    pub assets: AssetStore,
    pub events: crate::events::Events,
    pub attempts: AttemptStore,
    tools: BTreeMap<String, Entry>,
    native_queue: Arc<super::native_queue::Queue>,
    preserve_native: bool,
    artifact_jobs: Arc<super::artifact_jobs::Jobs>,
    submission: Mutex<()>,
    replays: replays::Registry,
    preparations: super::prepared::retained::Retained,
    active: Mutex<BTreeMap<String, Active>>,
    renders: Mutex<BTreeMap<String, renders::View>>,
    workers: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    stopping: AtomicBool,
    shutdown_phase: std::sync::atomic::AtomicU8,
}
impl Coordinator {
    pub async fn open(assets: AssetStore, descriptors: Vec<Descriptor>) -> Result<Self> {
        Self::open_with_options(assets, descriptors, Options::default()).await
    }
    pub async fn open_with_options(
        assets: AssetStore,
        descriptors: Vec<Descriptor>,
        options: Options,
    ) -> Result<Self> {
        let attempts = AttemptStore::new(assets.database.clone());
        attempts.recover().await?;
        let execution = assets.root().join("execution");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(&execution)?;
        super::prepared::files::recover(&assets, &attempts).await?;
        let mut names = std::collections::HashSet::new();
        if descriptors.len() > 32 || descriptors.iter().any(|d| !names.insert(d.id.clone())) {
            return Err(Error::Invalid(
                "Duplicate or excessive registered tools".into(),
            ));
        }
        let mut tools = BTreeMap::new();
        for mut descriptor in descriptors {
            if tools.contains_key(&descriptor.id) || tools.len() >= 32 {
                return Err(Error::Invalid(
                    "Duplicate or excessive registered tools".into(),
                ));
            }
            descriptor.allow_dirs.push(execution.clone());
            let id = descriptor.id.clone();
            let super::probe::Started {
                client,
                declaration,
                error,
            } = super::probe::start(descriptor.clone()).await;
            tools.insert(
                id,
                Entry {
                    descriptor,
                    state: std::sync::RwLock::new(DeclarationState {
                        client,
                        declaration,
                        error,
                    }),
                    reservation: Arc::new(Semaphore::new(1)),
                    discovery: Arc::new(Mutex::new(())),
                },
            );
        }
        let events = crate::events::Events::new()?;
        let artifact_jobs = super::artifact_jobs::Jobs::new(events.clone());
        Ok(Self {
            assets,
            events,
            artifact_jobs,
            native_queue: Arc::new(super::native_queue::Queue::default()),
            preserve_native: options.preserve_native,
            attempts,
            tools,
            submission: Mutex::new(()),
            replays: replays::Registry::default(),
            preparations: super::prepared::retained::Retained::new(),
            active: Mutex::new(BTreeMap::new()),
            renders: Mutex::new(BTreeMap::new()),
            workers: Mutex::new(vec![]),
            stopping: AtomicBool::new(false),
            shutdown_phase: std::sync::atomic::AtomicU8::new(0),
        })
    }
    pub async fn description(&self, id: &str) -> Result<Declaration> {
        self.tools
            .get(id)
            .ok_or(Error::Missing("Tool not registered"))?
            .state
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .declaration
            .clone()
            .ok_or_else(|| Error::Invalid("This tool has no supported declaration".into()))
    }
    pub async fn session(&self, id: &str) -> Result<Value> {
        let entry = self
            .tools
            .get(id)
            .ok_or(Error::Missing("Tool not registered"))?;
        let _discovery = entry.discovery.lock().await;
        let declaration = self.description(id).await?;
        let client = entry
            .client()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        client.observe_session(&declaration.document["tool"]).await
    }
    pub async fn snapshot(&self) -> Result<Value> {
        self.reap_exited().await;
        let tools: Vec<_> = self.tools.iter().map(|(id, entry)| {
            let state = entry.state.read().unwrap_or_else(|e| e.into_inner());
            let mut declaration = state.declaration.clone();
            if let (Some(declaration), Some(client)) = (&mut declaration, &state.client)
                && let Some(observed) = client.last_session.read().unwrap_or_else(|e| e.into_inner()).clone()
                && observed["id"] == declaration.tool_session["id"]
            {
                // Amounts are observations, not input schema changes or spending permission.
                declaration.tool_session = observed;
            }
            // These are the validated launch/read permissions, not the tool's effective grants.
            let directories = state.client.as_ref().map(|client| json!({
                "requested_dirs": client.descriptor.allow_dirs,
                "evidence_dirs": client.descriptor.evidence_dirs,
            }));
            json!({"id":id,"declaration":declaration,"error":state.error,"directory_grants":directories,
                "busy":entry.reservation.available_permits()==0,
                "progress":state.client.as_ref().and_then(|client|client.progress().borrow().clone())})
        }).collect();
        Ok(
            json!({"native_preservations":self.native_queue.count(),"artifact_transfers":self.artifact_jobs.count(),"tools":tools,"attempts":self.attempts.history(None,50,None,None).await?,"event_session":self.events.session()}),
        )
    }
    pub async fn submit(self: &Arc<Self>, request: Submission) -> Result<Attempt> {
        let coordinator = self.clone();
        tokio::spawn(async move { coordinator.submit_owned(request, false).await })
            .await
            .map_err(|error| Error::Invalid(format!("Submission task failed: {error}")))?
    }
    async fn submit_owned(
        self: &Arc<Self>,
        request: Submission,
        retained: bool,
    ) -> Result<Attempt> {
        super::prepared::validate_id(&request.id)?;
        let _submission = self.submission.lock().await;
        let _change = SubmissionChange(self.events.clone());
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        if let Some(attempt) = self.attempts.find(&request).await? {
            return Ok(attempt);
        }
        self.validate_lineage(&request).await?;
        let entry = self
            .tools
            .get(&request.tool)
            .ok_or(Error::Missing("Tool not registered"))?;
        let permit = entry.reservation.clone().try_acquire_owned().map_err(|_| {
            Error::Conflict("This tool is busy. Wait for its current attempt to finish.")
        })?;
        let client = entry
            .client()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        // Serialize discovery through preflight and the actual wire write, not execution.
        let discovery = entry.discovery.clone().lock_owned().await;
        let current = self.refresh_under_guard(&request.tool).await?;
        self.validate_bundle_source(&request, &current.document)
            .await?;
        if current.revision != request.revision {
            return Err(Error::Conflict(
                "Tool description changed. Reload its inputs before running.",
            ));
        }
        let action = current.document["actions"]
            .as_array()
            .and_then(|actions| actions.iter().find(|a| a["name"] == request.action))
            .ok_or(Error::Missing("Action not declared"))?;
        match action["cost"]["may_spend"].as_bool() {
            Some(false) => (),
            Some(true) if retained => (),
            Some(true) => {
                return Err(Error::Conflict(
                    "Paid actions require an owned preparation and allowed estimate",
                ));
            }
            None => {
                return Err(Error::Conflict(
                    "Action spending declaration is unavailable",
                ));
            }
        }
        let (mut prepared, estimate) = if retained {
            self.preparations
                .take(&request, &current, &client.session)?
        } else {
            if self.preparations.get(&request.id).is_ok() {
                return Err(Error::Conflict(
                    "This ID belongs to a prepared draft. Submit that preparation instead.",
                ));
            }
            (
                super::prepared::PreparedCall::new(
                    &self.assets,
                    request.clone(),
                    &current,
                    &client.session,
                )
                .await?,
                None,
            )
        };
        prepared.set_launch(&client.descriptor)?;
        let arguments = prepared.arguments().clone();
        let attempt = prepared.persist(&self.attempts, estimate.as_ref()).await?;
        self.claim_chain_request(&request).await?;
        let binding = estimate
            .as_ref()
            .map(|quote| quote.observation()["binding"].clone());
        let (written, dispatched) = tokio::sync::watch::channel(false);
        *client.gate.lock().unwrap_or_else(|e| e.into_inner()) = Some(Permit {
            discovery,
            id: request.id.clone(),
            store: self.attempts.clone(),
            action: request.action.clone(),
            arguments: arguments.clone(),
            generation: current.generation,
            estimate,
            written,
        });
        let finished = Arc::new(AtomicBool::new(false));
        self.active.lock().await.insert(
            request.id.clone(),
            Active {
                client: client.clone(),
                finished: finished.clone(),
                _permit: permit,
                written: dispatched,
                cancel_requested: false,
            },
        );
        let coordinator = self.clone();
        let worker = tokio::spawn(async move {
            coordinator
                .execute(client, request, arguments, binding)
                .await;
            finished.store(true, Ordering::Release);
            coordinator.events.changed();
        });
        self.workers.lock().await.push(worker);
        self.events.changed();
        Ok(attempt)
    }
    async fn execute(
        self: &Arc<Self>,
        client: Arc<ToolClient>,
        request: Submission,
        arguments: Value,
        binding: Option<Value>,
    ) {
        let result = async {
            let mut meta = json!({"table-manners/client_run_id":request.id});
            if let Some(binding) = binding { meta["table-manners/estimate"] = binding; }
            let message = serde_json::from_value(json!({"method":"tools/call","params":{"name":request.action,"arguments":arguments,"_meta":meta}}))?;
            client.start_association(&request.id);
            let mut progress = client.progress();
            let call = client.peer()?.send_request(message);
            tokio::pin!(call);
            let response = loop {
                tokio::select! {
                    response = &mut call => { let response=response?;client.begin_preservation();break response; },
                    changed = progress.changed() => {
                        if changed.is_ok() { self.record_reference(&client, &request.id).await?; self.events.changed(); }
                    },
                }
            };
            let raw = serde_json::to_value(response)?;
            let saved_raw = raw.clone();
            self.attempts.update(&request.id, move |attempt| {
                attempt.raw_response = Some(saved_raw);
                Ok(())
            }).await?;
            let result = structured_result(&raw)?;
            validate_contract("urn:table-manners:core:result", &result)?;
            if result["action"] != request.action {
                return Err(Error::Invalid("Result action does not match the submitted action".into()));
            }
            Ok::<_, Error>(result)
        }.await;
        let result = match result {
            Err(Error::Mcp(rmcp::ServiceError::Cancelled { .. })) => {
                self.recover_associated(&client, &request.id).await
            }
            Err(error @ Error::Mcp(rmcp::ServiceError::McpError(_))) => {
                self.recover_rpc(&client, &request.id, error).await
            }
            other => other,
        };
        let mut preserved = false;
        match result {
            Ok(result) => {
                let saved = async {
                    self.record_reference(&client, &request.id).await?;
                    let attempt = self.attempts.get(&request.id).await?;
                    let input_notices = super::evidence::check_association(&attempt, &result)?;
                    let attempt = self.attempts.bind_result(&request.id, result).await?;
                    if self.preserve_native
                        && let Err(error) = self.queue_native_preservation(&request.id).await
                    {
                        let reason = error.to_string();
                        let _ = self
                            .attempts
                            .update(&request.id, move |attempt| {
                                attempt.preservation["native"] = json!("failed");
                                attempt.notices.push(format!(
                                    "Native preservation could not be queued: {reason}"
                                ));
                                Ok(())
                            })
                            .await;
                    }
                    self.events.changed();
                    let (outputs, journal, mut notices) =
                        super::evidence::capture(&self.assets, &client, &attempt).await;
                    notices.extend(input_notices);
                    let session_after = match client
                        .observe_session(&attempt.declaration["document"]["tool"])
                        .await
                    {
                        Ok(session) => Some(session),
                        Err(error) => {
                            notices.push(format!("Final session observation unavailable: {error}"));
                            None
                        }
                    };
                    self.attempts
                        .update(&request.id, move |attempt| {
                            attempt.preservation["contract"] = json!(if notices.is_empty() {
                                "complete"
                            } else {
                                "incomplete"
                            });
                            if let Some(session) = session_after {
                                attempt.observations["session_after"] = session;
                            }
                            attempt.outputs = outputs;
                            attempt.journal = journal;
                            attempt.notices.extend(notices);
                            Ok(())
                        })
                        .await?;
                    Ok::<_, Error>(())
                }
                .await;
                client.end_preservation();
                match saved {
                    Ok(()) => preserved = true,
                    Err(error) => {
                        let message = error.to_string();
                        let _ = self
                            .attempts
                            .update(&request.id, move |attempt| {
                                if attempt.result.is_none() {
                                    attempt.execution = "interrupted".into();
                                }
                                attempt.preservation["contract"] = json!("incomplete");
                                attempt.notices.push(message);
                                Ok(())
                            })
                            .await;
                    }
                }
            }
            Err(error) => {
                client.end_preservation();
                let message = error.to_string();
                let _ = self
                    .attempts
                    .update(&request.id, move |attempt| {
                        if attempt.execution == "rejected" {
                            return Ok(());
                        }
                        if attempt.execution != "unconfirmed" {
                            attempt.execution = "interrupted".into();
                        }
                        attempt.preservation["contract"] = json!("incomplete");
                        attempt.notices.push(message);
                        Ok(())
                    })
                    .await;
            }
        }
        super::dispatch_transport::revoke(&client.gate, &request.id);
        if self.attempts.get(&request.id).await.is_ok_and(|attempt| {
            attempt
                .result
                .as_ref()
                .is_some_and(|result| result["error"]["code"] == "credential")
                || attempt.observations["refusal"]["reason"] == "not_ready"
        }) {
            // A provider rejection or newly missing requirement changes setup, not just this
            // run's message. Refresh discovery once; never restart or retry paid execution.
            let _ = self.refresh(&request.tool).await;
        }
        let child_exited = client.confirm_exit().await.unwrap_or(false);
        if preserved
            || child_exited
            || self
                .attempts
                .get(&request.id)
                .await
                .is_ok_and(|a| a.execution == "rejected")
        {
            if child_exited {
                let _ = self
                    .attempts
                    .update(&request.id, |attempt| {
                        if attempt.result.is_none() && attempt.execution != "rejected" {
                            attempt.execution = "interrupted".into();
                            attempt.preservation["contract"] = json!("incomplete");
                        }
                        Ok(())
                    })
                    .await;
            }
            self.discard_materialized(&request.id).await;
            // Publish availability only after all old-worker cleanup has finished.
            self.active.lock().await.remove(&request.id);
        }
        self.events.changed();
    }
    async fn record_reference(&self, client: &ToolClient, id: &str) -> Result<()> {
        if let Some(reference) = client.run_reference()? {
            let attempt = self.attempts.get(id).await?;
            super::association::check_reference(&attempt, &reference)?;
            if attempt.observations.get("run_reference") != Some(&reference) {
                self.attempts
                    .update(id, move |attempt| {
                        attempt.observations["run_reference"] = reference;
                        Ok(())
                    })
                    .await?;
            }
        }
        Ok(())
    }
    pub async fn cancel(self: &Arc<Self>, id: &str) -> Result<Attempt> {
        let mut active = self.active.lock().await;
        let Some(running) = active.get_mut(id) else {
            return Ok(self.attempts.get(id).await?);
        };
        if running.cancel_requested {
            return Ok(self.attempts.get(id).await?);
        }
        let attempt = self.attempts.request_cancel(id).await?;
        running.cancel_requested = true;
        let mut written = running.written.clone();
        if attempt.execution == "unconfirmed" {
            let client = running.client.clone();
            let request_id = attempt
                .wire
                .as_ref()
                .and_then(|wire| wire.get("id"))
                .cloned()
                .ok_or_else(|| Error::Invalid("Request identity missing".into()))?;
            let coordinator = self.clone();
            let id = id.to_owned();
            self.workers.lock().await.push(tokio::spawn(async move {
                // A cancellation must not overtake the call while its DB write is pending.
                if written.wait_for(|sent|*sent).await.is_ok() {
                    let sent=async {let params=serde_json::from_value(serde_json::json!({"requestId":request_id,"reason":"Cancelled by the workspace user"}))?;client.peer()?.notify_cancelled(params).await?;Ok::<_,Error>(())}.await;
                    if let Err(error)=sent {let message=format!("Cancellation could not be delivered: {error}");let _=coordinator.attempts.update(&id,move|a|{a.notices.push(message);Ok(())}).await;}
                    coordinator.events.changed();
                }
            }));
        }
        drop(active);
        self.events.changed();
        Ok(attempt)
    }

    async fn reap_exited(&self) {
        let ids = self.active.lock().await.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            let Ok(attempt) = self.attempts.get(&id).await else {
                continue;
            };
            let client = {
                let active = self.active.lock().await;
                let Some(active) = active.get(&id) else {
                    continue;
                };
                // The worker owns cleanup until it returns, even after the child exits.
                if !active.finished.load(Ordering::Acquire) {
                    continue;
                }
                active.client.clone()
            };
            if !client.has_exited().await.unwrap_or(false) {
                continue;
            }
            // A terminal result may still be getting its evidence saved by the worker.
            if attempt.result.is_some() && attempt.preservation["contract"] == "pending" {
                continue;
            }
            if self
                .attempts
                .update(&id, |attempt| {
                    if attempt.result.is_none() && attempt.execution != "rejected" {
                        attempt.execution = "interrupted".into();
                        attempt.preservation["contract"] = json!("incomplete");
                    }
                    Ok(())
                })
                .await
                .is_ok()
            {
                self.discard_materialized(&id).await;
                self.active.lock().await.remove(&id);
                self.events.changed();
            }
        }
    }
    pub async fn try_quit(&self) -> bool {
        let _submission = self.submission.lock().await;
        self.reap_exited().await;
        if !self.active.lock().await.is_empty() {
            return false;
        }
        self.stopping.store(true, Ordering::Release);
        true
    }
    pub async fn cleanup(
        &self,
        selection: crate::library::cleanup::Selection,
        token: String,
    ) -> Result<crate::library::cleanup::Outcome> {
        let _submission = self.submission.lock().await;
        self.reap_exited().await;
        if self.stopping.load(Ordering::Acquire) || !self.active.lock().await.is_empty() {
            return Err(Error::Conflict(
                "Wait until all tool activity has stopped before cleaning the library",
            ));
        }
        if self.native_queue.count() > 0 || !self.artifact_jobs.tools().is_empty() {
            return Err(Error::Conflict(
                "Wait for artifact transfers before cleaning the library",
            ));
        }
        if !self.preparations.is_empty() {
            return Err(Error::Conflict(
                "Discard prepared drafts before cleaning the library",
            ));
        }
        let result = crate::library::cleanup::Cleanup(self.assets.clone())
            .confirm(selection, token)
            .await?;
        self.events.changed();
        Ok(result)
    }
    pub async fn close(&self) -> bool {
        let submission = self.submission.lock().await;
        self.stopping.store(true, Ordering::Release);
        self.preparations.clear();
        let preserving = self.preserving_tools().await;
        let mut stopped = self.close_renderers(false).await && preserving.is_empty();
        for (id, entry) in &self.tools {
            if preserving.contains(id) {
                continue;
            }
            if let Some(client) = entry.client() {
                stopped &= client.close().await.unwrap_or(false);
            }
        }
        // Workers may still need this lock to reject a late preservation request. Once all
        // children are closed, drain them without holding the lock they must acquire to exit.
        drop(submission);
        if stopped {
            let tasks = self.workers.lock().await.drain(..).collect::<Vec<_>>();
            for task in tasks {
                let _ = task.await;
            }
            let ids = self.active.lock().await.keys().cloned().collect::<Vec<_>>();
            for id in ids {
                let _ = self
                    .attempts
                    .update(&id, |attempt| {
                        if attempt.result.is_none() && attempt.execution != "rejected" {
                            attempt.execution = "interrupted".into();
                            attempt.notices.push(
                                "Tool session ended without a confirmed final result.".into(),
                            );
                        }
                        Ok(())
                    })
                    .await;
                self.discard_materialized(&id).await;
            }
            self.active.lock().await.clear();
        }
        stopped
    }
    async fn preserving_tools(&self) -> std::collections::BTreeSet<String> {
        let ids = self.active.lock().await.keys().cloned().collect::<Vec<_>>();
        let mut preserving = self.artifact_jobs.tools();
        preserving.extend(self.native_queue.tools());
        for (id, entry) in &self.tools {
            if entry
                .client()
                .as_ref()
                .is_some_and(|client| client.is_preserving())
            {
                preserving.insert(id.clone());
            }
        }
        for id in ids {
            if let Ok(attempt) = self.attempts.get(&id).await
                && attempt.result.is_some()
                && attempt.preservation["contract"] == "pending"
            {
                preserving.insert(attempt.request.tool);
            }
        }
        preserving
    }
    async fn discard_materialized(&self, id: &str) {
        if let Ok(attempt) = self.attempts.get(id).await
            && let Err(error) = super::materialized::discard(self.assets.root(), &attempt)
        {
            let notice = format!("Disposable input cleanup was incomplete: {error}");
            let _ = self
                .attempts
                .update(id, move |attempt| {
                    attempt.notices.push(notice);
                    Ok(())
                })
                .await;
        }
    }
}

#[cfg(test)]
mod close_tests {
    use super::*;
    #[tokio::test]
    async fn closing_children_does_not_join_workers_while_holding_the_submission_lock() {
        let root = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&root.path().join("Library")).unwrap();
        let coordinator = Arc::new(Coordinator::open(assets, vec![]).await.unwrap());
        let release = Arc::new(tokio::sync::Notify::new());
        let worker = {
            let coordinator = coordinator.clone();
            let release = release.clone();
            tokio::spawn(async move {
                release.notified().await;
                let _submission = coordinator.submission.lock().await;
                assert!(coordinator.stopping.load(Ordering::Acquire));
            })
        };
        coordinator.workers.lock().await.push(worker);
        let closing = {
            let coordinator = coordinator.clone();
            tokio::spawn(async move { coordinator.close().await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !coordinator.stopping.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
            release.notify_one();
            assert!(closing.await.unwrap());
        })
        .await
        .expect("close must release submission before joining its workers");
    }
}
