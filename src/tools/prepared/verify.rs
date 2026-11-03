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
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    for (slot, index, asset) in call.request.bound_images() {
        *counts.entry(slot).or_default() += 1;
        let argument = &call.arguments[slot];
        let binding = if argument.is_array() {
            &argument[index]
        } else {
            argument
        };
        let evidence = call.inputs[slot]
            .as_array()
            .and_then(|entries| {
                entries
                    .iter()
                    .enumerate()
                    .find(|(i, entry)| {
                        entry["argument_index"]
                            .as_u64()
                            .map_or(*i, |value| value as usize)
                            == index
                    })
                    .map(|(_, entry)| entry)
            })
            .ok_or(Error::Conflict("Prepared image evidence changed"))?;
        if evidence["asset"]["id"] != asset {
            return Err(Error::Conflict("Prepared image asset changed"));
        }
        let path = binding["path"]
            .as_str()
            .ok_or(Error::Conflict("Prepared image path is missing"))?;
        let digest = binding["expect"]["file_sha256"]
            .as_str()
            .ok_or(Error::Conflict("Prepared image fingerprint is missing"))?;
        let size = evidence["asset"]["size"]
            .as_u64()
            .ok_or(Error::Conflict("Prepared image size is missing"))?;
        files.push((PathBuf::from(path), digest.to_owned(), size));
    }
    for (slot, count) in counts {
        if call.inputs[slot]
            .as_array()
            .is_none_or(|entries| entries.len() != count)
        {
            return Err(Error::Conflict("Prepared image count changed"));
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
