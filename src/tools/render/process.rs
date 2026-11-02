//! The worker owns the child even when a requester drops its handle.
use super::ResolvedRender;
use crate::tools::{Error, Result};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    sync::{mpsc, oneshot, watch},
};

pub const STDOUT_LIMIT: usize = 8 * 1024;
pub const STDERR_LIMIT: usize = 64 * 1024;
#[cfg(target_os = "macos")]
mod owner;
const CLOSE_GRACE: Duration = Duration::from_secs(2);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessPhase {
    Running,
    Stopping,
    NeedsForce,
    Exited,
}
#[derive(Clone, Debug, Default)]
pub struct Output {
    pub bytes: Vec<u8>,
    pub finished: bool,
    pub overflow: bool,
    pub error: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ProcessState {
    pub phase: ProcessPhase,
    pub close_requested: bool,
    pub exit_code: Option<i32>,
    pub stdout: Output,
    pub stderr: Output,
    pub error: Option<String>,
}
enum Control {
    Close,
    Force(oneshot::Sender<std::result::Result<(), String>>),
}
#[derive(Clone)]
#[must_use = "Retain the render handle until its owned process is confirmed stopped"]
pub struct RenderProcess {
    control: mpsc::UnboundedSender<Control>,
    state: watch::Receiver<ProcessState>,
}
impl RenderProcess {
    pub fn spawn(plan: &ResolvedRender) -> Result<Self> {
        #[cfg(not(target_os = "macos"))]
        return Err(Error::Invalid(
            "Report process groups currently require macOS".into(),
        ));
        #[cfg(target_os = "macos")]
        {
            let mut command = plan.command()?;
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut group = super::group::OwnedGroup::spawn(&mut command)?;
            let (stdin, stdout, stderr) = group.take_stdio();
            let (state, observed) = watch::channel(ProcessState {
                phase: ProcessPhase::Running,
                close_requested: false,
                exit_code: None,
                stdout: Output::default(),
                stderr: Output::default(),
                error: None,
            });
            let (control, receiver) = mpsc::unbounded_channel();
            let readers = [
                tokio::spawn(drain(stdout, state.clone(), true)),
                tokio::spawn(drain(stderr, state.clone(), false)),
            ];
            tokio::spawn(owner::own(group, Some(stdin), receiver, state, readers));
            Ok(Self {
                control,
                state: observed,
            })
        }
    }
    pub fn snapshot(&self) -> ProcessState {
        self.state.borrow().clone()
    }
    pub fn observe(&self) -> watch::Receiver<ProcessState> {
        self.state.clone()
    }
    pub async fn close(&self) -> Result<bool> {
        if self.snapshot().phase == ProcessPhase::Exited {
            return Ok(true);
        }
        if self.control.send(Control::Close).is_err() {
            return if self.snapshot().phase == ProcessPhase::Exited {
                Ok(true)
            } else {
                Err(Error::Missing("Renderer owner stopped"))
            };
        }
        self.settled().await
    }
    pub async fn force_stop(&self) -> Result<bool> {
        match self.snapshot().phase {
            ProcessPhase::Exited => return Ok(true),
            ProcessPhase::NeedsForce => (),
            _ => {
                return Err(Error::Conflict(
                    "Force stop requires a renderer waiting after graceful close",
                ));
            }
        }
        let (accepted, reply) = oneshot::channel();
        if self.control.send(Control::Force(accepted)).is_err() {
            return if self.snapshot().phase == ProcessPhase::Exited {
                Ok(true)
            } else {
                Err(Error::Missing("Renderer owner stopped"))
            };
        }
        match reply.await {
            Ok(result) => result.map_err(Error::Invalid)?,
            Err(_) if self.snapshot().phase == ProcessPhase::Exited => return Ok(true),
            Err(_) => return Err(Error::Missing("Renderer owner stopped")),
        }
        self.settled().await
    }
    async fn settled(&self) -> Result<bool> {
        let mut state = self.state.clone();
        let settled = state
            .wait_for(|s| matches!(s.phase, ProcessPhase::Exited | ProcessPhase::NeedsForce))
            .await
            .map_err(|_| Error::Missing("Renderer owner stopped"))?;
        Ok(settled.phase == ProcessPhase::Exited)
    }
}
async fn drain(
    mut reader: impl AsyncRead + Unpin,
    state: watch::Sender<ProcessState>,
    stdout: bool,
) {
    let limit = if stdout { STDOUT_LIMIT } else { STDERR_LIMIT };
    let mut output = Output::default();
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) => {
                output.finished = true;
            }
            Ok(count) => {
                let keep = count.min(limit - output.bytes.len());
                output.bytes.extend_from_slice(&chunk[..keep]);
                if keep < count {
                    output.overflow = true;
                    output.finished = true;
                }
            }
            Err(error) => {
                output.error = Some(error.to_string());
                output.finished = true;
            }
        }
        state.send_modify(|s| {
            if stdout {
                s.stdout = output.clone();
            } else {
                s.stderr = output.clone();
            }
        });
        if output.finished {
            break;
        }
    }
}
#[cfg(test)]
#[path = "process_tests.rs"]
pub(super) mod tests;
