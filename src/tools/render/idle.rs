//! In-memory idle expiry. Observation alone never counts as inspection activity.
use super::process::{ProcessPhase, RenderProcess};
use std::time::Duration;
use tokio::{sync::watch, time::Instant};

const IDLE_TIMEOUT: Duration = Duration::from_secs(300);
#[derive(Clone, Copy)]
struct State {
    deadline: Instant,
    expired: bool,
}
#[derive(Clone)]
pub struct IdleLease {
    state: watch::Sender<State>,
    duration: Duration,
}
impl IdleLease {
    pub fn document() -> Self {
        Self::new(IDLE_TIMEOUT)
    }
    pub fn attach(process: RenderProcess) -> Self {
        let lease = Self::new(IDLE_TIMEOUT);
        lease.supervise(process);
        lease
    }
    fn new(duration: Duration) -> Self {
        let (state, _) = watch::channel(State {
            deadline: Instant::now() + duration,
            expired: false,
        });
        Self { state, duration }
    }
    pub fn expired(&self) -> bool {
        let state = self.state.borrow();
        state.expired || Instant::now() >= state.deadline
    }
    pub fn touch(&self) -> bool {
        let mut accepted = false;
        self.state.send_if_modified(|state| {
            if state.expired {
                return false;
            }
            let now = Instant::now();
            if now >= state.deadline {
                state.expired = true;
            } else {
                state.deadline = now + self.duration;
                accepted = true;
            }
            true
        });
        accepted
    }
    fn expire_if_due(&self) -> bool {
        self.state.send_if_modified(|state| {
            if state.expired || Instant::now() < state.deadline {
                return false;
            }
            state.expired = true;
            true
        });
        self.expired()
    }
    pub(super) async fn wait_expired(&self) {
        let mut changes = self.state.subscribe();
        loop {
            let state = *changes.borrow_and_update();
            if state.expired {
                return;
            }
            tokio::select! {
                _ = tokio::time::sleep_until(state.deadline) => { if self.expire_if_due() { return; } },
                changed = changes.changed() => { if changed.is_err() { return; } },
            }
        }
    }
    fn supervise(&self, process: RenderProcess) {
        let lease = self.clone();
        let mut changes = process.observe();
        tokio::spawn(async move {
            let expired = tokio::select! {
                _ = lease.wait_expired() => true,
                _ = async { let _ = changes.wait_for(|s| s.phase != ProcessPhase::Running).await; } => false,
            };
            if expired {
                let _ = process.close().await;
            }
        });
    }
}

#[cfg(test)]
#[path = "idle_tests.rs"]
mod tests;
