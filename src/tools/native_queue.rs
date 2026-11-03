//! Two on-demand workers, at most sixteen waiting attempts; no action is ever replayed.
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex, Weak},
};
#[derive(Default)]
struct State {
    waiting: VecDeque<(String, String)>,
    active: BTreeMap<String, String>,
    workers: usize,
    refs: BTreeMap<String, Weak<tokio::sync::Mutex<()>>>,
}
#[derive(Default)]
pub(super) struct Queue(Mutex<State>);
impl Queue {
    pub fn contains(&self, id: &str) -> bool {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.active.contains_key(id) || state.waiting.iter().any(|(queued, _)| queued == id)
    }
    pub fn enqueue(&self, id: &str, tool: &str) -> bool {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if state.active.contains_key(id) || state.waiting.iter().any(|(queued, _)| queued == id) {
            return true;
        }
        if state.waiting.len() >= 16 {
            return false;
        }
        state.waiting.push_back((id.into(), tool.into()));
        true
    }
    pub fn workers_needed(&self) -> usize {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let count = (2 - state.workers).min(state.waiting.len());
        state.workers += count;
        count
    }
    pub fn next(&self) -> Option<String> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let Some((id, tool)) = state.waiting.pop_front() else {
            state.workers -= 1;
            return None;
        };
        state.active.insert(id.clone(), tool);
        Some(id)
    }
    pub fn finish(&self, id: &str) {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active
            .remove(id);
    }
    pub fn tools(&self) -> BTreeSet<String> {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state
            .active
            .values()
            .chain(state.waiting.iter().map(|(_, tool)| tool))
            .cloned()
            .collect()
    }
    pub fn count(&self) -> usize {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.active.len() + state.waiting.len()
    }
    pub fn reference(&self, key: String) -> Arc<tokio::sync::Mutex<()>> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        state.refs.retain(|_, value| value.strong_count() > 0);
        if let Some(lock) = state.refs.get(&key).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(tokio::sync::Mutex::new(()));
        state.refs.insert(key, Arc::downgrade(&lock));
        lock
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_workers_and_sixteen_waiting_attempts_are_bounded_and_deduplicated() {
        let queue = Queue::default();
        assert!(queue.enqueue("first", "peer"));
        assert_eq!(queue.workers_needed(), 1);
        assert_eq!(queue.next().as_deref(), Some("first"));
        assert!(queue.enqueue("second", "peer"));
        assert_eq!(queue.workers_needed(), 1);
        assert_eq!(queue.next().as_deref(), Some("second"));
        for index in 0..16 {
            assert!(queue.enqueue(&format!("waiting-{index}"), "peer"));
        }
        assert!(!queue.enqueue("overflow", "peer"));
        assert!(queue.enqueue("first", "peer"));
        assert_eq!(queue.count(), 18);
        assert_eq!(queue.workers_needed(), 0);
        queue.finish("first");
        assert!(queue.next().is_some());
        assert!(queue.enqueue("overflow", "peer"));
        let one = queue.reference("same".into());
        assert!(Arc::ptr_eq(&one, &queue.reference("same".into())));
        assert!(!Arc::ptr_eq(&one, &queue.reference("other".into())));
    }
}
