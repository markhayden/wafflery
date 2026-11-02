use super::{Coordinator, Error, Result};
use crate::tools::render::{
    RenderKind, ResolvedRender,
    document::DocumentGrant,
    document_monitor::DocumentMonitor,
    idle::IdleLease,
    process::{ProcessPhase, RenderProcess},
    readiness::{Readiness, ServerReadiness},
    supervision::ServerMonitor,
};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;

pub(super) struct View {
    id: String,
    attempt: String,
    name: String,
    kind: RenderKind,
    process: RenderProcess,
    server: Option<ServerMonitor>,
    document: Option<DocumentMonitor>,
    idle: IdleLease,
}
impl View {
    fn observation(&self) -> Value {
        let state = self.process.snapshot();
        let document = self.document.as_ref().map(DocumentMonitor::snapshot);
        let server = self.server.as_ref().map(ServerMonitor::snapshot);
        let readiness = document
            .as_ref()
            .map(|view| &view.readiness)
            .or(server.as_ref());
        let error = match readiness {
            Some(Readiness::Failed(reason)) => Some((*reason).to_owned()),
            _ => state.error.clone().or_else(|| {
                (self.server.is_some()
                    && state.phase != ProcessPhase::Running
                    && !state.close_requested)
                    .then(|| "Report process is not running".to_owned())
            }),
        };
        let phase = match state.phase {
            ProcessPhase::NeedsForce => "needs_force",
            ProcessPhase::Exited if document.as_ref().is_none_or(|view| view.closed) => "exited",
            _ if state.phase == ProcessPhase::Stopping
                || state.close_requested
                || self.idle.expired()
                || document.as_ref().is_some_and(|view| view.closing) =>
            {
                "stopping"
            }
            _ if matches!(readiness, Some(Readiness::Ready(_))) => "ready",
            _ => "starting",
        };
        json!({"id":self.id,"attempt":self.attempt,"name":self.name,
   "kind":match self.kind {RenderKind::Server=>"server",RenderKind::Document=>"document"},
   "phase":phase,
   "exit_code":state.exit_code,"error":error,"idle_expired":self.idle.expired(),
   "diagnostics":String::from_utf8_lossy(&state.stderr.bytes),"diagnostics_truncated":state.stderr.overflow})
    }
    fn stopped(&self) -> bool {
        self.process.snapshot().phase == ProcessPhase::Exited
            && self
                .document
                .as_ref()
                .is_none_or(|view| view.snapshot().closed)
    }
    fn accepts_activity(&self) -> bool {
        let process = self.process.snapshot();
        if process.close_requested || self.idle.expired() {
            return false;
        }
        if let Some(document) = &self.document {
            let view = document.snapshot();
            !view.closing
                && !view.closed
                && !matches!(view.readiness, Readiness::Failed(_))
                && matches!(process.phase, ProcessPhase::Running | ProcessPhase::Exited)
        } else {
            process.phase == ProcessPhase::Running
        }
    }
}
async fn close_owned_view(
    process: RenderProcess,
    document: Option<DocumentMonitor>,
    force: bool,
) -> Result<bool> {
    if let Some(document) = document {
        document.close(force).await
    } else if force {
        process.force_stop().await
    } else {
        process.close().await
    }
}
impl Coordinator {
    pub async fn open_render(&self, attempt_id: &str, name: &str, origin: &str) -> Result<Value> {
        // Linearize shutdown and opens; store ownership before returning to a cancellable requester.
        let _submission = self.submission.lock().await;
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("Workspace is stopping"));
        }
        let attempt = self.attempts.get(attempt_id).await?;
        let entry = self
            .tools
            .get(&attempt.request.tool)
            .ok_or(Error::Missing("Tool is not registered"))?;
        let declaration = self.description(&attempt.request.tool).await?;
        if declaration.document["tool"]["name"] != attempt.declaration["document"]["tool"]["name"] {
            return Err(Error::Invalid(
                "Saved run belongs to a different tool installation".into(),
            ));
        }
        let client = entry
            .client()
            .ok_or(Error::Missing("Serving tool is unavailable"))?;
        client.peer()?;
        let mut views = self.renders.lock().await;
        if let Some(view) = views.get(attempt_id)
            && !view.stopped()
        {
            return if view.name == name && view.accepts_activity() {
                if view.idle.touch() {
                    Ok(view.observation())
                } else {
                    Err(Error::Conflict(
                        "Report expired; wait for it to close before reopening",
                    ))
                }
            } else {
                Err(Error::Conflict(
                    "Close the existing report and confirm its exit before opening another",
                ))
            };
        }
        if views.len() >= 16 {
            views.retain(|_, view| !view.stopped());
            if views.len() >= 16 {
                return Err(Error::Conflict("At most 16 report views may remain open"));
            }
        }
        let plan = ResolvedRender::resolve(&client.descriptor, &attempt, name, origin)?;
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|error| Error::Invalid(error.to_string()))?;
        let id = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let validator = if plan.kind() == RenderKind::Server {
            let timeout = plan.ready_timeout().ok_or_else(|| {
                Error::Invalid("Server report requires a readiness deadline".into())
            })?;
            Some(ServerReadiness::new(origin, timeout)?)
        } else {
            None
        };
        let grant = if plan.kind() == RenderKind::Document {
            Some(DocumentGrant::new(plan.run_dir())?)
        } else {
            None
        };
        let process = RenderProcess::spawn(&plan)?;
        let server = validator.map(|validator| ServerMonitor::start(process.clone(), validator));
        let (document, idle) = if let Some(grant) = grant {
            let idle = IdleLease::document();
            (
                Some(DocumentMonitor::start(
                    process.clone(),
                    grant,
                    origin,
                    idle.clone(),
                )),
                idle,
            )
        } else {
            (None, IdleLease::attach(process.clone()))
        };
        let view = View {
            id,
            attempt: attempt_id.into(),
            name: name.into(),
            kind: plan.kind(),
            process,
            server,
            document,
            idle,
        };
        let observation = view.observation();
        views.insert(attempt_id.into(), view);
        Ok(observation)
    }
    pub async fn render_state(&self, id: &str) -> Result<Value> {
        self.renders
            .lock()
            .await
            .values()
            .find(|view| view.id == id)
            .map(View::observation)
            .ok_or(Error::Missing("Report view is no longer available"))
    }
    pub async fn render_activity(&self, id: &str) -> Result<Value> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("Workspace is stopping"));
        }
        let views = self.renders.lock().await;
        let view = views
            .values()
            .find(|view| view.id == id)
            .ok_or(Error::Missing("Report view is no longer available"))?;
        if !view.accepts_activity() || !view.idle.touch() {
            return Err(Error::Conflict("Report is closing or expired"));
        }
        Ok(view.observation())
    }
    pub async fn close_render(&self, id: &str, force: bool, confirmed: bool) -> Result<Value> {
        if force && !confirmed {
            return Err(Error::Conflict("Confirm force-closing this report"));
        }
        let (process, document) = self
            .renders
            .lock()
            .await
            .values()
            .find(|view| view.id == id)
            .map(|view| (view.process.clone(), view.document.clone()))
            .ok_or(Error::Missing("Report view is no longer available"))?;
        close_owned_view(process, document, force).await?;
        self.render_state(id).await
    }
    pub(super) async fn close_renderers(&self, force: bool) -> bool {
        let processes = self
            .renders
            .lock()
            .await
            .values()
            .map(|view| (view.process.clone(), view.document.clone()))
            .collect::<Vec<_>>();
        futures_util::future::join_all(processes.into_iter().map(
            |(process, document)| async move {
                close_owned_view(process, document, force)
                    .await
                    .unwrap_or(false)
            },
        ))
        .await
        .into_iter()
        .all(|stopped| stopped)
    }
    pub(super) async fn renderers_stopped(&self) -> bool {
        self.renders.lock().await.values().all(View::stopped)
    }
}
