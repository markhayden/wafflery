//! Serializes cancellation with run allocation and final publication.
use std::sync::Mutex;
#[derive(Default)]
struct State {
    cancelled: bool,
    finished: bool,
}
#[derive(Default)]
pub struct Cancellation(Mutex<State>);
impl Cancellation {
    pub fn cancel(&self) {
        let mut state = self.0.lock().unwrap();
        if !state.finished {
            state.cancelled = true;
        }
    }
    pub fn cancelled(&self) -> bool {
        self.0.lock().unwrap().cancelled
    }
    pub fn allocate<T, E>(&self, operation: impl FnOnce() -> Result<T, E>) -> Result<Option<T>, E> {
        let state = self.0.lock().unwrap();
        if state.cancelled || state.finished {
            return Ok(None);
        }
        operation().map(Some)
    }
    pub fn finish<T, E: std::fmt::Display>(
        &self,
        operation: impl FnOnce(bool) -> Result<T, E>,
    ) -> Result<Option<T>, E> {
        let mut state = self.0.lock().unwrap();
        let result = operation(state.cancelled);
        state.finished = true;
        if state.cancelled {
            if let Err(error) = result {
                eprintln!("Could not save cancelled test run: {error}");
            }
            Ok(None)
        } else {
            result.map(Some)
        }
    }
}
