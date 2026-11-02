//! Readiness supervision continues without an HTTP observer or a connected browser.
use super::{
    process::RenderProcess,
    readiness::{Readiness, ServerReadiness},
};
use tokio::sync::watch;

pub struct ServerMonitor {
    state: watch::Receiver<Readiness>,
}
impl ServerMonitor {
    /// Attach without yielding, after the validator and process have been created.
    pub fn start(process: RenderProcess, mut validator: ServerReadiness) -> Self {
        let (state, observed) = watch::channel(Readiness::Waiting);
        let deadline = validator.deadline();
        tokio::spawn(async move {
            let mut changes = process.observe();
            loop {
                let snapshot = changes.borrow_and_update().clone();
                // Explicit close is not a report failure, including an already coalesced exit.
                if snapshot.close_requested {
                    return;
                }
                let readiness = validator.observe(&snapshot);
                let waiting = readiness == Readiness::Waiting;
                let failed = matches!(readiness, Readiness::Failed(_));
                state.send_replace(readiness);
                if failed {
                    let _ = process.close().await;
                    return;
                }
                tokio::select! {
                    changed = changes.changed() => { if changed.is_err() { return; } },
                    _ = tokio::time::sleep_until(deadline), if waiting => (),
                }
            }
        });
        Self { state: observed }
    }
    pub fn snapshot(&self) -> Readiness {
        self.state.borrow().clone()
    }
}
