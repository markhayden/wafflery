//! Resolve declared argv from saved evidence. No render executes during resolution.
pub mod process;
use super::{Error, Result, environment, registration::Descriptor, validate_contract};
use crate::library::attempts::Attempt;
use serde::Deserialize;
use std::{
    fs::{File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RenderKind {
    Document,
    Server,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclaredRender {
    #[serde(rename = "name")]
    _name: String,
    kind: RenderKind,
    command: Vec<String>,
    ready_timeout_s: Option<u64>,
}

pub struct ResolvedRender {
    descriptor: Descriptor,
    args: Vec<String>,
    run_dir: PathBuf,
    directory: File,
    kind: RenderKind,
    timeout: Option<Duration>,
}
impl ResolvedRender {
    /// `descriptor` must be the canonical descriptor of the serving client.
    pub fn resolve(
        descriptor: &Descriptor,
        attempt: &Attempt,
        name: &str,
        host_origin: &str,
    ) -> Result<Self> {
        if attempt.request.tool != descriptor.id {
            return Err(invalid("Saved run belongs to another registration"));
        }
        if descriptor.clone().validate()? != *descriptor {
            return Err(invalid(
                "Serving registration paths changed before rendering",
            ));
        }
        let descriptor = descriptor.clone();
        let document = &attempt.declaration["document"];
        validate_contract("urn:table-manners:core:description", document)?;
        let result = attempt
            .result
            .as_ref()
            .ok_or(Error::Missing("No saved tool result is available"))?;
        validate_contract("urn:table-manners:core:result", result)?;
        if result["tool"]["name"] != document["tool"]["name"]
            || result["tool"]["version"] != document["tool"]["version"]
            || result["action"] != attempt.request.action
        {
            return Err(invalid("Saved result conflicts with its tool or action"));
        }
        let actions: Vec<_> = document["actions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|a| a["name"] == attempt.request.action)
            .collect();
        if actions.len() != 1 {
            return Err(invalid("Saved action is missing or ambiguous"));
        }
        let choices: Vec<_> = actions[0]["renders"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["name"] == name)
            .collect();
        if choices.len() != 1 {
            return Err(invalid("Render is missing or ambiguous"));
        }
        let declared: DeclaredRender = serde_json::from_value(choices[0].clone())?;
        let tool = document["tool"]["name"]
            .as_str()
            .ok_or_else(|| invalid("Missing tool name"))?;
        if declared.command.first().map(String::as_str) != Some(tool)
            || Path::new(tool).components().count() != 1
            || tool.contains('/')
        {
            return Err(invalid(
                "Render must name the serving tool, not another executable",
            ));
        }
        if declared.command.len() > 128
            || declared.command.iter().map(String::len).sum::<usize>() > 128 * 1024
        {
            return Err(invalid("Render command exceeds host limits"));
        }
        let raw = result["run"]["dir"]
            .as_str()
            .ok_or(Error::Missing("This run has no local report directory"))?;
        if !Path::new(raw).is_absolute() {
            return Err(invalid("Run directory must be absolute"));
        }
        let run_dir = Path::new(raw).canonicalize()?;
        if !descriptor
            .roots
            .iter()
            .any(|root| run_dir.starts_with(root))
        {
            return Err(invalid("Run directory is outside registered folders"));
        }
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
            .open(&run_dir)?;
        let run_text = run_dir
            .to_str()
            .ok_or_else(|| invalid("Run directory cannot be represented as an argument"))?;
        let origin = checked_origin(host_origin)?;
        let mut run_args = 0;
        let mut frame_args = 0;
        let args = declared
            .command
            .iter()
            .skip(1)
            .map(|arg| match arg.as_str() {
                "{run.dir}" => {
                    run_args += 1;
                    Ok(run_text.to_owned())
                }
                "{frame_ancestor}" if declared.kind == RenderKind::Server => {
                    frame_args += 1;
                    Ok(origin.clone())
                }
                _ if arg.contains(['{', '}', '\0']) => {
                    Err(invalid("Unknown or embedded render placeholder"))
                }
                _ => Ok(arg.clone()),
            })
            .collect::<Result<Vec<_>>>()?;
        if args.iter().map(String::len).sum::<usize>() > 128 * 1024 {
            return Err(invalid("Expanded render command exceeds host limits"));
        }
        if run_args == 0 || (declared.kind == RenderKind::Server && frame_args == 0) {
            return Err(invalid(
                "Render must bind its run directory and server framing origin",
            ));
        }
        Ok(Self {
            descriptor,
            args,
            run_dir,
            directory,
            kind: declared.kind,
            timeout: declared.ready_timeout_s.map(Duration::from_secs),
        })
    }
    pub fn kind(&self) -> RenderKind {
        self.kind
    }
    pub fn ready_timeout(&self) -> Option<Duration> {
        self.timeout
    }
    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }
    /// Recheck the directory identity immediately before the owner spawns this command.
    /// This is a host authorization check, not an OS sandbox for the installed tool.
    pub fn command(&self) -> Result<Command> {
        if self.descriptor.clone().validate()? != self.descriptor {
            return Err(invalid(
                "Serving registration paths changed before rendering",
            ));
        }
        let current = std::fs::symlink_metadata(&self.run_dir)?;
        let pinned = self.directory.metadata()?;
        if !current.is_dir()
            || current.dev() != pinned.dev()
            || current.ino() != pinned.ino()
            || self.run_dir.canonicalize()? != self.run_dir
        {
            return Err(invalid("Run directory changed before rendering"));
        }
        let mut command = Command::new(&self.descriptor.executable);
        environment::apply(&mut command, &[]);
        command.args(&self.args).current_dir(&self.descriptor.cwd);
        Ok(command)
    }
}
fn checked_origin(value: &str) -> Result<String> {
    let origin = url::Url::parse(value).map_err(|_| invalid("Invalid report framing origin"))?;
    if origin.scheme() != "http"
        || origin.host_str() != Some("127.0.0.1")
        || origin.port().is_none_or(|port| port == 0)
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.query().is_some()
        || origin.fragment().is_some()
        || origin.path() != "/"
        || origin.origin().ascii_serialization() != value
    {
        return Err(invalid(
            "Report framing requires the exact local host origin",
        ));
    }
    Ok(value.to_owned())
}
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
#[cfg(test)]
#[path = "render/tests.rs"]
mod tests;

#[cfg(test)]
#[allow(dead_code)]
#[path = "../../testkit/fixtures.rs"]
mod fixtures;
