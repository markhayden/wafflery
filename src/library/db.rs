//! One bounded worker owns the SQLite connection. No DB server or async-thread blocking queries.
use super::{Error, Result};
use rusqlite::{Connection, OpenFlags};
use std::{
    fs,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

type Job = Box<dyn FnOnce(&mut Connection) + Send>;
enum Message {
    Work(Job),
    Stop(oneshot::Sender<()>),
}

struct Inner {
    sender: mpsc::Sender<Message>,
    thread: Mutex<Option<JoinHandle<()>>>,
    stopping: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct Database(Arc<Inner>);

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let path = path.to_owned();
        let (sender, mut receiver) = mpsc::channel::<Message>(64);
        let (ready, opened) = std::sync::mpsc::sync_channel(1);
        let stopping = Arc::new(AtomicBool::new(false));
        let worker_stopping = stopping.clone();
        let thread = std::thread::Builder::new()
            .name("wafflehouse-library".into())
            .spawn(move || {
                let mut connection = match open_connection(&path) {
                    Ok(connection) => {
                        let _ = ready.send(Ok(()));
                        connection
                    }
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let acknowledgement = loop {
                    match receiver.blocking_recv() {
                        Some(Message::Work(job)) => {
                            if !worker_stopping.load(Ordering::Acquire) {
                                job(&mut connection);
                            }
                        }
                        Some(Message::Stop(reply)) => break Some(reply),
                        None => break None,
                    }
                };
                drop(connection);
                if let Some(reply) = acknowledgement {
                    let _ = reply.send(());
                }
            })?;
        opened.recv().map_err(|_| Error::Unavailable)??;
        Ok(Self(Arc::new(Inner {
            sender,
            thread: Mutex::new(Some(thread)),
            stopping,
        })))
    }

    pub async fn call<T, F>(&self, operation: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    {
        if self.0.stopping.load(Ordering::Acquire) {
            return Err(Error::Unavailable);
        }
        let (reply, result) = oneshot::channel();
        self.0
            .sender
            .send(Message::Work(Box::new(move |connection| {
                let _ = reply.send(operation(connection));
            })))
            .await
            .map_err(|_| Error::Unavailable)?;
        result.await.map_err(|_| Error::Unavailable)?
    }

    pub async fn shutdown(&self) -> Result<()> {
        self.0.stopping.store(true, Ordering::Release);
        let handle = self.0.thread.lock().expect("worker handle").take();
        if let Some(handle) = handle {
            let (reply, finished) = oneshot::channel();
            let _ = self.0.sender.send(Message::Stop(reply)).await;
            tokio::time::timeout(Duration::from_secs(6), finished)
                .await
                .map_err(|_| Error::Unavailable)?
                .map_err(|_| Error::Unavailable)?;
            tokio::task::spawn_blocking(move || handle.join())
                .await
                .map_err(|_| Error::Unavailable)?
                .map_err(|_| Error::Unavailable)?;
        }
        Ok(())
    }
}

fn open_connection(path: &Path) -> Result<Connection> {
    // macOS /var aliases /private/var. Resolve the parent, but never follow the DB file itself.
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("database has no parent"))?
        .canonicalize()?;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("database has no file name"))?;
    let canonical = parent.join(name);
    let path = canonical.as_path();
    // Create privately, refuse symlinks, and never interpret a filesystem path as a SQLite URI.
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("invalid database file").into());
    }
    let populated = file.metadata()?.len() > 0;
    drop(file);
    let mut connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    const MIGRATIONS: &[&str] = &[
        include_str!("migrations/001-library.sql"),
        include_str!("migrations/002-assets.sql"),
    ];
    let latest = MIGRATIONS.len() as i64;
    if version < 0 {
        return Err(Error::InvalidSchema(version));
    }
    if version > latest {
        return Err(Error::NewerSchema(version));
    }
    if version < latest && populated {
        let backup = tempfile::Builder::new()
            .prefix(&format!("before-v{latest}-"))
            .suffix(".sqlite3")
            .tempfile_in(
                path.parent()
                    .ok_or_else(|| std::io::Error::other("database has no parent"))?,
            )?;
        connection.backup(
            rusqlite::MAIN_DB,
            backup.path(),
            rusqlite::backup::NO_PROGRESS,
        )?;
        backup.as_file().sync_all()?;
        backup.keep().map_err(|error| error.error)?;
    }
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "journal_mode", "DELETE")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "trusted_schema", false)?;
    if version < latest {
        let transaction = connection.transaction()?;
        for migration in &MIGRATIONS[version as usize..] {
            transaction.execute_batch(migration)?;
        }
        transaction.pragma_update(None, "user_version", latest)?;
        transaction.commit()?;
    }
    Ok(connection)
}
