use super::{Coordinator, Error, Result};
use crate::tools::estimate::ValidatedEstimate;
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::time::{Instant, timeout_at};

impl Coordinator {
    /// Rust control service only; the authenticated browser route joins in V10b3.
    pub async fn estimate_prepared(self: &Arc<Self>, id: &str) -> Result<ValidatedEstimate> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let submission = timeout_at(deadline, self.submission.lock()).await?;
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        let receipt = self.preparations.get(id)?;
        let entry = self
            .tools
            .get(&receipt.tool)
            .ok_or(Error::Missing("Tool not registered"))?;
        let client = entry
            .client()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        let slot = client
            .estimate_slot
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Conflict("An estimate is already pending for this tool"))?;
        let declaration = self.description(&receipt.tool).await?;
        let lease = self.preparations.lease(id, &declaration, &client.session)?;
        let action = declaration.document["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|action| action["name"] == lease.request().action)
            .ok_or(Error::Missing("Action not declared"))?;
        if action["cost"]["may_spend"] != true {
            return Err(Error::Conflict(
                "Only paid actions need a bound cost estimate",
            ));
        }
        let cap = lease.arguments()["max_spend_usd"]
            .as_f64()
            .filter(|cap| cap.is_finite() && *cap > 0.0)
            .ok_or_else(|| {
                Error::Invalid("Choose a positive spending cap before estimating".into())
            })?;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let mut workers = timeout_at(deadline, self.workers.lock()).await?;
        workers.retain(|worker| !worker.is_finished());
        let coordinator = self.clone();
        workers.push(tokio::spawn(async move {
            let result = client
                .estimate_control(
                    &declaration,
                    &lease.request().action,
                    lease.arguments(),
                    cap,
                    deadline,
                )
                .await;
            let result = result.and_then(|quote| {
                // No submission/discovery await here: shutdown may hold those while joining us.
                let state = coordinator.tools[&receipt.tool]
                    .state
                    .read()
                    .unwrap_or_else(|e| e.into_inner());
                let current = state
                    .declaration
                    .as_ref()
                    .ok_or(Error::Missing("Tool declaration is unavailable"))?;
                if Instant::now() >= deadline
                    || coordinator.stopping.load(Ordering::Acquire)
                    || !state
                        .client
                        .as_ref()
                        .is_some_and(|current| Arc::ptr_eq(current, &client))
                    || client.generation() != current.generation
                    || !coordinator
                        .preparations
                        .is_current(&lease, current, &client.session)
                {
                    return Err(Error::Conflict(
                        "Estimate no longer belongs to an available preparation",
                    ));
                }
                Ok(quote)
            });
            // Release ownership before reporting completion; invalidated inputs can now disappear.
            drop(lease);
            drop(slot);
            let _ = sender.send(result);
            coordinator.events.changed();
        }));
        drop(workers);
        drop(submission);
        timeout_at(deadline, receiver)
            .await?
            .map_err(|_| Error::Invalid("Owned estimate task did not complete".into()))?
    }
}
