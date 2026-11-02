//! Recheck the copied bytes priced by the tool before persisting a paid attempt.
use super::{Error, PreparedCall, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::PathBuf,
};

pub(super) async fn inputs(call: &PreparedCall) -> Result<()> {
    let directory = call
        .directory
        .clone()
        .ok_or(Error::Conflict("Prepared inputs are no longer owned"))?;
    let mut files = Vec::new();
    for (slot, ids) in &call.request.images {
        // Optional scalar slots may be supplied empty and are omitted from wire arguments.
        if ids.is_empty() {
            continue;
        }
        let binding = &call.arguments[slot];
        let bindings = binding
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_else(|| std::slice::from_ref(binding));
        if bindings.len() != ids.len()
            || call.inputs[slot]
                .as_array()
                .is_none_or(|inputs| inputs.len() != ids.len())
        {
            return Err(Error::Conflict("Prepared image count changed"));
        }
        for (index, binding) in bindings.iter().enumerate() {
            let path = binding["path"]
                .as_str()
                .ok_or(Error::Conflict("Prepared image path is missing"))?;
            let digest = binding["expect"]["file_sha256"]
                .as_str()
                .ok_or(Error::Conflict("Prepared image fingerprint is missing"))?;
            let size = call.inputs[slot][index]["asset"]["size"]
                .as_u64()
                .ok_or(Error::Conflict("Prepared image size is missing"))?;
            files.push((PathBuf::from(path), digest.to_owned(), size));
        }
    }
    tokio::task::spawn_blocking(move || {
        let metadata = fs::symlink_metadata(&directory)?;
        if !metadata.is_dir()
            || metadata.uid() != nix::unistd::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
            || directory.canonicalize()? != directory
        {
            return Err(Error::Conflict("Prepared input directory changed"));
        }
        for (path, expected, size) in files {
            if path.parent() != Some(directory.as_path())
                || size > crate::library::assets::MAX_BYTES
            {
                return Err(Error::Conflict("Prepared image escaped its owned bounds"));
            }
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
                .open(&path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file()
                || metadata.uid() != nix::unistd::geteuid().as_raw()
                || metadata.nlink() != 1
                || metadata.len() != size
            {
                return Err(Error::Conflict("Prepared image changed since estimation"));
            }
            let mut reader = file.take(size + 1);
            let mut hash = Sha256::new();
            let mut total = 0;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let count = reader.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                total += count as u64;
                hash.update(&buffer[..count]);
            }
            let digest: String = hash
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            if total != size || digest != expected {
                return Err(Error::Conflict("Prepared image changed since estimation"));
            }
        }
        Ok(())
    })
    .await
    .map_err(|error| Error::Invalid(format!("Prepared input verification failed: {error}")))?
}
