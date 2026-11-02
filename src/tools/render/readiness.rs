//! Validate accumulated stdout without treating diagnostic text as a ready address.
use super::{checked_origin, process::ProcessPhase, process::ProcessState};
use crate::tools::{Error, Result};
use std::time::Duration;
use tokio::time::Instant;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Readiness {
    Waiting,
    Ready(Url),
    Failed(&'static str),
}

pub struct ServerReadiness {
    host_origin: String,
    started: Instant,
    timeout: Duration,
    accepted: Option<Url>,
    failure: Option<&'static str>,
}

impl ServerReadiness {
    pub fn new(host_origin: &str, timeout: Duration) -> Result<Self> {
        if !(1..=60).contains(&timeout.as_secs()) || timeout.subsec_nanos() != 0 {
            return Err(Error::Invalid(
                "Report readiness timeout must be 1–60 whole seconds".into(),
            ));
        }
        Ok(Self {
            host_origin: checked_origin(host_origin)?,
            started: Instant::now(),
            timeout,
            accepted: None,
            failure: None,
        })
    }

    pub fn observe(&mut self, state: &ProcessState) -> Readiness {
        self.observe_at(state, Instant::now())
    }

    fn observe_at(&mut self, state: &ProcessState, now: Instant) -> Readiness {
        if let Some(reason) = self.failure {
            return Readiness::Failed(reason);
        }
        let observed = self.check(state, now);
        match &observed {
            Readiness::Failed(reason) => self.failure = Some(reason),
            Readiness::Ready(url) => self.accepted = Some(url.clone()),
            Readiness::Waiting => (),
        }
        observed
    }

    fn check(&self, state: &ProcessState, now: Instant) -> Readiness {
        if state.phase != ProcessPhase::Running || state.error.is_some() {
            return Readiness::Failed("Report process is not running");
        }
        if state.stdout.overflow
            || state.stderr.overflow
            || state.stdout.error.is_some()
            || state.stderr.error.is_some()
        {
            return Readiness::Failed("Report output exceeded its limit or could not be read");
        }
        if self.accepted.is_none() && now.duration_since(self.started) >= self.timeout {
            return Readiness::Failed("Report did not become ready before its deadline");
        }
        let bytes = &state.stdout.bytes;
        let Some(end) = bytes.iter().position(|&b| b == b'\n') else {
            return if state.stdout.finished || self.accepted.is_some() {
                Readiness::Failed("Report did not provide one complete URL line")
            } else {
                Readiness::Waiting
            };
        };
        if end + 1 != bytes.len() {
            return Readiness::Failed("Report wrote extra stdout after its URL");
        }
        let line = bytes[..end].strip_suffix(b"\r").unwrap_or(&bytes[..end]);
        let Some(url) = std::str::from_utf8(line).ok().and_then(ready_url) else {
            return Readiness::Failed(
                "Report URL must use HTTP on literal loopback with an explicit port",
            );
        };
        if url.origin().ascii_serialization() == self.host_origin {
            return Readiness::Failed("Report must use a separate origin from the workspace");
        }
        if self
            .accepted
            .as_ref()
            .is_some_and(|accepted| accepted != &url)
        {
            return Readiness::Failed("Report changed its ready URL");
        }
        Readiness::Ready(url)
    }
}

fn ready_url(line: &str) -> Option<Url> {
    if line
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
    {
        return None;
    }
    // Check the literal authority before URL parsing can normalize shorthand IPs or ports.
    let authority = line.strip_prefix("http://127.0.0.1:")?;
    let port = authority.split(['/', '?', '#']).next()?;
    if port.is_empty()
        || !port.bytes().all(|b| b.is_ascii_digit())
        || port.parse::<u16>().ok()? == 0
    {
        return None;
    }
    Url::parse(line).ok()
}

#[cfg(test)]
#[path = "readiness_tests.rs"]
mod tests;
