use super::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{io::Read, os::unix::fs::OpenOptionsExt};

pub fn load(path: &std::path::Path) -> std::io::Result<Vec<Descriptor>> {
    let file = match std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(error) => return Err(error),
    };
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("tools.json must be a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err(std::io::Error::other("tools.json exceeds 1 MiB"));
    }
    serde_json::from_slice(&bytes).map_err(std::io::Error::other)
}

/// Explicit, local-only development registration. No shell expansion or environment overrides.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub id: String,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub roots: Vec<PathBuf>,
}
impl Descriptor {
    pub fn validate(mut self) -> Result<Self> {
        if self.id.is_empty()
            || self.id.len() > 64
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err(Error::Invalid(
                "Registration id must be 1–64 letters, digits, underscores or hyphens".into(),
            ));
        }
        if !self.executable.is_absolute()
            || !self.cwd.is_absolute()
            || self.roots.iter().any(|p| !p.is_absolute())
        {
            return Err(Error::Invalid("Registration paths must be absolute".into()));
        }
        self.executable = self.executable.canonicalize()?;
        self.cwd = self.cwd.canonicalize()?;
        if !self.cwd.is_dir()
            || !self.executable.is_file()
            || self.roots.len() > 32
            || self.args.len() > 128
        {
            return Err(Error::Invalid(
                "Invalid executable, workspace or registration limits".into(),
            ));
        }
        for root in &mut self.roots {
            *root = root.canonicalize()?;
            if !root.is_dir() {
                return Err(Error::Invalid("MCP roots must be directories".into()));
            }
        }
        Ok(self)
    }
}
