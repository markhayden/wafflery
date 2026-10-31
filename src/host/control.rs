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
    _lock: File,
    pub root: PathBuf,
    pub socket: PathBuf,
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
        lock.try_lock().map_err(|_| {
            io::Error::new(io::ErrorKind::WouldBlock, "another host owns this library")
        })?;
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
    }
}

pub fn request(root: &Path, operation: &str) -> io::Result<Value> {
    let root = root.canonicalize()?;
    let mut stream = UnixStream::connect(socket_path(&root)?)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let message = json!({"protocol": 1, "root": root, "operation": operation});
    stream.write_all(message.to_string().as_bytes())?;
    stream.shutdown(std::net::Shutdown::Write)?;
    let mut bytes = Vec::new();
    stream.take(16_385).read_to_end(&mut bytes)?;
    if bytes.len() > 16_384 {
        return Err(io::Error::other("oversized control response"));
    }
    let result: Value = serde_json::from_slice(&bytes)?;
    if result["protocol"] != 1
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
    owner: &Ownership,
    origin: &str,
    shutdown: tokio::sync::watch::Sender<bool>,
) -> io::Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::UnixListener::bind(&owner.socket)?;
    fs::set_permissions(&owner.socket, fs::Permissions::from_mode(0o600))?;
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
                || message["root"] != json!(root)
                || !matches!(message["operation"].as_str(), Some("status" | "quit"))
            {
                return Ok(());
            }
            let reply =
                json!({"protocol": 1, "root": root, "version": crate::VERSION, "origin": origin});
            stream.write_all(reply.to_string().as_bytes()).await?;
            stream.shutdown().await?;
            if message["operation"] == "quit" {
                let _ = shutdown.send(true);
            }
            Ok(())
        })
        .await;
    }
}
