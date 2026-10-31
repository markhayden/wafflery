//! Bounded invalidations, not an event log or evidence of execution.
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
#[derive(Clone, Debug, serde::Serialize)]
pub struct Event {
    pub session: String,
    pub sequence: u64,
    pub kind: &'static str,
}
struct Inner {
    session: String,
    state: Mutex<(u64, bool)>,
    sender: broadcast::Sender<Event>,
}
#[derive(Clone)]
pub struct Events(Arc<Inner>);
impl Events {
    pub fn new() -> std::io::Result<Self> {
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(std::io::Error::other)?;
        let session = random.iter().map(|b| format!("{b:02x}")).collect();
        let (sender, _) = broadcast::channel(64);
        Ok(Self(Arc::new(Inner {
            session,
            state: Mutex::new((0, false)),
            sender,
        })))
    }
    pub fn session(&self) -> &str {
        &self.0.session
    }
    fn current(&self, kind: &'static str) -> Event {
        let state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        Event {
            session: self.0.session.clone(),
            sequence: state.0,
            kind,
        }
    }
    pub fn changed(&self) {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.1 {
            state.0 += 1;
            let _ = self.0.sender.send(Event {
                session: self.0.session.clone(),
                sequence: state.0,
                kind: "invalidate",
            });
        }
    }
    pub fn close(&self) {
        let mut state = self.0.state.lock().unwrap_or_else(|e| e.into_inner());
        state.1 = true;
        let _ = self.0.sender.send(Event {
            session: self.0.session.clone(),
            sequence: state.0,
            kind: "shutdown",
        });
    }
    pub fn subscribe(&self) -> Subscriber {
        Subscriber {
            events: self.clone(),
            receiver: self.0.sender.subscribe(),
            initial: true,
        }
    }
}
pub struct Subscriber {
    events: Events,
    receiver: broadcast::Receiver<Event>,
    initial: bool,
}
impl Subscriber {
    pub async fn next(&mut self) -> Option<Event> {
        if self
            .events
            .0
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .1
        {
            return None;
        }
        if self.initial {
            self.initial = false;
            return Some(self.events.current("reset"));
        }
        match tokio::time::timeout(std::time::Duration::from_secs(15), self.receiver.recv()).await {
            Ok(Ok(event)) if event.kind == "shutdown" => None,
            Ok(Ok(event)) => Some(event),
            Ok(Err(broadcast::error::RecvError::Lagged(_))) => {
                self.receiver = self.events.0.sender.subscribe();
                Some(self.events.current("reset"))
            }
            Ok(Err(broadcast::error::RecvError::Closed)) => None,
            Err(_) => Some(self.events.current("heartbeat")),
        }
    }
}
