//! The worker owns the child even when a requester drops its handle.
use super::ResolvedRender;
use crate::tools::{Error, Result};
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, ChildStdin},
    sync::{mpsc, oneshot, watch},
    time::Instant,
};

pub const STDOUT_LIMIT: usize = 8 * 1024;
pub const STDERR_LIMIT: usize = 64 * 1024;
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
        let mut child = plan
            .command()?
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let stdin = child.stdin.take().expect("piped render stdin");
        let stdout = child.stdout.take().expect("piped render stdout");
        let stderr = child.stderr.take().expect("piped render stderr");
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
        tokio::spawn(own(child, Some(stdin), receiver, state, readers));
        Ok(Self {
            control,
            state: observed,
        })
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
async fn own(
    mut child: Child,
    mut stdin: Option<ChildStdin>,
    mut control: mpsc::UnboundedReceiver<Control>,
    state: watch::Sender<ProcessState>,
    mut readers: [tokio::task::JoinHandle<()>; 2],
) {
    let mut changes = state.subscribe();
    let mut open = true;
    let mut deadline = None;
    let mut can_wait = true;
    loop {
        tokio::select! {
            result = child.wait(), if can_wait => {
                let status = match result {
                    Ok(status) => status,
                    Err(error) => {
                        can_wait = false;
                        deadline = None;
                        stdin.take();
                        state.send_modify(|s| { s.phase = ProcessPhase::NeedsForce; s.error = Some(error.to_string()); });
                        continue;
                    }
                };
                // A descendant keeping a pipe open must not hold result inspection indefinitely.
                for reader in &mut readers {
                    if tokio::time::timeout(CLOSE_GRACE, &mut *reader).await.is_err() { reader.abort(); let _ = reader.await; }
                }
                state.send_modify(|s| {
                    s.phase = ProcessPhase::Exited;
                    s.exit_code = status.code();
                    for output in [&mut s.stdout,&mut s.stderr] {
                        if !output.finished { output.error = Some("Renderer output remained open after exit".into()); output.finished = true; }
                    }
                });
                return;
            },
            request = control.recv(), if open => {
                match request {
                    Some(Control::Force(accepted)) => {
                        let result = if state.borrow().phase == ProcessPhase::NeedsForce {
                            match child.start_kill() {
                                Ok(()) => {
                                    can_wait = true;
                                    deadline = Some(Instant::now() + CLOSE_GRACE);
                                    state.send_modify(|s| {s.phase = ProcessPhase::Stopping; s.error = None;});
                                    Ok(())
                                },
                                Err(error) => {
                                    let message = error.to_string();
                                    state.send_modify(|s| s.error = Some(message.clone()));
                                    Err(message)
                                },
                            }
                        } else { Ok(()) };
                        let _ = accepted.send(result);
                    },
                    request => {
                        if request.is_none() { open = false; }
                        state.send_modify(|s|s.close_requested = true);
                        if state.borrow().phase == ProcessPhase::Running {
                            stdin.take(); deadline = Some(Instant::now() + CLOSE_GRACE);
                            state.send_modify(|s|s.phase = ProcessPhase::Stopping);
                        }
                    },
                }
            },
            _ = changes.changed() => {
                let bad_output = { let s = changes.borrow_and_update(); s.stdout.overflow || s.stderr.overflow || s.stdout.error.is_some() || s.stderr.error.is_some() };
                if bad_output && state.borrow().phase == ProcessPhase::Running {
                    stdin.take(); deadline = Some(Instant::now() + CLOSE_GRACE);
                    state.send_modify(|s|s.phase = ProcessPhase::Stopping);
                }
            },
            _ = async { if let Some(at) = deadline { tokio::time::sleep_until(at).await; } else { std::future::pending::<()>().await; } } => {
                deadline = None;
                state.send_modify(|s|s.phase = ProcessPhase::NeedsForce);
            },
        }
    }
}
#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
