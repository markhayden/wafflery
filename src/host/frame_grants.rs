//! Host-lifetime capabilities for one wrapper load; never browser API credentials.
use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::Instant;

const LIFETIME: Duration = Duration::from_secs(60);
const CAPACITY: usize = 64;
struct Grant {
    view: String,
    expires: Instant,
}
#[derive(Clone, Default)]
pub struct FrameGrants(Arc<Mutex<HashMap<String, Grant>>>);
impl FrameGrants {
    /// Call only after validating the live view. Consumption must revalidate it again.
    pub fn issue(&self, view: &str) -> io::Result<String> {
        let mut grants = self.0.lock().expect("frame grant mutex");
        let now = Instant::now();
        grants.retain(|_, grant| grant.expires > now);
        if grants.len() >= CAPACITY {
            return Err(io::Error::other("Too many unused report frame grants"));
        }
        let mut random = [0_u8; 32];
        getrandom::fill(&mut random).map_err(io::Error::other)?;
        let token: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        if grants.contains_key(&token) {
            return Err(io::Error::other("Could not allocate a unique frame grant"));
        }
        grants.insert(
            token.clone(),
            Grant {
                view: view.into(),
                expires: now + LIFETIME,
            },
        );
        Ok(token)
    }
    /// Removal is atomic and precedes target validation: a failed load cannot reuse a grant.
    pub fn consume(&self, token: &str) -> Option<String> {
        let grant = self.0.lock().expect("frame grant mutex").remove(token)?;
        (Instant::now() < grant.expires).then_some(grant.view)
    }
}
#[cfg(test)]
#[path = "frame_grants_tests.rs"]
mod tests;
