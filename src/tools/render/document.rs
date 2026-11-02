//! One tool-declared output file; never a directory grant or a reusable path capability.
use super::process::{ProcessPhase, ProcessState, STDOUT_LIMIT};
use crate::tools::{Error, Result};
use std::{
    fs::{File, Metadata, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    time::Duration,
};

const DOCUMENT_LIMIT: usize = 32 * 1024 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
static READ_SLOTS: LazyLock<Arc<tokio::sync::Semaphore>> =
    LazyLock::new(|| Arc::new(tokio::sync::Semaphore::new(16)));

#[derive(Clone)]
pub struct Document {
    bytes: Arc<[u8]>,
    name: String,
}
impl Document {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}
pub struct DocumentGrant {
    run: PathBuf,
    directory: File,
}
impl DocumentGrant {
    pub fn new(run: &Path) -> Result<Self> {
        if !run.is_absolute() {
            return Err(invalid(
                "Document rendering requires an absolute run directory",
            ));
        }
        let run = run.canonicalize()?;
        let directory = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
            .open(&run)?;
        Ok(Self { run, directory })
    }
    pub async fn read(&self, state: &ProcessState) -> Result<Document> {
        let path = output_path(state)?;
        let task = self.spawn_read(path, &READ_SLOTS, || {})?;
        tokio::time::timeout(READ_TIMEOUT, task)
            .await?
            .map_err(|_| invalid("Document reader stopped"))?
    }
    fn spawn_read(
        &self,
        path: PathBuf,
        slots: &Arc<tokio::sync::Semaphore>,
        after_metadata: impl FnOnce() + Send + 'static,
    ) -> Result<tokio::task::JoinHandle<Result<Document>>> {
        let permit = slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| Error::Conflict("Document readers are busy"))?;
        let grant = Self {
            run: self.run.clone(),
            directory: self.directory.try_clone()?,
        };
        // A filesystem stall cannot block the async host. The blocking task retains its
        // exact descriptor until it finishes; timeout never publishes partial content.
        Ok(tokio::task::spawn_blocking(move || {
            let _permit = permit;
            grant.read_file(&path, after_metadata)
        }))
    }
    fn check_run(&self) -> Result<()> {
        let current = std::fs::symlink_metadata(&self.run)?;
        if !current.is_dir()
            || !same_identity(&current, &self.directory.metadata()?)
            || self.run.canonicalize()? != self.run
        {
            return Err(invalid(
                "Run directory changed while rendering the document",
            ));
        }
        Ok(())
    }
    fn read_file(&self, path: &Path, after_metadata: impl FnOnce()) -> Result<Document> {
        self.check_run()?;
        let before = std::fs::symlink_metadata(path)?;
        if !before.is_file() {
            return Err(invalid(
                "Document output must be a regular file, not a symlink",
            ));
        }
        if before.len() > DOCUMENT_LIMIT as u64 {
            return Err(invalid("Document exceeds the 32 MiB limit"));
        }
        let canonical = path.canonicalize()?;
        let run_identity = self.directory.metadata()?;
        for ancestor in canonical.ancestors().skip(1) {
            if same_identity(&std::fs::metadata(ancestor)?, &run_identity) {
                return Err(invalid("Document output must be outside the run directory"));
            }
        }
        after_metadata();
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
            .open(&canonical)?;
        let opened = file.metadata()?;
        if !opened.is_file() || !same_snapshot(&before, &opened) {
            return Err(invalid("Document output changed before it could be opened"));
        }
        let mut bytes = Vec::new();
        (&file)
            .take(DOCUMENT_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > DOCUMENT_LIMIT {
            return Err(invalid("Document exceeds the 32 MiB limit"));
        }
        let after = file.metadata()?;
        let named = std::fs::symlink_metadata(path)?;
        if !same_snapshot(&opened, &after)
            || bytes.len() as u64 != after.len()
            || !named.is_file()
            || !same_snapshot(&after, &named)
            || path.canonicalize()? != canonical
        {
            return Err(invalid("Document output changed while it was being read"));
        }
        self.check_run()?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("Invalid document filename"))?;
        Ok(Document {
            bytes: bytes.into(),
            name: name.into(),
        })
    }
}
fn output_path(state: &ProcessState) -> Result<PathBuf> {
    if state.phase != ProcessPhase::Exited
        || state.exit_code != Some(0)
        || state.error.is_some()
        || !state.stdout.finished
        || !state.stderr.finished
        || state.stdout.overflow
        || state.stderr.overflow
        || state.stdout.error.is_some()
        || state.stderr.error.is_some()
        || state.stdout.bytes.len() > STDOUT_LIMIT
    {
        return Err(invalid(
            "Document renderer did not finish successfully with complete output",
        ));
    }
    let line = state
        .stdout
        .bytes
        .strip_suffix(b"\n")
        .ok_or_else(|| invalid("Document renderer must print one complete path line"))?;
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    if line.contains(&b'\n') || line.contains(&0) {
        return Err(invalid("Document renderer must print only one path"));
    }
    let text = std::str::from_utf8(line).map_err(|_| invalid("Document path must be UTF-8"))?;
    let path = PathBuf::from(text);
    if !path.is_absolute() {
        return Err(invalid("Document renderer must return an absolute path"));
    }
    Ok(path)
}
fn same_identity(a: &Metadata, b: &Metadata) -> bool {
    a.dev() == b.dev() && a.ino() == b.ino()
}
fn same_snapshot(a: &Metadata, b: &Metadata) -> bool {
    same_identity(a, b)
        && a.len() == b.len()
        && a.mtime() == b.mtime()
        && a.mtime_nsec() == b.mtime_nsec()
        && a.ctime() == b.ctime()
        && a.ctime_nsec() == b.ctime_nsec()
}
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
