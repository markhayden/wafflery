//! Cooperative process-group fixtures; no shared host types.
use std::{
    io::{self, Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

pub fn run(mode: &str, run: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if matches!(mode, "descendant" | "orphan") {
        let child_mode = if mode == "orphan" {
            "child-stuck"
        } else {
            "child-term"
        };
        let child = Command::new(std::env::current_exe()?)
            .args(["--render-test", child_mode])
            .arg(run)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let marker = std::env::current_dir()?.join("descendant-ready");
        let until = std::time::Instant::now() + Duration::from_secs(5);
        while !marker.exists() {
            if std::time::Instant::now() >= until {
                return Err("descendant did not start".into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        println!("descendant {}", child.id());
        io::stdout().flush()?;
        // Intentionally leave child ownership to the host's process group on leader exit.
        if mode == "descendant" {
            let _ = io::stdin().read_to_end(&mut Vec::new())?;
        }
        return Ok(());
    }
    tokio::runtime::Runtime::new()?.block_on(async {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        if mode.starts_with("child-") {
            std::fs::write(
                std::env::current_dir()?.join("descendant-ready"),
                std::process::id().to_string(),
            )?;
        } else {
            println!("synthetic renderer started");
            eprintln!("synthetic renderer started");
            io::stdout().flush()?;
        }
        let expires = tokio::time::Instant::now() + Duration::from_secs(30);
        loop {
            tokio::select! {
                _ = term.recv() => (),
                // Red tests must not leave orphan fixtures behind if the host loses ownership.
                _ = tokio::time::sleep_until(expires), if mode != "stuck" => return Err("orphan fixture expired".into()),
            }
            if matches!(mode, "term" | "child-term") {
                return Ok(());
            }
        }
    })
}
