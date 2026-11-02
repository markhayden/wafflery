//! Reclaim only identifiable, unclaimed preparation inputs; never adopt arbitrary directories.
use super::{Error, Result, validate_id};
use crate::library::{assets::AssetStore, attempts::AttemptStore};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
const MARKER: &str = ".wafflery-preparation.json";
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    version: u8,
    id: String,
}

pub(super) fn mark(directory: &Path, id: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(directory.join(MARKER))?;
    serde_json::to_writer(
        &mut file,
        &Marker {
            version: 1,
            id: id.into(),
        },
    )?;
    file.sync_all()
}
fn marked(directory: &Path, id: &str) -> io::Result<bool> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(directory.join(MARKER))
    {
        Ok(file) => file,
        Err(error)
            if matches!(
                error.raw_os_error(),
                Some(nix::libc::ENOENT | nix::libc::ELOOP)
            ) =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
        || metadata.len() > 1024
        || metadata.nlink() != 1
    {
        return Ok(false);
    }
    let mut bytes = vec![];
    file.take(1025).read_to_end(&mut bytes)?;
    Ok(serde_json::from_slice::<Marker>(&bytes).is_ok_and(|m| m.version == 1 && m.id == id))
}
pub(crate) fn unmark(directory: &Path, id: &str) -> io::Result<()> {
    if !directory.join(MARKER).try_exists()? {
        return Ok(());
    }
    if !marked(directory, id)? {
        return Err(io::Error::other(
            "Unrecognized preparation ownership marker",
        ));
    }
    fs::remove_file(directory.join(MARKER))
}
fn private_directory(path: &Path) -> io::Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(metadata.is_dir()
        && metadata.uid() == nix::unistd::geteuid().as_raw()
        && metadata.mode() & 0o077 == 0
        && path.canonicalize()? == path)
}
fn input_name(name: &str) -> bool {
    let Some((stem, extension)) = name.rsplit_once('.') else {
        return false;
    };
    if !matches!(extension, "png" | "jpg") {
        return false;
    }
    let Some(indices) = stem.strip_prefix("input-") else {
        return false;
    };
    let parts: Vec<_> = indices.split('-').collect();
    (1..=2).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}
pub(crate) async fn recover(assets: &AssetStore, attempts: &AttemptStore) -> Result<()> {
    let execution = assets.root().join("execution");
    if !private_directory(&execution)? {
        return Err(Error::Invalid(
            "Execution directory must be private, owned and not a symlink".into(),
        ));
    }
    for entry in fs::read_dir(&execution)? {
        let path = entry?.path();
        let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if validate_id(id).is_err() {
            continue;
        }
        // A recorded attempt is outside this recovery domain, even if its marker is damaged.
        match attempts.get(id).await {
            Ok(_) => continue,
            Err(crate::library::Error::Missing(_)) => (),
            Err(error) => return Err(error.into()),
        }
        if !private_directory(&path)? || !marked(&path, id)? {
            continue;
        }
        let mut files = Vec::new();
        let mut owned = true;
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name();
            let metadata = fs::symlink_metadata(entry.path())?;
            if !metadata.is_file()
                || metadata.uid() != nix::unistd::geteuid().as_raw()
                || metadata.nlink() != 1
                || !name
                    .to_str()
                    .is_some_and(|name| name == MARKER || input_name(name))
            {
                owned = false;
                break;
            }
            if name != MARKER {
                files.push(entry.path());
            }
        }
        if !owned {
            continue;
        }
        // Validate the entire directory first. Unlink only known regular inputs, then our marker.
        for file in files {
            fs::remove_file(file)?;
        }
        unmark(&path, id)?;
        fs::remove_dir(path)?;
    }
    Ok(())
}
