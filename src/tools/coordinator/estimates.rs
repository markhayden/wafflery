use super::{Coordinator, Error, PreparedEstimate, Result};
use std::{
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::time::{Instant, timeout_at};

impl Coordinator {
    pub async fn estimate_prepared(self: &Arc<Self>, id: &str) -> Result<PreparedEstimate> {
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
        let (lease, ticket) =
            self.preparations
                .begin_estimate(id, &declaration, &client.session)?;
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
        // Optional: without a limit the tool's own caps decide (table-manners 4.5).
        let cap = match &lease.arguments()["max_spend_usd"] {
            serde_json::Value::Null => None,
            asked => Some(
                asked
                    .as_f64()
                    .filter(|cap| cap.is_finite() && *cap > 0.0)
                    .ok_or_else(|| {
                        Error::Invalid("A spending limit must be a positive amount".into())
                    })?,
            ),
        };
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let mut workers = timeout_at(deadline, self.workers.lock()).await?;
        workers.retain(|worker| !worker.is_finished());
        let coordinator = self.clone();
        let worker_ticket = ticket.clone();
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
                coordinator.preparations.save_estimate(
                    &lease,
                    &worker_ticket,
                    current,
                    &client.session,
                    quote,
                )
            });
            // Release ownership before reporting completion; invalidated inputs can now disappear.
            drop(lease);
            drop(slot);
            let _ = sender.send(result);
            coordinator.events.changed();
        }));
        drop(workers);
        drop(submission);
        let result = match timeout_at(deadline, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::Invalid(
                "Owned estimate task did not complete".into(),
            )),
            Err(error) => Err(error.into()),
        };
        if result.is_err() {
            ticket.invalidate();
        }
        result
    }
    pub async fn prepared_estimate(&self, id: &str) -> Result<PreparedEstimate> {
        let _submission = self.submission.lock().await;
        let receipt = self.preparations.get(id)?;
        let current = self.description(&receipt.tool).await?;
        let client = self.tools[&receipt.tool]
            .client()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        if client.generation() != current.generation || client.peer().is_err() {
            self.preparations.invalidate_tool(&receipt.tool);
            return Err(Error::Conflict(
                "Tool changed or disconnected; prepare a new estimate",
            ));
        }
        self.preparations
            .reconcile(&receipt.tool, &current, &client.session);
        self.preparations.estimate(id)
    }
}
