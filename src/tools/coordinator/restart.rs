use super::{Coordinator, Declaration, DeclarationState, Error, Result, SubmissionChange};
use std::sync::{Arc, atomic::Ordering};

impl Coordinator {
    /// Replacement is deliberate, serialized with submission, and never force-stops a child.
    pub async fn restart(self: &Arc<Self>, id: &str, confirmed: bool) -> Result<Declaration> {
        if !confirmed {
            return Err(Error::Conflict(
                "Confirm restart: the replacement has a new session and spending ledger.",
            ));
        }
        // Dropping an HTTP requester must not abandon a newly spawned child. The task
        // owns the coordinator until the old child is stopped and the replacement is published.
        let coordinator = self.clone();
        let id = id.to_owned();
        tokio::spawn(async move { coordinator.restart_owned(&id).await })
            .await
            .map_err(|error| Error::Invalid(format!("Tool restart task failed: {error}")))?
    }
    async fn restart_owned(&self, id: &str) -> Result<Declaration> {
        let _submission = self.submission.lock().await;
        let _change = SubmissionChange(self.events.clone());
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        self.reap_exited().await;
        let entry = self
            .tools
            .get(id)
            .ok_or(Error::Missing("Tool not registered"))?;
        let _reservation = entry.reservation.clone().try_acquire_owned().map_err(|_| {
            Error::Conflict(
                "This tool has active or unconfirmed work. Stop that work before restarting.",
            )
        })?;
        let _discovery = entry.discovery.lock().await;
        self.preparations.invalidate_tool(id);
        {
            let mut state = entry.state.write().unwrap_or_else(|e| e.into_inner());
            state.declaration = None;
            state.error = Some("Restarting tool session.".into());
        }
        self.events.changed();
        if let Some(client) = entry.client() {
            match client.close().await {
                Ok(true) => (),
                result => {
                    entry.state.write().unwrap_or_else(|e| e.into_inner()).error = Some(match result {
                        Err(error) => format!("The previous tool could not be stopped: {error}"),
                        _ => "The previous tool has not stopped. No replacement was started; quit the workspace to stop the owned child.".into(),
                    });
                    return Err(Error::Conflict(
                        "The previous tool has not stopped; no replacement was started.",
                    ));
                }
            }
        }
        let started = crate::tools::probe::start(entry.descriptor.clone()).await;
        let result = started.declaration.clone().ok_or_else(|| {
            Error::Invalid(
                started
                    .error
                    .clone()
                    .unwrap_or_else(|| "Tool restart failed".into()),
            )
        });
        *entry.state.write().unwrap_or_else(|e| e.into_inner()) = DeclarationState {
            client: started.client,
            declaration: started.declaration,
            error: started.error,
        };
        result
    }
}
