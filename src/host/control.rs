//! Same-user readiness channel. This is private host IPC, not a tool interface.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        net::UnixStream,
    },
    path::{Path, PathBuf},
    time::Duration,
};

pub struct Ownership {
    _lock: OwnershipLock,
    pub root: PathBuf,
    pub socket: PathBuf,
}

// End the lock with this guard, not with the last inherited/duplicated descriptor.
// This also releases ownership if acquire fails after taking the lock.
struct OwnershipLock(File);
impl Drop for OwnershipLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

fn private_directory(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true).mode(0o700);
    match builder.create(path) {
        Ok(()) => (),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error),
    }
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(io::Error::other(
            "host directory must be a private, owned directory, not a symlink",
        ));
    }
    Ok(())
}

fn socket_path(root: &Path) -> io::Result<PathBuf> {
    let runtime = std::env::temp_dir().join(format!("wh-{}", nix::unistd::geteuid()));
    private_directory(&runtime)?;
    let digest = Sha256::digest(root.as_os_str().as_encoded_bytes());
    let name: String = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(runtime.join(format!("{name}.sock")))
}

impl Ownership {
    /// Publish only after binding the socket; a discovery file alone never proves readiness.
    pub fn publish(&self) -> io::Result<()> {
        fs::set_permissions(&self.socket, fs::Permissions::from_mode(0o600))?;
        let mut record = tempfile::NamedTempFile::new_in(&self.root)?;
        serde_json::to_writer(&mut record, &json!({"socket": self.socket}))?;
        record.as_file().sync_all()?;
        record
            .persist(self.root.join("host.json"))
            .map_err(|error| error.error)?;
        Ok(())
    }

    pub fn acquire(root: &Path) -> io::Result<Self> {
        private_directory(root)?;
        let root = root.canonicalize()?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(nix::libc::O_NOFOLLOW)
            .open(root.join("host.lock"))?;
        let metadata = lock.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != nix::unistd::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err(io::Error::other("invalid host lock"));
        }
        lock.try_lock().map_err(|error| match error {
            fs::TryLockError::WouldBlock => {
                io::Error::new(io::ErrorKind::WouldBlock, "another host owns this library")
            }
            fs::TryLockError::Error(error) => error,
        })?;
        let lock = OwnershipLock(lock);
        let socket = socket_path(&root)?;
        // A stale socket is removable only after winning the lifetime lock.
        match fs::symlink_metadata(&socket) {
            Ok(metadata) => {
                use std::os::unix::fs::FileTypeExt;
                if !metadata.file_type().is_socket()
                    || metadata.uid() != nix::unistd::geteuid().as_raw()
                {
                    return Err(io::Error::other("invalid private control socket"));
                }
                fs::remove_file(&socket)?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
        Ok(Self {
            _lock: lock,
            root,
            socket,
        })
    }
}

impl Drop for Ownership {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
        let _ = fs::remove_file(self.root.join("host.json"));
    }
}

pub fn request(root: &Path, operation: &str) -> io::Result<Value> {
    let root = root.canonicalize()?;
    let record = OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(root.join("host.json"))?;
    let metadata = record.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
        || metadata.len() > 4096
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid host discovery record",
        ));
    }
    let discovery: Value = serde_json::from_reader(record.take(4097))?;
    let socket = Path::new(
        discovery["socket"]
            .as_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing control socket"))?,
    );
    let directory =
        fs::symlink_metadata(socket.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "invalid control directory")
        })?)?;
    if !socket.is_absolute()
        || !directory.is_dir()
        || directory.uid() != nix::unistd::geteuid().as_raw()
        || directory.mode() & 0o077 != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control directory is not private",
        ));
    }
    let metadata = fs::symlink_metadata(socket)?;
    use std::os::unix::fs::FileTypeExt;
    if !metadata.file_type().is_socket()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid private control socket",
        ));
    }
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let message =
        json!({"protocol": 1, "product": "wafflery", "root": root, "operation": operation});
    stream.write_all(message.to_string().as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut bytes = Vec::new();
    stream.take(16_385).read_to_end(&mut bytes)?;
    if bytes.len() > 16_384 {
        return Err(io::Error::other("oversized control response"));
    }
    let result: Value = serde_json::from_slice(&bytes)?;
    if result["protocol"] != 1
        || result["product"] != "wafflery"
        || result["root"] != json!(root)
        || result["version"] != crate::VERSION
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "incompatible host identity; quit the existing host before restarting",
        ));
    }
    Ok(result)
}

pub async fn listen(
    listener: tokio::net::UnixListener,
    owner: &Ownership,
    origin: &str,
    shutdown: tokio::sync::watch::Sender<bool>,
    auth: super::auth::Auth,
    tools: std::sync::Arc<crate::tools::coordinator::Coordinator>,
) -> io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    loop {
        let (mut stream, _) = listener.accept().await?;
        let root = owner.root.clone();
        let origin = origin.to_owned();
        // Serial, bounded control requests keep the local channel inexpensive.
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            let mut bytes = Vec::new();
            (&mut stream).take(4097).read_to_end(&mut bytes).await?;
            if bytes.len() > 4096 {
                return Ok::<_, io::Error>(());
            }
            let message: Value = serde_json::from_slice(&bytes)?;
            if message["protocol"] != 1
                || message["product"] != "wafflery"
                || message["root"] != json!(root)
                || !matches!(
                    message["operation"].as_str(),
                    Some("status" | "quit" | "bootstrap")
                )
            {
                return Ok(());
            }
            let mut reply =
                json!({"protocol": 1, "product": "wafflery", "root": root, "version": crate::VERSION, "origin": origin});
            if message["operation"] == "bootstrap" {
                reply["bootstrap"] = json!(auth.issue()?);
            }
            if message["operation"] == "quit"
                && let Err(error) = tools.begin_shutdown(false, shutdown.clone()).await
            {
                reply["error"] = json!(error.to_string());
            }
            stream.write_all(reply.to_string().as_bytes()).await?;
            stream.shutdown().await?;
            Ok(())
        })
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropping_owner_releases_lock_even_while_a_duplicate_descriptor_exists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("library");
        let owner = Ownership::acquire(&root).unwrap();
        // Model the shared open-file description inherited during concurrent process creation.
        let duplicate = owner._lock.0.try_clone().unwrap();
        assert!(
            matches!(Ownership::acquire(&root), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
        );
        drop(owner);
        let replacement =
            Ownership::acquire(&root).expect("ownership ended despite a duplicated descriptor");
        drop(duplicate);
        assert!(
            matches!(Ownership::acquire(&root), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
        );
        drop(replacement);
        Ownership::acquire(&root).unwrap();
    }
}
