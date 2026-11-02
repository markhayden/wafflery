//! The leader stays unreaped until the group is drained; no signal targets a released ID.
use super::{CLOSE_GRACE, Control, ProcessPhase, ProcessState};
use crate::tools::render::group::{GroupState, OwnedGroup};
use std::time::Duration;
use tokio::{
    process::ChildStdin,
    sync::{mpsc, watch},
    time::Instant,
};

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Eof,
    Term,
    Force,
}

pub(super) async fn own(
    mut group: OwnedGroup,
    mut stdin: Option<ChildStdin>,
    mut control: mpsc::UnboundedReceiver<Control>,
    state: watch::Sender<ProcessState>,
    mut readers: [tokio::task::JoinHandle<()>; 2],
) {
    let mut changes = state.subscribe();
    let mut open = true;
    let mut deadline = None;
    let mut stage = None;
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = tick.tick() => {
                // Do not cancel a membership observation to service another event. It is
                // bounded, owns its helper, and completes before this worker can exit.
                match group.check().await {
                    Ok(GroupState::Running) => (),
                    Ok(GroupState::Draining) => {
                        if state.borrow().phase != ProcessPhase::NeedsForce && (stage.is_none() || stage == Some(Stage::Eof)) {
                            stdin.take();
                            terminate(&group, &state, &mut stage, &mut deadline);
                        }
                    },
                    Ok(GroupState::Exited(status)) => {
                        for reader in &mut readers {
                            if tokio::time::timeout(CLOSE_GRACE, &mut *reader).await.is_err() { reader.abort(); let _ = reader.await; }
                        }
                        state.send_modify(|s| {
                            s.phase = ProcessPhase::Exited;
                            s.exit_code = status.code();
                            for output in [&mut s.stdout, &mut s.stderr] {
                                if !output.finished { output.error = Some("Renderer output remained open after exit".into()); output.finished = true; }
                            }
                        });
                        return;
                    },
                    Err(error) => {
                        stdin.take(); deadline = None;
                        state.send_modify(|s| { s.phase = ProcessPhase::NeedsForce; s.error = Some(error.to_string()); });
                    },
                }
            },
            request = control.recv(), if open => {
                match request {
                    Some(Control::Force(accepted)) => {
                        let result = if state.borrow().phase == ProcessPhase::NeedsForce {
                            match group.force() {
                                Ok(()) => {
                                    stage = Some(Stage::Force); deadline = Some(Instant::now() + CLOSE_GRACE);
                                    state.send_modify(|s| { s.phase = ProcessPhase::Stopping; s.error = None; });
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
                        state.send_modify(|s| s.close_requested = true);
                        if state.borrow().phase == ProcessPhase::Running {
                            stdin.take(); stage = Some(Stage::Eof); deadline = Some(Instant::now() + CLOSE_GRACE);
                            state.send_modify(|s| s.phase = ProcessPhase::Stopping);
                        }
                    },
                }
            },
            _ = changes.changed() => {
                let bad_output = { let s = changes.borrow_and_update(); s.stdout.overflow || s.stderr.overflow || s.stdout.error.is_some() || s.stderr.error.is_some() };
                if bad_output && state.borrow().phase == ProcessPhase::Running {
                    stdin.take(); stage = Some(Stage::Eof); deadline = Some(Instant::now() + CLOSE_GRACE);
                    state.send_modify(|s| s.phase = ProcessPhase::Stopping);
                }
            },
            _ = async { if let Some(at) = deadline { tokio::time::sleep_until(at).await; } else { std::future::pending::<()>().await; } } => {
                if stage == Some(Stage::Eof) { terminate(&group, &state, &mut stage, &mut deadline); }
                else { deadline = None; state.send_modify(|s| s.phase = ProcessPhase::NeedsForce); }
            },
        }
    }
}
fn terminate(
    group: &OwnedGroup,
    state: &watch::Sender<ProcessState>,
    stage: &mut Option<Stage>,
    deadline: &mut Option<Instant>,
) {
    match group.terminate() {
        Ok(()) => {
            *stage = Some(Stage::Term);
            *deadline = Some(Instant::now() + CLOSE_GRACE);
            state.send_modify(|s| s.phase = ProcessPhase::Stopping);
        }
        Err(error) => {
            *deadline = None;
            state.send_modify(|s| {
                s.phase = ProcessPhase::NeedsForce;
                s.error = Some(error.to_string());
            });
        }
    }
}
