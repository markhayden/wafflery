use super::{Coordinator, Error, Result};
use std::sync::{Arc, atomic::Ordering};
#[derive(serde::Serialize)]
pub struct ShutdownStatus {
    pub phase: &'static str,
    pub active_attempts: usize,
}
impl Coordinator {
    pub async fn shutdown_status(&self) -> ShutdownStatus {
        ShutdownStatus {
            phase: match self.shutdown_phase.load(Ordering::Acquire) {
                0 => "running",
                1 => "stopping",
                2 => "waiting",
                _ => "stopped",
            },
            active_attempts: self.active.lock().await.len(),
        }
    }
    pub async fn begin_shutdown(
        self: &Arc<Self>,
        confirmed: bool,
        quit: tokio::sync::watch::Sender<bool>,
    ) -> Result<ShutdownStatus> {
        let submission = self.submission.lock().await;
        self.reap_exited().await;
        if self.shutdown_phase.load(Ordering::Acquire) != 0 {
            return Ok(self.shutdown_status().await);
        }
        let ids = self.active.lock().await.keys().cloned().collect::<Vec<_>>();
        if !confirmed && !ids.is_empty() {
            return Err(Error::Conflict(
                "This workspace has active or unconfirmed work. Confirm cancellation before quitting.",
            ));
        }
        self.stopping.store(true, Ordering::Release);
        self.shutdown_phase.store(1, Ordering::Release);
        drop(submission);
        let coordinator = self.clone();
        tokio::spawn(async move {
            for id in ids {
                let _ = coordinator.cancel(&id).await;
            }
            let stopped = coordinator.close().await;
            coordinator.finish_shutdown(stopped, quit).await;
        });
        Ok(self.shutdown_status().await)
    }
    pub async fn force_shutdown(
        self: &Arc<Self>,
        quit: tokio::sync::watch::Sender<bool>,
    ) -> Result<ShutdownStatus> {
        self.shutdown_phase
            .compare_exchange(2, 1, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                Error::Conflict(
                    "Force quit is available only after graceful shutdown is waiting for tools",
                )
            })?;
        let coordinator = self.clone();
        tokio::spawn(async move {
            for entry in coordinator.tools.values() {
                if let Some(client) = entry.client() {
                    let _ = client.force_stop().await;
                }
            }
            let stopped = coordinator.close().await;
            coordinator.finish_shutdown(stopped, quit).await;
        });
        Ok(self.shutdown_status().await)
    }
    async fn finish_shutdown(&self, mut stopped: bool, quit: tokio::sync::watch::Sender<bool>) {
        loop {
            self.shutdown_phase
                .store(if stopped { 3 } else { 2 }, Ordering::Release);
            self.events.changed();
            if stopped {
                let _ = quit.send(true);
                return;
            }
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                if self.shutdown_phase.load(Ordering::Acquire) != 2 {
                    return;
                }
                let mut all_exited = true;
                for entry in self.tools.values() {
                    if let Some(client) = entry.client() {
                        all_exited &= client.has_exited().await.unwrap_or(false);
                    }
                }
                let preserving = self.preserving_tools().await;
                let ready_to_close = self.tools.iter().any(|(id, entry)| {
                    !preserving.contains(id)
                        && entry
                            .client()
                            .as_ref()
                            .is_some_and(|client| client.peer().is_ok())
                });
                if all_exited || ready_to_close {
                    if self
                        .shutdown_phase
                        .compare_exchange(2, 1, Ordering::AcqRel, Ordering::Acquire)
                        .is_err()
                    {
                        return;
                    }
                    stopped = self.close().await;
                    break;
                }
            }
        }
    }
}
