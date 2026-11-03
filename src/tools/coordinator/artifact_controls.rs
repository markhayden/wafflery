use super::{Coordinator, Declaration, Error, Result};
use crate::{
    library::{
        blob_links::{BlobLink, Owner},
        blobs::BlobStore,
    },
    tools::{
        artifact_io,
        artifact_jobs::{Job, Waiter},
        client::ToolClient,
        validate_contract, validator,
    },
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
struct Context {
    owner: String,
    direction: String,
    index: u32,
    reference: Value,
    portable: String,
    declaration: Declaration,
    client: Arc<ToolClient>,
    run_dir: Option<PathBuf>,
    _job: Job,
}
impl Coordinator {
    async fn artifact_context(
        &self,
        id: &str,
        direction: &str,
        index: usize,
        control: &str,
        slot: Option<tokio::sync::OwnedSemaphorePermit>,
        target: Option<&str>,
    ) -> Result<Context> {
        let _submission = self.submission.lock().await;
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        if !matches!(direction, "in" | "out") {
            return Err(Error::Invalid("Unknown artifact direction".into()));
        }
        let saved = self.attempts.get(id).await?;
        let result = saved
            .result
            .as_ref()
            .ok_or(Error::Missing("Saved artifact result is unavailable"))?;
        let reference = result["artifacts"][direction]
            .get(index)
            .map(|entry| entry["artifact"].clone())
            .ok_or(Error::Missing("Saved artifact occurrence is unavailable"))?;
        let tool = target.unwrap_or(&saved.request.tool);
        let entry = self
            .tools
            .get(tool)
            .ok_or(Error::Missing("Artifact tool is not registered"))?;
        let client = entry
            .client()
            .ok_or(Error::Missing("Artifact tool is unavailable"))?;
        let declaration = self.description(tool).await?;
        super::artifacts::validate_reference(&reference, &declaration.document)?;
        if client.generation() != declaration.generation {
            return Err(Error::Conflict(
                "Tool declaration changed. Refresh before transferring artifacts.",
            ));
        }
        let portable = declaration.document["artifact_types"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|kind| kind["type"] == reference["type"])
            .and_then(|kind| kind["portable"].as_str())
            .filter(|value| matches!(*value, "json" | "bundle"))
            .ok_or(Error::Invalid(
                "This artifact type has no declared portable native form".into(),
            ))?
            .to_owned();
        if !declaration.schemas.contains_key(control) {
            return Err(Error::Invalid("Artifact control is not advertised".into()));
        }
        let job = match slot {
            Some(slot) => self.artifact_jobs.start_reserved(tool, slot),
            None => self.artifact_jobs.start(tool)?,
        };
        Ok(Context {
            owner: id.into(),
            direction: direction.into(),
            index: u32::try_from(index)
                .map_err(|_| Error::Invalid("Artifact occurrence exceeds the host limit".into()))?,
            reference,
            portable,
            declaration,
            client,
            run_dir: result["run"]["dir"].as_str().map(PathBuf::from),
            _job: job,
        })
    }
    pub(super) async fn preserve_artifact(
        &self,
        id: &str,
        direction: &str,
        index: usize,
    ) -> Result<BlobLink> {
        let slot = self.artifact_jobs.reserve().await?;
        let context = self
            .artifact_context(id, direction, index, "export", Some(slot), None)
            .await?;
        self.export_artifact_owned(context, Arc::new(AtomicBool::new(false)))
            .await
    }
    pub async fn export_artifact(
        self: &Arc<Self>,
        id: &str,
        direction: &str,
        index: usize,
    ) -> Result<BlobLink> {
        let context = self
            .artifact_context(id, direction, index, "export", None, None)
            .await?;
        let waiter = Waiter::new();
        let cancelled = waiter.0.clone();
        let coordinator = self.clone();
        let worker =
            tokio::spawn(
                async move { coordinator.export_artifact_owned(context, cancelled).await },
            );
        let result = tokio::time::timeout(Duration::from_secs(10), worker).await?;
        result
            .map_err(|error| Error::Invalid(format!("Artifact export worker stopped: {error}")))?
    }
    async fn export_artifact_owned(
        &self,
        context: Context,
        cancelled: Arc<AtomicBool>,
    ) -> Result<BlobLink> {
        let arguments = json!({"artifact":context.reference});
        validate_input(&context, "export", &arguments)?;
        let response = artifact_io::call(&context.client, "export", arguments).await?;
        validate_control_response(&context, "export-response", &response)?;
        if context.portable == "json" && response["file"]["media_type"] != "application/json" {
            return Err(Error::Invalid(
                "A JSON artifact export must declare application/json".into(),
            ));
        }
        let file = response["file"].clone();
        let roots = context.client.descriptor.evidence_dirs.clone();
        let run_dir = context.run_dir.clone();
        let bytes = tokio::task::spawn_blocking(move || {
            artifact_io::read_export(&file, &roots, run_dir.as_deref())
        })
        .await
        .map_err(|_| Error::Invalid("Native export reader stopped".into()))??;
        check_publication(&context, &cancelled)?;
        let link = BlobLink {
            direction: context.direction.clone(),
            occurrence: context.index,
            sha256: response["file"]["sha256"].as_str().unwrap().into(),
            media_type: response["file"]["media_type"].as_str().unwrap().into(),
            artifact: context.reference.clone(),
        };
        let blobs = BlobStore(self.assets.clone());
        let _lease = self.assets.storage.read().await;
        blobs.publish_leased(bytes, link.sha256.clone()).await?;
        check_publication(&context, &cancelled)?;
        blobs
            .attach_leased(Owner::Attempt(context.owner.clone()), link.clone())
            .await?;
        self.events.changed();
        Ok(link)
    }
    pub async fn import_artifact(
        self: &Arc<Self>,
        id: &str,
        direction: &str,
        index: usize,
    ) -> Result<Value> {
        self.import_artifact_into(id, direction, index, None).await
    }
    pub async fn import_artifact_into(
        self: &Arc<Self>,
        id: &str,
        direction: &str,
        index: usize,
        target: Option<&str>,
    ) -> Result<Value> {
        let context = self
            .artifact_context(id, direction, index, "import", None, target)
            .await?;
        let waiter = Waiter::new();
        let cancelled = waiter.0.clone();
        let coordinator = self.clone();
        let worker =
            tokio::spawn(
                async move { coordinator.import_artifact_owned(context, cancelled).await },
            );
        let result = tokio::time::timeout(Duration::from_secs(10), worker).await?;
        result
            .map_err(|error| Error::Invalid(format!("Artifact import worker stopped: {error}")))?
    }
    async fn import_artifact_owned(
        &self,
        context: Context,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Value> {
        let blobs = BlobStore(self.assets.clone());
        let link=blobs.links(Owner::Attempt(context.owner.clone())).await?.into_iter().find(|link|link.direction==context.direction&&link.occurrence==context.index&&link.artifact==context.reference).ok_or(Error::Missing("No owned native export is available for this artifact. Export it before importing."))?;
        let bytes = blobs.read(&link.sha256).await?;
        let inline_limit = context.declaration.document["limits"]["inline_bytes"]
            .as_u64()
            .unwrap_or(0)
            .min(1024 * 1024);
        let mut staging = None;
        let arguments = if bytes.len() as u64 <= inline_limit {
            json!({"inline":{"media_type":link.media_type,"data":STANDARD.encode(&bytes)},"sha256":link.sha256})
        } else {
            let root = self.assets.root().join("execution").canonicalize()?;
            if !context.declaration.tool_session["allowed_dirs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .filter_map(|allowed| PathBuf::from(allowed).canonicalize().ok())
                .any(|allowed| root.starts_with(allowed))
            {
                return Err(Error::Invalid(
                    "The tool's effective file access does not allow import staging".into(),
                ));
            }
            let directory = tempfile::Builder::new()
                .prefix("artifact-import-")
                .tempdir_in(root)?;
            let path = directory.path().join("artifact.native");
            tokio::fs::write(&path, &bytes).await?;
            staging = Some(directory);
            json!({"file":{"path":path},"sha256":link.sha256})
        };
        validate_input(&context, "import", &arguments)?;
        check_publication(&context, &cancelled)?;
        // The owned worker retains staging until a terminal RPC response or confirmed child exit.
        let response = artifact_io::call(&context.client, "import", arguments).await?;
        drop(staging);
        validate_control_response(&context, "import-response", &response)?;
        if cancelled.load(Ordering::Acquire) {
            return Err(Error::Conflict(
                "Import may have completed. Retry explicitly to reconcile by identity.",
            ));
        }
        if context.client.generation() != context.declaration.generation {
            return Err(Error::Conflict(
                "Tool declaration changed during import. The import may have completed; reconcile explicitly.",
            ));
        }
        Ok(response)
    }
}
fn validate_input(context: &Context, control: &str, arguments: &Value) -> Result<()> {
    validator(&json!({"$ref":format!("urn:table-manners:operations:{control}-request")}))?
        .validate(arguments)
        .map_err(|error| Error::Invalid(format!("Invalid artifact request: {error}")))?;
    validator(&context.declaration.schemas[control])?
        .validate(arguments)
        .map_err(|error| {
            Error::Invalid(format!(
                "Artifact request conflicts with the advertised control: {error}"
            ))
        })?;
    Ok(())
}
fn validate_control_response(context: &Context, kind: &str, response: &Value) -> Result<()> {
    validate_contract(&format!("urn:table-manners:operations:{kind}"), response)?;
    if response["artifact"] != context.reference
        || response["tool"]["name"] != context.declaration.document["tool"]["name"]
        || response["tool"]["version"] != context.declaration.document["tool"]["version"]
    {
        return Err(Error::Invalid(
            "Artifact control response conflicts with its requested reference or tool".into(),
        ));
    }
    super::artifacts::validate_reference(&response["artifact"], &context.declaration.document)?;
    Ok(())
}
fn check_publication(context: &Context, cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Acquire) {
        return Err(Error::Conflict(
            "Artifact transfer stopped before publication",
        ));
    }
    if context.client.generation() != context.declaration.generation {
        return Err(Error::Conflict(
            "Artifact declaration changed during transfer",
        ));
    }
    Ok(())
}
