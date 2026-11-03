//! Launcher owns attachment and browser handoff; tool callers pass only a tool hint.
use super::control;
use std::{
    ffi::OsString,
    io,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct Options {
    pub start: bool,
    pub internal: bool,
    pub no_open: bool,
    pub root: PathBuf,
    pub tool: Option<String>,
    pub registration: Option<PathBuf>,
    pub replace_registration: bool,
}

impl Options {
    pub fn parse(arguments: Vec<OsString>) -> Result<Self, &'static str> {
        let mut options = Self::default();
        let mut arguments = arguments.into_iter().peekable();
        if arguments.peek().is_some_and(|arg| arg == "start") {
            options.start = true;
            arguments.next();
        }
        while let Some(argument) = arguments.next() {
            match argument.to_str() {
                Some("--internal-host") if !options.start && !options.internal => {
                    options.internal = true
                }
                Some("--no-open") if !options.no_open => options.no_open = true,
                Some("--replace-registration") if !options.replace_registration => {
                    options.replace_registration = true;
                }
                Some("--register") if options.registration.is_none() => {
                    options.registration = Some(
                        arguments
                            .next()
                            .filter(|path| !path.is_empty())
                            .ok_or("missing registration file")?
                            .into(),
                    );
                }
                Some("--data-dir") if options.root.as_os_str().is_empty() => {
                    options.root = arguments
                        .next()
                        .filter(|path| !path.is_empty())
                        .ok_or("missing data directory")?
                        .into();
                }
                Some("--tool") if options.tool.is_none() => {
                    let value = arguments
                        .next()
                        .and_then(|value| value.into_string().ok())
                        .ok_or("missing tool ID")?;
                    if value.is_empty()
                        || value.len() > 64
                        || !value
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
                    {
                        return Err(
                            "tool IDs must contain 1–64 letters, digits, underscores or hyphens",
                        );
                    }
                    options.tool = Some(value);
                }
                _ => return Err("unknown, duplicate or incompatible arguments"),
            }
        }
        if options.start && options.tool.is_some() {
            return Err("start accepts only --data-dir and --no-open");
        }
        if options.replace_registration && options.registration.is_none() {
            return Err("--replace-registration requires --register");
        }
        if options.registration.is_some()
            && (options.start || options.internal || options.no_open || options.tool.is_some())
        {
            return Err(
                "--register accepts only --data-dir and --replace-registration; it does not launch a workspace",
            );
        }
        if options.root.as_os_str().is_empty() {
            options.root = if let Some(root) = std::env::var_os("WAFFLERY_DATA_DIR") {
                let path = PathBuf::from(root);
                if !path.is_absolute() {
                    return Err("WAFFLERY_DATA_DIR must be a nonempty absolute path");
                }
                path
            } else {
                PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unavailable")?)
                    .join("Library/Application Support/Wafflery")
            };
        }
        if !options.root.is_absolute() {
            options.root = std::env::current_dir()
                .map_err(|_| "working directory unavailable")?
                .join(options.root);
        }
        Ok(options)
    }
}

pub fn attach(options: &Options) -> io::Result<String> {
    let state = match control::request(&options.root, "status") {
        Ok(state) => state,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            let mut child = crate::process::spawn_std(
                Command::new(std::env::current_exe()?)
                    .args(["--internal-host", "--no-open", "--data-dir"])
                    .arg(&options.root)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null()),
            )?;
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                match control::request(&options.root, "status") {
                    Ok(state) => break state,
                    Err(error)
                        if matches!(
                            error.kind(),
                            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                        ) => {}
                    Err(error) => return Err(error),
                }
                // A concurrent launcher may have won the lock. Allow its readiness to arrive.
                if let Some(status) = child.try_wait()?
                    && status.code() != Some(69)
                {
                    return Err(io::Error::other("host process failed during startup"));
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "host did not become ready; run wafflery start --no-open to inspect startup",
                    ));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        Err(error) => return Err(error),
    };
    let origin = state["origin"]
        .as_str()
        .ok_or_else(|| io::Error::other("host omitted its address"))?;
    let mut url = format!("{origin}/");
    if let Some(tool) = &options.tool {
        url.push_str(&format!("?tool={tool}"));
    }
    if options.no_open {
        println!("{url}");
    } else {
        let bootstrap = control::request(&options.root, "bootstrap")?;
        let token = bootstrap["bootstrap"]
            .as_str()
            .ok_or_else(|| io::Error::other("host omitted browser bootstrap"))?;
        url.push_str(&format!("#bootstrap={token}"));
        open_browser(&url).inspect_err(|_| {
            eprintln!("Host is ready at {origin}; browser handoff failed.");
        })?;
    }
    Ok(url)
}

pub fn open_browser(url: &str) -> io::Result<()> {
    let status = crate::process::spawn_std(
        Command::new("/usr/bin/open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )?
    .wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("macOS could not open the browser"))
    }
}

/// Shell display only; never used to execute a command.
pub(super) fn shell_quote(path: &std::path::Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}
