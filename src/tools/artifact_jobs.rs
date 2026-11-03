//! Bounded transfer ownership survives a dropped HTTP waiter.
use super::{Error, Result};
use crate::events::Events;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
pub(super) struct Jobs {
    slots: Arc<Semaphore>,
    active: Mutex<BTreeMap<String, usize>>,
    events: Events,
}
pub(super) struct Job {
    jobs: Arc<Jobs>,
    tool: String,
    slot: Option<OwnedSemaphorePermit>,
}
impl Jobs {
    pub fn new(events: Events) -> Arc<Self> {
        Arc::new(Self {
            slots: Arc::new(Semaphore::new(2)),
            active: Mutex::new(BTreeMap::new()),
            events,
        })
    }
    pub async fn reserve(&self) -> Result<OwnedSemaphorePermit> {
        self.slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Error::Invalid("Artifact transfer registry stopped".into()))
    }
    pub fn start(self: &Arc<Self>, tool: &str) -> Result<Job> {
        let slot = self.slots.clone().try_acquire_owned().map_err(|_| {
            Error::Conflict("Two artifact transfers are active. Wait before starting another.")
        })?;
        Ok(self.start_reserved(tool, slot))
    }
    pub fn start_reserved(self: &Arc<Self>, tool: &str, slot: OwnedSemaphorePermit) -> Job {
        self.mark(tool, Some(slot))
    }
    pub fn group(self: &Arc<Self>, tool: &str) -> Job {
        self.mark(tool, None)
    }
    fn mark(self: &Arc<Self>, tool: &str, slot: Option<OwnedSemaphorePermit>) -> Job {
        *self
            .active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(tool.into())
            .or_default() += 1;
        self.events.changed();
        Job {
            jobs: self.clone(),
            tool: tool.into(),
            slot,
        }
    }
    pub fn count(&self) -> usize {
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .sum()
    }
    pub fn tools(&self) -> BTreeSet<String> {
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .keys()
            .cloned()
            .collect()
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        let mut active = self.jobs.active.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = active.get_mut(&self.tool) {
            *count -= 1;
            if *count == 0 {
                active.remove(&self.tool);
            }
        }
        drop(self.slot.take());
        drop(active);
        self.jobs.events.changed();
    }
}
pub(super) struct Waiter(pub Arc<AtomicBool>);
impl Waiter {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }
}
impl Drop for Waiter {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
