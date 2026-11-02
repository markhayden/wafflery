//! Serialize pipe creation with process creation on macOS.
//!
//! macOS lacks atomic pipe-plus-CLOEXEC creation. Concurrent spawns can inherit
//! each other's temporary pipe ends (rust-lang/rust#95584), preventing stdin EOF.
//! All host process launches use these functions; waiting happens outside the lock.
use std::io;

#[cfg(target_os = "macos")]
static SPAWN: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub fn spawn(command: &mut tokio::process::Command) -> io::Result<tokio::process::Child> {
    #[cfg(target_os = "macos")]
    let _guard = SPAWN.lock().unwrap_or_else(|error| error.into_inner());
    command.spawn()
}

pub fn spawn_std(command: &mut std::process::Command) -> io::Result<std::process::Child> {
    #[cfg(target_os = "macos")]
    let _guard = SPAWN.lock().unwrap_or_else(|error| error.into_inner());
    command.spawn()
}
