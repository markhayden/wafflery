//! Synthetic render processes; no provider, host library or shared host types.
mod document;
mod lifetime;
mod probe;
mod server;
use std::{
    io::{self, Read, Write},
    path::PathBuf,
};
pub fn run(mut args: impl Iterator<Item = String>) -> Result<(), Box<dyn std::error::Error>> {
    let mode = args.next().ok_or("render mode required")?;
    let run = PathBuf::from(args.next().ok_or("saved run required")?);
    if !run.is_dir() {
        return Err("run is missing".into());
    }
    if mode.starts_with("server") {
        return server::run(&mode, args);
    }
    if mode.starts_with("document") {
        return document::run(&mode, &run);
    }
    if matches!(
        mode.as_str(),
        "stuck" | "term" | "descendant" | "orphan" | "child-term" | "child-stuck"
    ) {
        return lifetime::run(&mode, &run);
    }
    match mode.as_str() {
        "echo" => println!(
            "{}",
            serde_json::json!({"args":args.collect::<Vec<_>>(),
            "env_names":std::env::vars_os().map(|(name,_)|name.to_string_lossy().into_owned()).collect::<Vec<_>>(),
            "cwd":std::env::current_dir()?})
        ),
        "early-error" => {
            eprintln!("Synthetic render failure");
            std::process::exit(17);
        }
        "stdout-flood" => {
            let _ = io::stdout().write_all(&vec![b'x'; 128 * 1024]);
        }
        "stderr-flood" => {
            let _ = io::stderr().write_all(&vec![b'e'; 128 * 1024]);
        }
        "hold" => {
            println!("synthetic renderer started");
            io::stdout().flush()?;
            let _ = io::stdin().read_to_end(&mut Vec::new())?;
        }
        _ => return Err("unknown synthetic render".into()),
    }
    Ok(())
}
