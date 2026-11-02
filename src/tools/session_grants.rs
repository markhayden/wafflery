//! Validate v0.4 effective access after the session schema and ledger checks.
use super::{Error, Result};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

fn directories(session: &Value) -> BTreeSet<&str> {
    session["allowed_dirs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|path| path.as_str().unwrap())
        .collect()
}

/// Requested directories are the canonical grants passed to this specific child.
/// Both observations have passed the v0.4 schema and existing ledger/identity checks.
/// A previous observation also passed this grant check for the same owned client.
/// A deliberate restart has a new client and no previous observation.
pub fn validate(session: &Value, requested: &[PathBuf], previous: Option<&Value>) -> Result<()> {
    let allowed = directories(session);
    if let Some(previous) = previous
        && (session["id"] != previous["id"] || allowed != directories(previous))
    {
        return Err(Error::Invalid(
            "Tool changed its effective access without an explicit new session".into(),
        ));
    }
    for name in allowed {
        let path = Path::new(name);
        // Reject names outside the requested lexical roots before resolving symlinks.
        if !path.is_absolute() || !requested.iter().any(|root| path.starts_with(root)) {
            return Err(Error::Invalid(
                "Tool effective access exceeds its requested input grants".into(),
            ));
        }
        // The directory list is frozen for this session. A later deletion or symlink change
        // must not hide its spending ledger; the tool still checks every actual file read.
        // A new client has no previous observation and validates its directories afresh.
        if previous.is_some() {
            continue;
        }
        let canonical = path.canonicalize().map_err(|_| {
            Error::Invalid("Tool effective access is not an existing canonical directory".into())
        })?;
        // Compare raw names: Path equality normalizes some non-canonical spellings.
        if canonical.as_os_str() != path.as_os_str() || !canonical.is_dir() {
            return Err(Error::Invalid(
                "Tool effective access is not an existing canonical directory".into(),
            ));
        }
    }
    Ok(())
}
