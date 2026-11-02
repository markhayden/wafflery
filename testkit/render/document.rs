//! One tool-owned standalone output, outside the saved evidence directory.
use std::{
    io::{self, Read},
    path::Path,
    time::Duration,
};
pub fn run(mode: &str, run: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::current_dir()?;
    if mode == "document-gated" {
        let (closed, eof) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = io::stdin().read_to_end(&mut Vec::new());
            let _ = closed.send(());
        });
        while !root.join("release-document").exists() {
            if eof.recv_timeout(Duration::from_millis(10)).is_ok() {
                return Ok(());
            }
        }
    }
    let output = if mode == "document-inside" {
        run.join("report.html")
    } else {
        root.join(format!("document-{}.html", std::process::id()))
    };
    if mode != "document-missing" {
        let body = if mode == "document-probe" {
            super::probe::html("http://127.0.0.1:9", "http://127.0.0.1:9/blocked")
        } else {
            "<!doctype html><title>Sample report</title><h1>Saved document</h1>".into()
        };
        std::fs::write(&output, body)?;
    }
    println!("{}", output.display());
    if mode == "document-extra" {
        println!("unexpected output");
    }
    Ok(())
}
