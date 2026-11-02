//! Result-file publication shared by the independent task peers.
use serde_json::Value;
use std::{io, io::Write, path::Path};

/// Readers see either the previous complete snapshot or the new one. Staging in
/// the destination directory keeps rename on one filesystem; no crash durability
/// beyond the synthetic peer's lifetime is promised.
pub fn publish(path: &Path, value: &Value) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing result directory"))?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer(&mut staged, value)?;
    staged.flush()?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}
