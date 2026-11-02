//! Own document generation, immutable reading and the content server as one view.
use super::{
    content::{ContentState, DocumentServer},
    document::DocumentGrant,
    idle::IdleLease,
    process::{ProcessPhase, RenderProcess},
    readiness::Readiness,
};
use crate::tools::{Error, Result};
use std::time::Duration;
use tokio::sync::watch;
const GENERATION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct DocumentState {
    pub readiness: Readiness,
    pub closing: bool,
    pub closed: bool,
    content: Option<watch::Receiver<ContentState>>,
}
#[derive(Clone)]
pub struct DocumentMonitor {
    process: RenderProcess,
    stop: watch::Sender<bool>,
    state: watch::Receiver<DocumentState>,
}
impl DocumentMonitor {
    pub fn start(
        process: RenderProcess,
        grant: DocumentGrant,
        origin: &str,
        idle: IdleLease,
    ) -> Self {
        Self::start_with_timeout(process, grant, origin, idle, GENERATION_TIMEOUT)
    }
    fn start_with_timeout(
        process: RenderProcess,
        grant: DocumentGrant,
        origin: &str,
        idle: IdleLease,
        timeout: Duration,
    ) -> Self {
        let (state, observed) = watch::channel(DocumentState {
            readiness: Readiness::Waiting,
            closing: false,
            closed: false,
            content: None,
        });
        let (stop, receiver) = watch::channel(false);
        tokio::spawn(own(
            process.clone(),
            grant,
            origin.to_owned(),
            idle,
            timeout,
            receiver,
            state,
        ));
        Self {
            process,
            stop,
            state: observed,
        }
    }
    pub fn snapshot(&self) -> DocumentState {
        let mut state = self.state.borrow().clone();
        state.closing |= *self.stop.borrow();
        state
    }
    pub fn current_url(&self) -> Result<url::Url> {
        let state = self.snapshot();
        if state.closing || state.closed {
            return Err(Error::Conflict("Document view is closing"));
        }
        let Readiness::Ready(url) = state.readiness else {
            return Err(Error::Conflict("Document view is not ready"));
        };
        let Some(content) = state.content else {
            return Err(Error::Conflict("Document content is unavailable"));
        };
        let content = content.borrow();
        if content.closed || content.error.is_some() {
            return Err(Error::Conflict("Document content is unavailable"));
        }
        Ok(url)
    }
    pub fn subscribe(&self) -> watch::Receiver<DocumentState> {
        self.state.clone()
    }
    pub async fn close(&self, force: bool) -> Result<bool> {
        if force
            && !matches!(
                self.process.snapshot().phase,
                ProcessPhase::Exited | ProcessPhase::NeedsForce
            )
        {
            return Err(Error::Conflict(
                "Force stop requires a renderer waiting after graceful close",
            ));
        }
        self.stop.send_replace(true);
        let stopped = if force {
            self.process.force_stop().await?
        } else {
            self.process.close().await?
        };
        if !stopped {
            return Ok(false);
        }
        self.state
            .clone()
            .wait_for(|s| s.closed)
            .await
            .map_err(|_| Error::Missing("Document owner stopped before confirming closure"))?;
        Ok(true)
    }
}
async fn own(
    process: RenderProcess,
    grant: DocumentGrant,
    origin: String,
    idle: IdleLease,
    timeout: Duration,
    mut stop: watch::Receiver<bool>,
    state: watch::Sender<DocumentState>,
) {
    let produced = tokio::select! {
        biased;
        _=stop.changed()=>None,
        _=idle.wait_expired()=>None,
        result=produce(&process,&grant,&origin,timeout)=>Some(result),
    };
    match produced {
        Some(Ok(server)) => {
            {
                let closing = stop.borrow();
                if !*closing && !idle.expired() {
                    match server.url() {
                        Ok(url) => state.send_modify(|s| {
                            s.content = Some(server.subscribe());
                            s.readiness = Readiness::Ready(url);
                        }),
                        Err(_) => state.send_modify(|s| {
                            s.readiness =
                                Readiness::Failed("Document content server is unavailable")
                        }),
                    }
                }
            }
            let mut content = server.subscribe();
            if !*stop.borrow() && !idle.expired() {
                tokio::select! {
                    biased;
                    _=stop.changed()=>(),
                    _=idle.wait_expired()=>(),
                    changed=content.wait_for(|s|s.closed||s.error.is_some())=> {
                        let reason=changed.ok().and_then(|s|s.error).unwrap_or("Document content server stopped unexpectedly");
                        state.send_modify(|s|s.readiness=Readiness::Failed(reason));
                    },
                }
            }
            state.send_modify(mark_closing);
            if server.close().await.is_err() {
                state.send_modify(|s| {
                    s.readiness =
                        Readiness::Failed("Document content server did not confirm closure")
                });
                return; // Never report the view as closed without confirming its content listener.
            }
        }
        Some(Err(reason)) => state.send_modify(|s| s.readiness = Readiness::Failed(reason)),
        None => (),
    }
    state.send_modify(mark_closing);
    let _ = process.close().await;
    // NeedsForce remains owned. Only a separately confirmed force request may kill it.
    let mut observed = process.observe();
    if observed
        .wait_for(|s| s.phase == ProcessPhase::Exited)
        .await
        .is_ok()
    {
        state.send_modify(|s| s.closed = true);
    }
}
fn mark_closing(state: &mut DocumentState) {
    state.closing = true;
    if matches!(state.readiness, Readiness::Ready(_)) {
        state.readiness = Readiness::Waiting;
    }
}
async fn produce(
    process: &RenderProcess,
    grant: &DocumentGrant,
    origin: &str,
    timeout: Duration,
) -> std::result::Result<DocumentServer, &'static str> {
    let mut observed = process.observe();
    let completed = tokio::time::timeout(
        timeout,
        observed.wait_for(|s| matches!(s.phase, ProcessPhase::Exited | ProcessPhase::NeedsForce)),
    )
    .await
    .map_err(|_| "Document generation exceeded its deadline")?
    .map_err(|_| "Document renderer owner stopped")?
    .clone();
    let document = grant
        .read(&completed)
        .await
        .map_err(|_| "Document renderer did not produce one valid bounded standalone file")?;
    DocumentServer::start(document, origin)
        .await
        .map_err(|_| "Document content server could not start")
}
#[cfg(test)]
#[path = "document_monitor_tests.rs"]
mod tests;
