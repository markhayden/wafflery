//! Canonical, bounded invocation containers. No caller paths or tool storage walks.
use super::{Error, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub const MAX_MANIFEST: u64 = 8 * 1024 * 1024;
pub const MAX_MEMBER: u64 = 64 * 1024 * 1024;
pub const MAX_MEMBERS: usize = 64;
pub const MAX_TOTAL: u64 = 256 * 1024 * 1024;
pub struct CanonicalArchive {
    pub file: tempfile::NamedTempFile,
    pub sha256: String,
    pub size: u64,
}
/// Includes tar headers, padding and the two terminating zero blocks in the total limit.
fn layout(manifest: u64, sizes: impl Iterator<Item = u64>) -> Result<u64> {
    if manifest > MAX_MANIFEST {
        return Err(Error::Conflict("Invocation manifest exceeds 8 MiB"));
    }
    let mut count = 1;
    let mut total = 1024 + 512 + manifest.div_ceil(512) * 512;
    for size in sizes {
        count += 1;
        if count > MAX_MEMBERS {
            return Err(Error::Conflict("Invocation bundle exceeds 64 members"));
        }
        if size > MAX_MEMBER {
            return Err(Error::Conflict("Invocation bundle member exceeds 64 MiB"));
        }
        total = total
            .checked_add(512 + size.div_ceil(512) * 512)
            .ok_or(Error::Conflict("Invocation bundle size overflow"))?;
        if total > MAX_TOTAL {
            return Err(Error::Conflict(
                "Invocation bundle exceeds 256 MiB including tar headers",
            ));
        }
    }
    Ok(total)
}
fn safe_name(name: &str) -> bool {
    let Some((directory, base)) = name.split_once('/') else {
        return false;
    };
    matches!(directory, "inputs" | "artifacts")
        && base.len() <= 96
        && !base.is_empty()
        && base
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
        && !base.starts_with('.')
        && !base.contains("..")
}
pub(crate) async fn write(
    root: &Path,
    manifest: &Value,
    members: BTreeMap<String, Vec<u8>>,
) -> Result<CanonicalArchive> {
    let metadata = serde_json::to_vec(manifest)?;
    layout(
        metadata.len() as u64,
        members.values().map(|bytes| bytes.len() as u64),
    )?;
    if members.keys().any(|name| !safe_name(name)) {
        return Err(Error::InvalidAsset("Unsafe invocation bundle member name"));
    }
    let root = root.to_owned();
    tokio::task::spawn_blocking(move || {
        let mut file = tempfile::Builder::new()
            .prefix("invocation-")
            .suffix(".tar")
            .tempfile_in(root.join("staging"))?;
        {
            let mut tar = tar::Builder::new(file.as_file_mut());
            tar.follow_symlinks(false);
            for (name, bytes) in
                std::iter::once(("invocation.json".into(), metadata)).chain(members)
            {
                let mut header = tar::Header::new_ustar();
                header.set_size(bytes.len() as u64);
                header.set_mode(0o644);
                header.set_uid(0);
                header.set_gid(0);
                header.set_mtime(0);
                header.set_username("")?;
                header.set_groupname("")?;
                header.set_entry_type(tar::EntryType::Regular);
                tar.append_data(&mut header, name, bytes.as_slice())?;
            }
            tar.finish()?;
        }
        let size = file.as_file().metadata()?.len();
        if size > MAX_TOTAL {
            return Err(Error::Conflict("Invocation bundle exceeds 256 MiB"));
        }
        let mut digest = Sha256::new();
        file.as_file_mut().seek(SeekFrom::Start(0))?;
        let mut block = [0; 65536];
        loop {
            let n = file.as_file_mut().read(&mut block)?;
            if n == 0 {
                break;
            }
            digest.update(&block[..n]);
        }
        file.as_file_mut().seek(SeekFrom::Start(0))?;
        Ok(CanonicalArchive {
            file,
            size,
            sha256: hex(&digest.finalize()),
        })
    })
    .await
    .map_err(|_| Error::Unavailable)?
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_include_boundaries_and_archive_overhead_without_large_allocations() {
        assert!(layout(MAX_MANIFEST, std::iter::once(MAX_MEMBER)).is_ok());
        assert!(layout(MAX_MANIFEST + 1, std::iter::empty()).is_err());
        assert!(layout(1, std::iter::once(MAX_MEMBER + 1)).is_err());
        assert!(layout(1, std::iter::repeat_n(1, 63)).is_ok());
        assert!(layout(1, std::iter::repeat_n(1, 64)).is_err());
        assert!(layout(1, std::iter::repeat_n(MAX_MEMBER, 4)).is_err());
        assert!(
            layout(
                1,
                [MAX_MEMBER, MAX_MEMBER, MAX_MEMBER, MAX_MEMBER - 4096].into_iter()
            )
            .is_ok()
        );
        for name in [
            "../escape",
            "inputs/../x",
            "inputs/a/b",
            "inputs/.hidden",
            "artifacts/A.json",
            "invocation.json",
        ] {
            assert!(!safe_name(name), "{name}");
        }
        assert!(!safe_name(&format!("inputs/{}", "a".repeat(97))));
    }
}
