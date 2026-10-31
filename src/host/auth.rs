//! Host-lifetime bearer sessions. No cookies or disk-persisted credentials.
use std::{
    collections::{HashMap, HashSet},
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Auth(Arc<Mutex<Tokens>>);

struct Tokens {
    bootstrap: HashMap<String, Instant>,
    sessions: HashSet<String>,
    host_session: String,
}

fn random_token() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::fill(&mut bytes).map_err(io::Error::other)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[derive(serde::Serialize)]
pub struct Session {
    pub token: String,
    pub host_session: String,
}

impl Auth {
    pub fn new() -> io::Result<Self> {
        Ok(Self(Arc::new(Mutex::new(Tokens {
            bootstrap: HashMap::new(),
            sessions: HashSet::new(),
            host_session: random_token()?,
        }))))
    }

    pub fn issue(&self) -> io::Result<String> {
        let mut tokens = self.0.lock().expect("auth mutex");
        let now = Instant::now();
        tokens.bootstrap.retain(|_, expires| *expires > now);
        if tokens.bootstrap.len() >= 64 || tokens.sessions.len() >= 256 {
            return Err(io::Error::other(
                "browser session limit reached; restart the idle host",
            ));
        }
        let token = random_token()?;
        tokens
            .bootstrap
            .insert(token.clone(), now + Duration::from_secs(60));
        Ok(token)
    }

    pub fn exchange(&self, bootstrap: &str) -> io::Result<Option<Session>> {
        let mut tokens = self.0.lock().expect("auth mutex");
        if tokens
            .bootstrap
            .remove(bootstrap)
            .is_none_or(|expires| expires <= Instant::now())
            || tokens.sessions.len() >= 256
        {
            return Ok(None);
        }
        let token = random_token()?;
        tokens.sessions.insert(token.clone());
        Ok(Some(Session {
            token,
            host_session: tokens.host_session.clone(),
        }))
    }

    pub fn accepts(&self, token: &str) -> bool {
        token.len() == 64 && self.0.lock().expect("auth mutex").sessions.contains(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_is_single_use_and_expiry_does_not_authorize() {
        let auth = Auth::new().unwrap();
        let bootstrap = auth.issue().unwrap();
        assert!(!auth.accepts(&bootstrap));
        let session = auth.exchange(&bootstrap).unwrap().unwrap();
        assert!(auth.accepts(&session.token));
        assert!(auth.exchange(&bootstrap).unwrap().is_none());
        let expired = auth.issue().unwrap();
        auth.0
            .lock()
            .unwrap()
            .bootstrap
            .insert(expired.clone(), Instant::now() - Duration::from_secs(1));
        assert!(auth.exchange(&expired).unwrap().is_none());
        assert!(!Auth::new().unwrap().accepts(&session.token));
    }
}
