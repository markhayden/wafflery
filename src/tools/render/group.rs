//! macOS process groups whose leader remains unreaped until no member can execute.
use crate::tools::{Error, Result, environment};
use nix::{
    errno::Errno,
    sys::{
        event::{EvFlags, EventFilter, FilterFlag, KEvent, Kqueue},
        signal::{Signal, killpg},
    },
    unistd::Pid,
};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    process::{ExitStatus, Stdio},
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
};

#[derive(Clone, Copy, Debug)]
pub enum GroupState {
    Running,
    Draining,
    Exited(ExitStatus),
}

/// Only this owner may reap the leader or signal its group. The PID cannot be recycled
/// while the leader is unreaped, even when it exits before its descendants.
pub struct OwnedGroup {
    child: Child,
    pid: i32,
    events: Kqueue,
    registered: bool,
    leader_exited: bool,
    exited: Option<ExitStatus>,
    observer: PathBuf,
}
impl OwnedGroup {
    pub(crate) fn take_stdio(
        &mut self,
    ) -> (
        tokio::process::ChildStdin,
        tokio::process::ChildStdout,
        tokio::process::ChildStderr,
    ) {
        (
            self.child.stdin.take().expect("piped render stdin"),
            self.child.stdout.take().expect("piped render stdout"),
            self.child.stderr.take().expect("piped render stderr"),
        )
    }
    pub fn spawn(command: &mut Command) -> Result<Self> {
        let mut group = Self::spawn_unwatched(command)?;
        group.register();
        Ok(group)
    }
    fn spawn_unwatched(command: &mut Command) -> Result<Self> {
        // Create the observer before starting a child we must retain on every error path.
        let events = Kqueue::new().map_err(os_error)?;
        let child = command.process_group(0).spawn()?;
        let pid = child.id().expect("newly spawned child has an ID") as i32;
        Ok(Self {
            child,
            pid,
            events,
            registered: false,
            leader_exited: false,
            exited: None,
            observer: "/bin/ps".into(),
        })
    }
    fn event(&self) -> KEvent {
        KEvent::new(
            self.pid as usize,
            EventFilter::EVFILT_PROC,
            EvFlags::EV_ADD | EvFlags::EV_ONESHOT,
            FilterFlag::NOTE_EXIT,
            0,
            0,
        )
    }
    fn register(&mut self) {
        // An already-exited child may reject registration. Keep its identity and use
        // a bounded membership observation instead; failure never means "drained".
        self.registered = self
            .events
            .kevent(&[self.event()], &mut [], Some(zero()))
            .is_ok();
    }
    pub async fn check(&mut self) -> Result<GroupState> {
        if let Some(status) = self.exited {
            return Ok(GroupState::Exited(status));
        }
        if !self.leader_exited && self.registered {
            let mut events = [self.event()];
            match self.events.kevent(&[], &mut events, Some(zero())) {
                Ok(0) => return Ok(GroupState::Running),
                Ok(_) if events[0].flags().contains(EvFlags::EV_ERROR) => self.registered = false,
                Ok(_) => self.leader_exited = true,
                Err(_) => self.registered = false,
            }
        }
        let snapshot = self.observe_members().await?;
        self.leader_exited = snapshot.leader_exited;
        if snapshot.drained {
            // No live member remains to fork. Reaping is now safe and permanently
            // disables signaling through this owner.
            let status = self.child.wait().await?;
            self.exited = Some(status);
            return Ok(GroupState::Exited(status));
        }
        Ok(if self.leader_exited {
            GroupState::Draining
        } else {
            GroupState::Running
        })
    }
    pub fn terminate(&self) -> Result<()> {
        self.signal(Signal::SIGTERM)
    }
    pub fn force(&self) -> Result<()> {
        self.signal(Signal::SIGKILL)
    }
    fn signal(&self, signal: Signal) -> Result<()> {
        if self.exited.is_some() {
            return Err(Error::Conflict("Report group identity has been released"));
        }
        match killpg(Pid::from_raw(self.pid), signal) {
            Ok(()) | Err(Errno::ESRCH) => Ok(()),
            Err(error) => Err(os_error(error)),
        }
    }
    async fn observe_members(&self) -> Result<Members> {
        let mut command = Command::new(&self.observer);
        environment::apply(&mut command, &[]);
        command
            .env("LC_ALL", "C")
            .args(["-g", &self.pid.to_string(), "-o", "pid=,pgid=,stat="]);
        members(
            &capture(command, 64 * 1024, Duration::from_secs(1)).await?,
            self.pid,
        )
    }
}
fn zero() -> nix::libc::timespec {
    nix::libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    }
}
fn os_error(error: Errno) -> Error {
    std::io::Error::from_raw_os_error(error as i32).into()
}

struct Members {
    leader_exited: bool,
    drained: bool,
}
fn members(bytes: &[u8], leader: i32) -> Result<Members> {
    let invalid = || Error::Invalid("Could not confirm report group membership".into());
    let text = std::str::from_utf8(bytes).map_err(|_| invalid())?;
    let mut pids = BTreeSet::new();
    let mut leader_exited = None;
    let mut drained = true;
    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(invalid());
        }
        let pid = fields[0].parse::<i32>().map_err(|_| invalid())?;
        let pgid = fields[1].parse::<i32>().map_err(|_| invalid())?;
        let state = fields[2];
        if pid <= 0
            || pgid != leader
            || !pids.insert(pid)
            || state.len() > 16
            || !matches!(
                state.as_bytes().first(),
                Some(b'I' | b'R' | b'S' | b'T' | b'U' | b'Z')
            )
            || !state
                .bytes()
                .all(|c| c.is_ascii_alphabetic() || b"+<>".contains(&c))
        {
            return Err(invalid());
        }
        let zombie = state.starts_with('Z');
        drained &= zombie;
        if pid == leader {
            leader_exited = Some(zombie);
        }
    }
    Ok(Members {
        leader_exited: leader_exited.ok_or_else(invalid)?,
        drained,
    })
}

/// A detached worker retains and reaps this trusted helper even if its caller is cancelled.
async fn capture(mut command: Command, limit: usize, timeout: Duration) -> Result<Vec<u8>> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().expect("piped observation stdout");
    tokio::spawn(async move {
        let result = tokio::time::timeout(timeout, async {
            let mut bytes = Vec::new();
            stdout
                .take(limit as u64 + 1)
                .read_to_end(&mut bytes)
                .await?;
            if bytes.len() > limit {
                return Err(Error::Invalid(
                    "Report group observation exceeded its limit".into(),
                ));
            }
            if !child.wait().await?.success() {
                return Err(Error::Invalid("Report group observer failed".into()));
            }
            Ok(bytes)
        })
        .await;
        if child.id().is_some() {
            child.kill().await?;
        }
        result.map_err(Error::from)?
    })
    .await
    .map_err(|_| Error::Invalid("Report group observer worker stopped".into()))?
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
