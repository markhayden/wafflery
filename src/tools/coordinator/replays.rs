//! Bounded guided replay. Ordinary preparation/Run are the only action dispatch paths.
use super::bundle_imports::ImportOptions;
use super::{Coordinator, Error, Result, Submission};
use crate::{
    library::attempts::{InputBinding, ReplayContext},
    tools::{
        bundle_import::{self, ImportJob},
        bundle_reader::ValidatedBundle,
        chain_edges,
        chain_replay::{self, ReplayRun, ReplayStatus},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
pub(super) struct Registry {
    items: StdMutex<BTreeMap<String, Arc<Replay>>>,
    slots: Arc<Semaphore>,
}
impl Default for Registry {
    fn default() -> Self {
        Self {
            items: StdMutex::new(BTreeMap::new()),
            slots: Arc::new(Semaphore::new(2)),
        }
    }
}
struct Replay {
    id: String,
    bundle: Arc<ValidatedBundle>,
    tools: Vec<String>,
    state: Mutex<State>,
    _slot: OwnedSemaphorePermit,
}
struct State {
    staging: bool,
    step: usize,
    generation: u64,
    phase: String,
    attempts: Vec<ReplayRun>,
    completed: BTreeMap<usize, String>,
    active: Option<String>,
    import: Option<Arc<ImportJob>>,
    error: Option<String>,
    cancelled: Arc<AtomicBool>,
}
impl Replay {
    fn snapshot(&self, s: &State) -> ReplayStatus {
        let import = s.import.as_ref().map(|job| job.snapshot());
        let settling = s.staging
            || s.active.is_some()
            || import.as_ref().is_some_and(|job| job.phase == "importing");
        ReplayStatus {
            settling,
            id: self.id.clone(),
            bundle: self.bundle.sha256().into(),
            tools: self.tools.clone(),
            step: s.step,
            total: self.tools.len(),
            generation: s.generation,
            phase: s.phase.clone(),
            attempts: s.attempts.clone(),
            import,
            error: s.error.clone(),
        }
    }
}
impl Coordinator {
    fn replay_entry(&self, id: &str) -> Result<Arc<Replay>> {
        self.replays
            .items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
            .ok_or(Error::Missing(
                "Chain replay is unavailable. Start a new review; saved attempts are unchanged.",
            ))
    }
    async fn owned_replay<T: Send + 'static>(
        &self,
        work: impl Future<Output = Result<T>> + Send + 'static,
    ) -> Result<T> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.track_bundle_import(async move {
            let result = work.await;
            let _ = tx.send(result);
        })
        .await;
        rx.await
            .map_err(|_| Error::Invalid("Chain replay worker stopped".into()))?
    }
    pub async fn start_chain_replay(
        self: &Arc<Self>,
        bundle: Arc<ValidatedBundle>,
        tools: Vec<String>,
    ) -> Result<ReplayStatus> {
        let c = self.clone();
        self.owned_replay(async move {
            let slot = c.replay_slot()?;
            c.start_chain_owned(bundle, tools, slot).await
        })
        .await
    }
    fn replay_slot(&self) -> Result<OwnedSemaphorePermit> {
        self.replays.slots.clone().try_acquire_owned().map_err(|_| {
            Error::Conflict(
                "Two chain reviews are already open. Stop and dismiss one before starting another.",
            )
        })
    }
    pub async fn start_saved_chain_replay(
        self: &Arc<Self>,
        id: &str,
        tools: Vec<String>,
    ) -> Result<ReplayStatus> {
        let c = self.clone();
        let id = id.to_owned();
        self.owned_replay(async move {
            if tools.is_empty() || tools.len() > chain_edges::MAX_STEPS {
                return Err(Error::Invalid(
                    "Select at most 16 receiving installations".into(),
                ));
            }
            let slot = c.replay_slot()?;
            let (file, size) = crate::library::bundle_store::BundleStore(c.assets.clone())
                .open(&id)
                .await?;
            let bundle = tokio::task::spawn_blocking(move || -> Result<_> {
                use std::io::Read;
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(size as usize)
                    .map_err(|_| Error::Invalid("Not enough memory to open this chain".into()))?;
                file.take(crate::library::bundle_archive::MAX_TOTAL + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() as u64 != size {
                    return Err(Error::Invalid(
                        "Saved chain size changed during verification".into(),
                    ));
                }
                let bundle = crate::tools::bundle_reader::read(bytes)?;
                if bundle.sha256() != id {
                    return Err(Error::Invalid(
                        "Saved chain fingerprint changed during verification".into(),
                    ));
                }
                Ok(Arc::new(bundle))
            })
            .await
            .map_err(|_| Error::Invalid("Saved chain review worker stopped".into()))??;
            c.start_chain_owned(bundle, tools, slot).await
        })
        .await
    }
    async fn start_chain_owned(
        &self,
        bundle: Arc<ValidatedBundle>,
        tools: Vec<String>,
        slot: OwnedSemaphorePermit,
    ) -> Result<ReplayStatus> {
        let steps = bundle.manifest()["steps"].as_array().unwrap();
        if steps.is_empty() || steps.len() > chain_edges::MAX_STEPS || tools.len() != steps.len() {
            return Err(Error::Invalid(
                "Select one receiving installation per step, with at most 16 steps".into(),
            ));
        }
        for (step, tool) in steps.iter().zip(&tools) {
            if self.description(tool).await?.document["tool"]["name"] != step["tool"]["name"] {
                return Err(Error::Invalid(
                    "A selected installation belongs to another tool".into(),
                ));
            }
        }
        let proof = bundle.clone();
        tokio::task::spawn_blocking(move || chain_edges::resolve(proof.manifest()))
            .await
            .map_err(|_| Error::Invalid("Chain connection review stopped".into()))??;
        let _submission = self.submission.lock().await;
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        bundle_import::retain(&self.assets, &bundle).await?;
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(std::io::Error::other)?;
        let id = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let state = State {
            staging: false,
            step: 0,
            generation: 0,
            phase: "ready".into(),
            attempts: vec![],
            completed: BTreeMap::new(),
            active: None,
            import: None,
            error: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let replay = Arc::new(Replay {
            id: id.clone(),
            bundle,
            tools,
            state: Mutex::new(state),
            _slot: slot,
        });
        let snapshot = replay.snapshot(&*replay.state.lock().await);
        let mut items = self.replays.items.lock().unwrap_or_else(|e| e.into_inner());
        if items.contains_key(&id) {
            return Err(Error::Conflict(
                "Could not allocate a distinct chain review",
            ));
        }
        items.insert(id, replay);
        self.events.changed();
        Ok(snapshot)
    }
    pub async fn chain_replay(&self, id: &str) -> Result<ReplayStatus> {
        self.reap_exited().await;
        let replay = self.replay_entry(id)?;
        let mut s = replay.state.lock().await;
        if s.phase == "importing"
            && let Some(job) = &s.import
        {
            let p = job.snapshot();
            if p.phase == "completed" {
                s.phase = "awaiting_run".into();
            } else if p.phase == "failed" {
                s.phase = "blocked".into();
                s.error = p.error;
            }
        }
        if let Some(id) = s.active.clone() {
            let attempt = self.attempts.get(&id).await?;
            let busy = self
                .tools
                .get(&attempt.request.tool)
                .is_some_and(|entry| entry.reservation.available_permits() == 0);
            if !busy
                && !matches!(
                    attempt.execution.as_str(),
                    "submitted" | "running" | "cancelling" | "unconfirmed"
                )
                && attempt.preservation["contract"] != "pending"
                && attempt.preservation["native"] != "preserving"
            {
                if let Some(row) = s.attempts.iter_mut().find(|row| row.attempt == id) {
                    row.outcome = attempt.execution.clone();
                }
                if attempt.execution == "completed"
                    && attempt.preservation["contract"] == "complete"
                    && attempt
                        .result
                        .as_ref()
                        .is_some_and(|result| result["status"] == "completed")
                {
                    let step = s.step;
                    s.completed.insert(step, id);
                    s.step += 1;
                    s.active = None;
                    s.import = None;
                    s.error = None;
                    if s.phase != "stopped" || s.step == replay.tools.len() {
                        s.phase = if s.step == replay.tools.len() {
                            "completed"
                        } else {
                            "ready"
                        }
                        .into();
                    }
                } else {
                    s.active = None;
                    if s.phase != "stopped" {
                        s.phase = "blocked".into();
                        s.error=Some("The current step did not complete with verified evidence. Its outcome and spend remain saved. Resume explicitly to review a new attempt.".into());
                    }
                }
            }
        }
        Ok(replay.snapshot(&s))
    }
    pub async fn list_chain_replays(&self) -> Result<Vec<ReplayStatus>> {
        let ids = self
            .replays
            .items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut rows = vec![];
        for id in ids {
            rows.push(self.chain_replay(&id).await?);
        }
        Ok(rows)
    }
    pub async fn stage_chain_replay(
        self: &Arc<Self>,
        id: &str,
        resume: bool,
    ) -> Result<ReplayStatus> {
        let id = id.to_owned();
        let c = self.clone();
        self.owned_replay(async move { c.stage_chain_owned(&id, resume).await })
            .await
    }
    async fn stage_chain_owned(self: &Arc<Self>, id: &str, resume: bool) -> Result<ReplayStatus> {
        self.chain_replay(id).await?;
        let replay = self.replay_entry(id)?;
        let (step, generation, completed, cancelled) = {
            let _submission = self.submission.lock().await;
            let mut s = replay.state.lock().await;
            if self.stopping.load(Ordering::Acquire) {
                return Err(Error::Conflict("The host is shutting down"));
            }
            if s.step == replay.tools.len() {
                return Err(Error::Conflict("This chain is complete"));
            }
            if s.staging
                || s.import
                    .as_ref()
                    .is_some_and(|job| job.snapshot().phase == "importing")
                || s.active.is_some()
            {
                return Err(Error::Conflict(
                    "Wait for the current owned step to stop before preparing another",
                ));
            }
            if s.phase != "ready" && !(resume && matches!(s.phase.as_str(), "blocked" | "stopped"))
            {
                return Err(Error::Conflict(
                    "This chain requires an explicit resume or its current Run review",
                ));
            }
            self.preparations.invalidate_replay(id);
            s.generation += 1;
            s.staging = true;
            s.phase = "preparing".into();
            s.active = None;
            s.import = None;
            s.error = None;
            s.cancelled = Arc::new(AtomicBool::new(false));
            (
                s.step,
                s.generation,
                s.completed.clone(),
                s.cancelled.clone(),
            )
        };
        let work = async {
            let mut attempts = BTreeMap::new();
            for (step, id) in completed {
                attempts.insert(step, self.attempts.get(&id).await?);
            }
            let fresh =
                chain_replay::fresh_inputs(&self.assets, replay.bundle.manifest(), step, &attempts)
                    .await?;
            let tool = &replay.tools[step];
            let mut checked = BTreeSet::new();
            for input in &fresh {
                if let InputBinding::Artifact { reference } = &input.binding
                    && checked.insert(serde_json::to_string(reference)?)
                {
                    if cancelled.load(Ordering::Acquire) {
                        return Err(Error::Conflict("Chain replay was stopped"));
                    }
                    self.inspect_reference_at(reference, tool).await?;
                }
            }
            let context = ReplayContext {
                id: id.into(),
                generation,
            };
            self.start_bundle_import_rebound(
                replay.bundle.clone(),
                step,
                tool.clone(),
                ImportOptions {
                    fresh,
                    cancelled: cancelled.clone(),
                    replay: Some(context),
                },
            )
            .await
        }
        .await;
        {
            let mut s = replay.state.lock().await;
            s.staging = false;
            match work {
                Ok(job) => {
                    // Keep the import owner visible even if Stop won while staging its inputs.
                    s.import = Some(job);
                    if s.generation == generation && !cancelled.load(Ordering::Acquire) {
                        s.phase = "importing".into();
                    }
                }
                Err(error) if s.generation == generation && !cancelled.load(Ordering::Acquire) => {
                    s.phase = "blocked".into();
                    s.error = Some(error.to_string());
                }
                Err(_) => (),
            }
        }
        self.events.changed();
        self.chain_replay(id).await
    }
    pub async fn stop_chain_replay(self: &Arc<Self>, id: &str) -> Result<ReplayStatus> {
        let replay = self.replay_entry(id)?;
        let active = {
            let _submission = self.submission.lock().await;
            let mut s = replay.state.lock().await;
            if s.phase == "completed" {
                return Ok(replay.snapshot(&s));
            }
            self.preparations.invalidate_replay(id);
            s.cancelled.store(true, Ordering::Release);
            s.generation += 1;
            s.phase = "stopped".into();
            s.active.clone()
        };
        if let Some(id) = active {
            self.cancel(&id).await?;
        }
        self.events.changed();
        self.chain_replay(id).await
    }
    pub async fn dismiss_chain_replay(&self, id: &str) -> Result<()> {
        self.chain_replay(id).await?;
        let _submission = self.submission.lock().await;
        let replay = self.replay_entry(id)?;
        let s = replay.state.lock().await;
        if s.staging
            || !matches!(s.phase.as_str(), "stopped" | "completed")
            || s.import
                .as_ref()
                .is_some_and(|job| job.snapshot().phase == "importing")
            || s.active.is_some()
        {
            return Err(Error::Conflict(
                "Stop the chain and wait for its owned work before dismissing it",
            ));
        }
        self.replays
            .items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        self.events.changed();
        Ok(())
    }
    pub(super) async fn validate_chain_request(&self, request: &Submission) -> Result<()> {
        let Some(source) = &request.source_bundle else {
            return Ok(());
        };
        let Some(context) = &source.replay else {
            return Ok(());
        };
        let replay = self.replay_entry(&context.id)?;
        let s = replay.state.lock().await;
        if s.phase != "awaiting_run"
            || s.cancelled.load(Ordering::Acquire)
            || s.generation != context.generation
            || s.step != source.step
            || replay.bundle.sha256() != source.id
            || replay.tools[s.step] != request.tool
        {
            return Err(Error::Conflict(
                "This chain draft is no longer current. Reopen its current step; no action was started.",
            ));
        }
        let progress = s
            .import
            .as_ref()
            .ok_or(Error::Missing("Chain input review is unavailable"))?
            .snapshot();
        if progress.phase != "completed" || progress.action != request.action {
            return Err(Error::Conflict(
                "Wait for chain inputs to finish before Run",
            ));
        }
        // User-reviewed many-port edits may reorder inputs. Require a distinct fresh occurrence
        // for every connected edge, while preserving the caller's actual order and lineage.
        let mut ports = BTreeMap::<String, BTreeMap<String, usize>>::new();
        for input in progress
            .inputs
            .iter()
            .filter(|input| input.source.is_some())
        {
            if !ports.contains_key(&input.name) {
                let bindings = request
                    .bindings
                    .get(&input.name)
                    .cloned()
                    .unwrap_or_else(|| {
                        request
                            .images
                            .get(&input.name)
                            .into_iter()
                            .flatten()
                            .enumerate()
                            .map(|(index, asset)| InputBinding::Image {
                                asset: asset.clone(),
                                frame: request
                                    .frames
                                    .get(&input.name)
                                    .and_then(|frames| frames.get(index))
                                    .cloned()
                                    .unwrap_or_default(),
                            })
                            .collect()
                    });
                let mut counts = BTreeMap::new();
                for (index, binding) in bindings.iter().enumerate() {
                    if let Some(source) = request
                        .lineage
                        .get(&input.name)
                        .and_then(|items| items.get(index))
                        .and_then(Option::as_ref)
                    {
                        *counts
                            .entry(serde_json::to_string(&(binding, source))?)
                            .or_insert(0usize) += 1;
                    }
                }
                ports.insert(input.name.clone(), counts);
            }
            let binding = input.binding.as_ref().ok_or(Error::Conflict(
                "A connected chain input has not finished staging",
            ))?;
            let key = serde_json::to_string(&(binding, input.source.as_ref().unwrap()))?;
            let count=ports.get_mut(&input.name).unwrap().get_mut(&key).filter(|count|**count>0).ok_or(Error::Conflict("A connected chain input changed. Reopen the step to use its fresh producer output, or leave the chain for an independent experiment."))?;
            *count -= 1;
        }
        Ok(())
    }
    pub(super) async fn claim_chain_request(&self, request: &Submission) -> Result<()> {
        if let Some(context) = request
            .source_bundle
            .as_ref()
            .and_then(|source| source.replay.as_ref())
        {
            let replay = self.replay_entry(&context.id)?;
            let mut s = replay.state.lock().await;
            self.preparations.invalidate_replay(&context.id);
            s.phase = "running".into();
            s.active = Some(request.id.clone());
            let step = s.step;
            s.attempts.push(ReplayRun {
                step,
                attempt: request.id.clone(),
                outcome: "running".into(),
            });
            self.events.changed();
        }
        Ok(())
    }
}
