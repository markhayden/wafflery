use super::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
};

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

/// Explicit local registration. No shell expansion or inherited environment overrides.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
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
            || self.executable.metadata()?.permissions().mode() & 0o111 == 0
            || self.roots.len() > 32
            || self.args.len() > 128
            || self.args.iter().any(|arg| arg.contains('\0'))
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

/// Append validated records while the host is stopped. Never execute or replace a registered tool.
pub fn register(root: &std::path::Path, source: &std::path::Path) -> std::io::Result<usize> {
    use std::{collections::HashSet, io};
    let incoming = load(source)?;
    if incoming.is_empty() || incoming.len() > 32 {
        return Err(io::Error::other("Registration needs 1–32 tool descriptors"));
    }
    let incoming: Vec<_> = incoming
        .into_iter()
        .map(Descriptor::validate)
        .collect::<Result<_>>()
        .map_err(io::Error::other)?;
    // Coordinator adds its execution root before the client's 32-root validation.
    if incoming.iter().any(|tool| tool.roots.len() > 31) {
        return Err(io::Error::other(
            "At most 31 user roots may be registered; one root is reserved for host inputs",
        ));
    }
    let mut ids = HashSet::new();
    if incoming.iter().any(|tool| !ids.insert(tool.id.clone())) {
        return Err(io::Error::other("Duplicate tool IDs in registration"));
    }
    // The same lock protects startup and setup, including a concurrent launcher.
    let owner = crate::host::control::Ownership::acquire(root)?;
    let destination = owner.root.join("tools.json");
    let mut current = load(&destination)?;
    ids.clear();
    if current.len() > 32 || current.iter().any(|tool| !ids.insert(tool.id.clone())) {
        return Err(io::Error::other(
            "Existing registration has duplicate or excessive tools",
        ));
    }
    let mut added = 0;
    for tool in incoming {
        if let Some(existing) = current.iter().find(|existing| existing.id == tool.id) {
            if existing != &tool {
                return Err(io::Error::other(format!(
                    "Tool ID '{}' already has a different registration; no changes were written",
                    tool.id
                )));
            }
        } else {
            current.push(tool);
            added += 1;
        }
    }
    if current.len() > 32 {
        return Err(io::Error::other("At most 32 tools may be registered"));
    }
    if added == 0 {
        return Ok(0);
    }
    let bytes = serde_json::to_vec_pretty(&current)?;
    if bytes.len() > 1024 * 1024 {
        return Err(io::Error::other("Combined registration exceeds 1 MiB"));
    }
    let mut temporary = tempfile::NamedTempFile::new_in(&owner.root)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(destination)
        .map_err(|error| error.error)?;
    std::fs::File::open(&owner.root)?.sync_all()?;
    Ok(added)
}
