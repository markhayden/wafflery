use super::{Coordinator, Error, Result, artifact_controls::Source};
use crate::{
    library::{
        attempts::{ImageFrame, InputBinding},
        blob_links::Owner,
        bundle_store::BundleStore,
    },
    tools::{
        bundle_import::{self, ImportJob, ImportedInput, Progress},
        bundle_reader::ValidatedBundle,
        bundle_step::{self, Input},
    },
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub(super) struct ImportOptions {
    pub fresh: Vec<crate::tools::chain_replay::FreshInput>,
    pub cancelled: Arc<AtomicBool>,
    pub replay: Option<crate::library::attempts::ReplayContext>,
}
impl Coordinator {
    pub(super) async fn validate_bundle_source(
        &self,
        request: &crate::library::attempts::Submission,
        description: &Value,
    ) -> Result<()> {
        let Some(source) = &request.source_bundle else {
            return Ok(());
        };
        let saved = BundleStore(self.assets.clone()).get(&source.id).await?;
        let step = saved.manifest["steps"]
            .get(source.step)
            .ok_or(Error::Missing("Source bundle step is unavailable"))?;
        if step["tool"]["name"] != description["tool"]["name"]
            || step["invocation"]["action"] != request.action
        {
            return Err(Error::Invalid(
                "Source bundle step does not match this tool and action".into(),
            ));
        }
        self.validate_chain_request(request).await
    }
    /// Explicit controls only. The owner survives an HTTP waiter or drawer being dropped.
    pub async fn start_bundle_import(
        self: &Arc<Self>,
        bundle: Arc<ValidatedBundle>,
        step: usize,
        tool: String,
    ) -> Result<Arc<ImportJob>> {
        self.start_bundle_import_rebound(bundle, step, tool, ImportOptions::default())
            .await
    }
    pub(super) async fn start_bundle_import_rebound(
        self: &Arc<Self>,
        bundle: Arc<ValidatedBundle>,
        step: usize,
        tool: String,
        options: ImportOptions,
    ) -> Result<Arc<ImportJob>> {
        let _submission = self.submission.lock().await;
        if options.cancelled.load(Ordering::Acquire) {
            return Err(Error::Conflict(
                "Chain replay was stopped; no more inputs were started",
            ));
        }
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        let declaration = self.description(&tool).await?;
        let mut plan =
            bundle_step::prepare_rebound(&bundle, step, &declaration.document, &options.fresh)?;
        if options.replay.is_some()
            && declaration.document["actions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|action| action["name"] == plan.action && action["cost"]["may_spend"] == true)
        {
            plan.params.as_object_mut().unwrap().remove("max_spend_usd");
        }
        let needs_files = plan
            .inputs
            .iter()
            .any(|input| matches!(input.input, Input::Artifact { .. }));
        if needs_files {
            if !declaration.schemas.contains_key("import") {
                return Err(Error::Invalid(
                    "The receiving tool does not advertise artifact import".into(),
                ));
            }
            let staging = self.assets.root().join("execution").canonicalize()?;
            if !declaration.tool_session["allowed_dirs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter_map(|p| std::path::Path::new(p).canonicalize().ok())
                .any(|allowed| staging.starts_with(allowed))
            {
                return Err(Error::Invalid("The tool's effective file access does not allow import staging. Update its explicit grants in Tool setup.".into()));
            }
        }
        let group = self.artifact_jobs.group(&tool);
        let job = Arc::new(ImportJob(RwLock::new(Progress {
            replay: options.replay,
            phase: "importing".into(),
            bundle: bundle.sha256().into(),
            step,
            tool: tool.clone(),
            action: plan.action,
            params: plan.params,
            inputs: plan
                .inputs
                .iter()
                .map(|input| ImportedInput {
                    source: match &input.input {
                        Input::Fresh { source, .. } => Some(source.clone()),
                        _ => None,
                    },
                    changed: matches!(&input.input, Input::Fresh { changed: true, .. }),
                    name: input.name.clone(),
                    index: input.index,
                    status: if matches!(input.input, Input::Unresolved { .. }) {
                        "unresolved"
                    } else {
                        "pending"
                    }
                    .into(),
                    binding: None,
                    reason: match &input.input {
                        Input::Unresolved { reason } => Some(reason.clone()),
                        _ => None,
                    },
                })
                .collect(),
            error: None,
        })));
        let coordinator = self.clone();
        let progress = job.clone();
        self.track_bundle_import(async move {
            let _group = group;
            let result = coordinator
                .import_bundle_owned(&bundle, &tool, plan.inputs, &progress, &options.cancelled)
                .await;
            progress.update(|state| match result {
                Ok(()) => state.phase = "completed".into(),
                Err(error) => {
                    state.phase = "failed".into();
                    state.error = Some(match &error {
                        Error::ArtifactControlRefusal(data) => data["error"]["reason"]
                            .as_str()
                            .unwrap_or("The tool refused an artifact import")
                            .into(),
                        _ => error.to_string(),
                    });
                    if let Some(input) = state
                        .inputs
                        .iter_mut()
                        .find(|input| input.status == "importing")
                    {
                        input.status = "failed".into();
                    }
                }
            });
            coordinator.events.changed();
        })
        .await;
        Ok(job)
    }
    pub(super) async fn track_bundle_import(
        &self,
        work: impl std::future::Future<Output = ()> + Send + 'static,
    ) {
        let mut workers = self.workers.lock().await;
        workers.retain(|worker| !worker.is_finished());
        // No cancellation point can separate starting work from publishing its shutdown owner.
        workers.push(tokio::spawn(work));
    }
    fn check_import_lifetime(&self) -> Result<()> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict(
                "The host is shutting down; remaining inputs were not started",
            ));
        }
        Ok(())
    }
    async fn import_bundle_owned(
        &self,
        bundle: &ValidatedBundle,
        tool: &str,
        inputs: Vec<bundle_step::InputClaim>,
        progress: &ImportJob,
        cancelled: &Arc<AtomicBool>,
    ) -> Result<()> {
        self.check_import_lifetime()?;
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::Conflict(
                "Chain replay was stopped; remaining inputs were not started",
            ));
        }
        bundle_import::retain(&self.assets, bundle).await?;
        let mut imported = BTreeMap::<String, InputBinding>::new();
        for (ordinal, input) in inputs.into_iter().enumerate() {
            self.check_import_lifetime()?;
            if cancelled.load(Ordering::Acquire) {
                return Err(Error::Conflict(
                    "Chain replay was stopped; remaining inputs were not started",
                ));
            }
            if matches!(input.input, Input::Unresolved { .. }) {
                continue;
            }
            progress.update(|state| state.inputs[ordinal].status = "importing".into());
            self.events.changed();
            let binding = match input.input {
                Input::Image { member, image } => {
                    let verified = bundle.clone();
                    let root = self.assets.root().join("staging");
                    let file = tokio::task::spawn_blocking(move || -> Result<_> {
                        use std::io::Write;
                        let bytes = verified
                            .member(&member)
                            .ok_or(Error::Missing("Bundle image member is unavailable"))?;
                        let mut file = tempfile::NamedTempFile::new_in(root)?;
                        file.write_all(bytes)?;
                        Ok(file)
                    })
                    .await
                    .map_err(|_| Error::Invalid("Image staging worker stopped".into()))??;
                    let asset = self.assets.import_path(file.path().into()).await?;
                    if asset.sha256 != image["file_sha256"]
                        || asset.width != image["width"]
                        || asset.height != image["height"]
                    {
                        return Err(Error::Invalid(
                            "Imported image conflicts with its verified archive header".into(),
                        ));
                    }
                    BundleStore(self.assets.clone())
                        .attach_input(bundle.sha256(), &asset.id)
                        .await?;
                    InputBinding::Image {
                        asset: asset.id,
                        frame: serde_json::from_value::<ImageFrame>(
                            json!({"projection":image["projection"],"topology":image["topology"]}),
                        )?,
                    }
                }
                Input::Fresh { binding, .. } => binding,
                Input::Location { value } => InputBinding::Location { value },
                Input::Artifact { member, reference } => {
                    let key = format!("{}:{member}", serde_json::to_string(&reference)?);
                    if let Some(binding) = imported.get(&key) {
                        binding.clone()
                    } else {
                        let slot = self.artifact_jobs.reserve().await?;
                        self.check_import_lifetime()?;
                        if cancelled.load(Ordering::Acquire) {
                            return Err(Error::Conflict(
                                "Chain replay was stopped; remaining inputs were not started",
                            ));
                        }
                        let context = self
                            .artifact_context_from(
                                Source {
                                    owner: Owner::Bundle(bundle.sha256().into()),
                                    direction: "bundle".into(),
                                    index: ordinal as u32,
                                    reference: reference.clone(),
                                    registration: tool.into(),
                                    run_dir: None,
                                },
                                "import",
                                Some(slot),
                                Some(tool),
                            )
                            .await?;
                        let bytes = bundle
                            .member(&member)
                            .ok_or(Error::Missing("Bundle native member is unavailable"))?
                            .to_vec();
                        let response = self
                            .import_bytes_owned(context, bytes, None, cancelled.clone())
                            .await?;
                        self.inspect_reference_at(&response["artifact"], tool)
                            .await?;
                        let binding = InputBinding::Artifact {
                            reference: response["artifact"].clone(),
                        };
                        imported.insert(key, binding.clone());
                        binding
                    }
                }
                Input::Unresolved { .. } => unreachable!(),
            };
            progress.update(|state| {
                state.inputs[ordinal].status = "completed".into();
                state.inputs[ordinal].binding = Some(binding);
            });
            self.events.changed();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cancellation_while_registering_a_worker_cannot_start_untracked_imports() {
        let root = tempfile::tempdir().unwrap();
        let assets =
            crate::library::assets::AssetStore::open(&root.path().join("Library")).unwrap();
        let coordinator = Arc::new(Coordinator::open(assets, vec![]).await.unwrap());
        let workers = coordinator.workers.lock().await;
        let entered = Arc::new(tokio::sync::Notify::new());
        let ran = Arc::new(AtomicBool::new(false));
        let starting = {
            let c = coordinator.clone();
            let entered = entered.clone();
            let ran = ran.clone();
            tokio::spawn(async move {
                entered.notify_one();
                c.track_bundle_import(async move {
                    ran.store(true, Ordering::Release);
                })
                .await;
            })
        };
        entered.notified().await;
        // Poll the registering future through its first blocked await; no wall-clock timing.
        tokio::task::yield_now().await;
        starting.abort();
        assert!(starting.await.unwrap_err().is_cancelled());
        drop(workers);
        tokio::task::yield_now().await;
        assert!(
            !ran.load(Ordering::Acquire),
            "work ran before its shutdown owner was registered"
        );
        assert!(coordinator.workers.lock().await.is_empty());
        assert!(coordinator.close().await);
        coordinator.assets.database.shutdown().await.unwrap();
    }
}
