//! Opt-in terminal diagnostics. Detached hosts never capture tool stderr.
use std::{
    fmt,
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, ChildStderr},
};

static ENABLED: AtomicBool = AtomicBool::new(false);
pub(super) fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}
pub(crate) fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}
pub(crate) fn event(message: fmt::Arguments<'_>) {
    if enabled() {
        let _ = writeln!(std::io::stderr().lock(), "[wafflery] {message}");
    }
}

pub(crate) struct ToolOutput(Mutex<Option<tokio::task::JoinHandle<()>>>);
impl ToolOutput {
    pub fn watch(
        id: &str,
        child: &Arc<tokio::sync::Mutex<Child>>,
        stderr: ChildStderr,
        pid: u32,
    ) -> Self {
        event(format_args!("Tool {id} (pid {pid}) launched"));
        let id = id.to_owned();
        let child = Arc::downgrade(child);
        Self(Mutex::new(Some(tokio::spawn(async move {
            let forward = forward(&id, stderr);
            tokio::pin!(forward);
            let mut drained = false;
            loop {
                tokio::select! {
                    _ = &mut forward, if !drained => drained = true,
                    _ = tokio::time::sleep(Duration::from_millis(50)) => {
                        let status = match child.upgrade() {
                            Some(child) => match child.try_lock() {
                                Ok(mut child) => child.try_wait(),
                                // Keep polling stderr while close/force-stop owns the child lock.
                                Err(_) => continue,
                            },
                            None => return,
                        };
                        match status {
                            Ok(Some(status)) => {
                                // A descendant must not hold shutdown hostage by inheriting stderr.
                                if !drained {
                                    let _ = tokio::time::timeout(Duration::from_millis(250), &mut forward).await;
                                }
                                event(format_args!("Tool {id} (pid {pid}) exited: {status}"));
                                return;
                            }
                            Ok(None) => (),
                            Err(error) => { event(format_args!("Tool {id} (pid {pid}) status error: {error}")); return; }
                        }
                    }
                }
            }
        }))))
    }
    pub async fn finish(&self) {
        let task = self.0.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(task) = task {
            let _ = task.await;
        }
    }
}
impl Drop for ToolOutput {
    fn drop(&mut self) {
        if let Some(task) = self.0.get_mut().unwrap_or_else(|e| e.into_inner()).take() {
            task.abort();
        }
    }
}

async fn forward(id: &str, mut stderr: ChildStderr) {
    let mut buffer = [0; 4096];
    let mut line = Vec::new();
    loop {
        match stderr.read(&mut buffer).await {
            Ok(0) => break,
            Ok(count) => {
                for byte in &buffer[..count] {
                    if *byte == b'\n' {
                        let _ = writeln!(
                            std::io::stderr().lock(),
                            "[{id}] {}",
                            String::from_utf8_lossy(&line)
                        );
                        line.clear();
                    } else {
                        line.push(*byte);
                        // Bound memory even when a peer emits no newlines.
                        if line.len() == 8192 {
                            let _ = writeln!(
                                std::io::stderr().lock(),
                                "[{id}] {}",
                                String::from_utf8_lossy(&line)
                            );
                            line.clear();
                        }
                    }
                }
            }
            Err(error) => {
                event(format_args!("Tool {id} stderr read failed: {error}"));
                break;
            }
        }
    }
    if !line.is_empty() {
        let _ = writeln!(
            std::io::stderr().lock(),
            "[{id}] {}",
            String::from_utf8_lossy(&line)
        );
    }
}
