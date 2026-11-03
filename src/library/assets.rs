//! Capture bytes first, validate and publish without clobbering, then commit the reference.
use super::{Error, Result, db::Database};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const MAX_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Asset {
    pub id: String,
    pub sha256: String,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub media_type: String,
}

#[derive(Clone)]
pub struct AssetStore {
    pub database: Database,
    pub storage: Arc<tokio::sync::RwLock<()>>,
    root: PathBuf,
    decoder: Arc<tokio::sync::Semaphore>,
}

pub(super) fn private_directory(path: &Path) -> Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir()
        || metadata.uid() != nix::unistd::geteuid().as_raw()
        || metadata.mode() & 0o077 != 0
    {
        return Err(
            std::io::Error::other("library directories must be private owned directories").into(),
        );
    }
    Ok(())
}

impl AssetStore {
    pub fn open(root: &Path) -> Result<Self> {
        private_directory(root)?;
        let root = root.canonicalize()?;
        for directory in [
            "assets",
            "assets/sha256",
            "staging",
            "blobs",
            "blobs/sha256",
            "bundles",
            "bundles/sha256",
        ] {
            private_directory(&root.join(directory))?;
        }
        let database = Database::open(&root.join("library.sqlite3"))?;
        Ok(Self {
            database,
            storage: Arc::new(tokio::sync::RwLock::new(())),
            root,
            decoder: Arc::new(tokio::sync::Semaphore::new(1)),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub async fn import_path(&self, path: PathBuf) -> Result<Asset> {
        self.import_with_hook(path, || {}).await
    }

    async fn import_with_hook(
        &self,
        path: PathBuf,
        after_copy: impl FnOnce() + Send + 'static,
    ) -> Result<Asset> {
        let store = self.clone();
        tokio::spawn(async move { store.import_owned(path, after_copy).await })
            .await
            .map_err(|_| Error::Unavailable)?
    }
    async fn import_owned(
        &self,
        path: PathBuf,
        after_copy: impl FnOnce() + Send + 'static,
    ) -> Result<Asset> {
        let storage = self.storage.clone().read_owned().await;
        self.database.call(|_| Ok(())).await?;
        let permit = self
            .decoder
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Error::Unavailable)?;
        let root = self.root.clone();
        let (asset, _storage) = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            capture(&root, &path, after_copy).map(|asset| (asset, storage))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        self.database.call(move |connection| {
            connection.execute("INSERT INTO assets(id, sha256, size, width, height, media_type) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(sha256) DO NOTHING",
                rusqlite::params![asset.id, asset.sha256, asset.size, asset.width, asset.height, asset.media_type])?;
            Ok(connection.query_row("SELECT id, sha256, size, width, height, media_type FROM assets WHERE sha256=?1", [asset.sha256], from_row)?)
        }).await
    }

    pub async fn shutdown(&self) -> Result<()> {
        let _storage = self.storage.write().await;
        self.database.shutdown().await
    }

    pub async fn list(&self) -> Result<Vec<Asset>> {
        self.database.call(|connection| {
            let mut statement = connection.prepare("SELECT id, sha256, size, width, height, media_type FROM assets ORDER BY rowid DESC LIMIT 100")?;
            Ok(statement.query_map([], from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await
    }

    pub async fn metadata(&self, id: &str) -> Result<Asset> {
        if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::InvalidAsset("unknown image"));
        }
        let id = id.to_owned();
        self.database
            .call(move |connection| {
                Ok(connection.query_row(
                    "SELECT id, sha256, size, width, height, media_type FROM assets WHERE id=?1",
                    [id],
                    from_row,
                )?)
            })
            .await
    }

    pub async fn read(&self, id: &str) -> Result<(Asset, Vec<u8>)> {
        let asset = self.metadata(id).await?;
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || {
            let bytes = verified_bytes(&owned_path(&root, &asset.sha256)?, &asset.sha256)?;
            if bytes.len() != asset.size as usize {
                return Err(Error::InvalidAsset("stored image size changed"));
            }
            Ok((asset, bytes))
        })
        .await
        .map_err(|_| Error::Unavailable)?
    }
    /// A derived display aid; never used as run input or as original-image evidence.
    pub async fn thumbnail(&self, id: &str) -> Result<Vec<u8>> {
        let storage = self.storage.clone().read_owned().await;
        let asset = self.metadata(id).await?;
        let root = self.root.clone();
        let cache_root = root.clone();
        let digest = asset.sha256.clone();
        let (cached, storage) = tokio::task::spawn_blocking(move || {
            let cached =
                thumbnail_path(&cache_root, &digest).and_then(|path| read_thumbnail(&path));
            (cached, storage)
        })
        .await
        .map_err(|_| Error::Unavailable)?;
        if let Ok(bytes) = cached {
            return Ok(bytes);
        }
        let permit = self
            .decoder
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Error::Unavailable)?;
        tokio::task::spawn_blocking(move || {
            let (_storage, _permit) = (storage, permit);
            let path = thumbnail_path(&root, &asset.sha256)?;
            if let Ok(bytes) = read_thumbnail(&path) {
                return Ok(bytes);
            }
            // Legacy assets and damaged derived files are rebuilt from verified originals only.
            let original = verified_bytes(&owned_path(&root, &asset.sha256)?, &asset.sha256)?;
            if original.len() != asset.size as usize {
                return Err(Error::InvalidAsset("stored image size changed"));
            }
            let staged = tempfile::NamedTempFile::new_in(root.join("staging"))?;
            fs::write(staged.path(), original)?;
            let preview = super::image_preview::decode(staged.path())?;
            write_thumbnail(&root, &asset.sha256, &preview.bytes)?;
            Ok(preview.bytes)
        })
        .await
        .map_err(|_| Error::Unavailable)?
    }
}

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Asset> {
    Ok(Asset {
        id: row.get(0)?,
        sha256: row.get(1)?,
        size: row.get(2)?,
        width: row.get(3)?,
        height: row.get(4)?,
        media_type: row.get(5)?,
    })
}

fn owned_path(root: &Path, digest: &str) -> Result<PathBuf> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::InvalidAsset("invalid stored image fingerprint"));
    }
    Ok(root.join("assets/sha256").join(&digest[..2]).join(digest))
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn verified_bytes(path: &Path, digest: &str) -> Result<Vec<u8>> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::InvalidAsset("stored image is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES || hash(&bytes) != digest {
        return Err(Error::InvalidAsset(
            "stored image failed fingerprint verification",
        ));
    }
    Ok(bytes)
}

fn capture(root: &Path, source: &Path, after_copy: impl FnOnce()) -> Result<Asset> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut input = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NONBLOCK)
        .open(source)?;
    let before = input.metadata()?;
    if !before.is_file() || before.len() > MAX_BYTES {
        return Err(Error::InvalidAsset(
            "choose a PNG or JPEG file no larger than 128 MiB",
        ));
    }
    let mut staged = tempfile::NamedTempFile::new_in(root.join("staging"))?;
    let (digest, size) = copy_hash(&mut input, &mut staged)?;
    after_copy();
    let after = fs::metadata(source)?;
    if (
        before.dev(),
        before.ino(),
        before.len(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    ) != (
        after.dev(),
        after.ino(),
        after.len(),
        after.mtime(),
        after.mtime_nsec(),
        after.ctime(),
        after.ctime_nsec(),
    ) {
        return Err(Error::InvalidAsset(
            "source image changed during import; try again",
        ));
    }
    let preview = super::image_preview::decode(staged.path())?;
    let (width, height, media_type) = (preview.width, preview.height, preview.media_type);
    staged.as_file().sync_all()?;
    staged
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o444))?;
    let destination = owned_path(root, &digest)?;
    let directory = destination.parent().expect("owned asset parent");
    private_directory(directory)?;
    write_thumbnail(root, &digest, &preview.bytes)?;
    match staged.persist_noclobber(&destination) {
        Ok(_) => File::open(directory)?.sync_all()?,
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            verified_bytes(&destination, &digest)?;
        }
        Err(error) => return Err(error.error.into()),
    }
    let mut id = [0_u8; 16];
    getrandom::fill(&mut id).map_err(std::io::Error::other)?;
    Ok(Asset {
        id: id.iter().map(|byte| format!("{byte:02x}")).collect(),
        sha256: digest,
        size: size as u32, // copy_hash has enforced the 128 MiB bound.
        width,
        height,
        media_type: media_type.into(),
    })
}

fn copy_hash(input: &mut impl Read, output: &mut impl Write) -> Result<(String, u64)> {
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        if size > MAX_BYTES {
            return Err(Error::InvalidAsset("image exceeds 128 MiB"));
        }
        output.write_all(&buffer[..count])?;
        hash.update(&buffer[..count]);
    }
    Ok((
        hash.finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        size,
    ))
}

fn thumbnail_path(root: &Path, digest: &str) -> Result<PathBuf> {
    let original = owned_path(root, digest)?;
    let parent = original.parent().expect("owned asset parent");
    if parent.canonicalize()? != parent {
        return Err(Error::InvalidAsset("Asset directory is no longer owned"));
    }
    Ok(original.with_extension("thumbnail.png"))
}
fn read_thumbnail(path: &Path) -> Result<Vec<u8>> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK)
        .open(path)?;
    if !file.metadata()?.is_file() {
        return Err(Error::InvalidAsset("Thumbnail is not a regular file"));
    }
    let mut bytes = vec![];
    file.take(2 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err(Error::InvalidAsset("Thumbnail exceeds its size limit"));
    }
    let invalid = || Error::InvalidAsset("Derived thumbnail is damaged or exceeds its limits");
    let mut decoder = png::Decoder::new_with_limits(
        std::io::Cursor::new(&bytes),
        png::Limits {
            bytes: 4 * 1024 * 1024,
        },
    );
    decoder.ignore_checksums(false);
    let mut reader = decoder.read_info().map_err(|_| invalid())?;
    if reader.info().width > super::image_preview::EDGE
        || reader.info().height > super::image_preview::EDGE
    {
        return Err(invalid());
    }
    let size = reader
        .output_buffer_size()
        .filter(|size| *size <= 4 * 1024 * 1024)
        .ok_or_else(invalid)?;
    reader
        .next_frame(&mut vec![0; size])
        .map_err(|_| invalid())?;
    reader.finish().map_err(|_| invalid())?;
    Ok(bytes)
}
fn write_thumbnail(root: &Path, digest: &str, bytes: &[u8]) -> Result<()> {
    let path = thumbnail_path(root, digest)?;
    let mut staged = tempfile::NamedTempFile::new_in(root.join("staging"))?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    staged
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o444))?;
    staged
        .persist(&path)
        .map_err(|error| Error::Io(error.error))?;
    File::open(path.parent().expect("owned thumbnail parent"))?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_failure_and_mid_capture_source_change_are_errors() {
        struct FullDisk;
        impl Write for FullDisk {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from_raw_os_error(nix::libc::ENOSPC))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(copy_hash(&mut &b"bytes"[..], &mut FullDisk).is_err());
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("staging")).unwrap();
        let source = temp.path().join("source");
        fs::write(&source, "first").unwrap();
        assert!(matches!(
            capture(temp.path(), &source, || {
                fs::write(&source, "replacement").unwrap();
            }),
            Err(Error::InvalidAsset(
                "source image changed during import; try again"
            ))
        ));
        assert_eq!(
            fs::read_dir(temp.path().join("staging")).unwrap().count(),
            0
        );
    }
    #[tokio::test]
    async fn an_abandoned_import_keeps_publication_owned_until_its_database_record_exists() {
        let root = tempfile::tempdir().unwrap();
        let store = AssetStore::open(&root.path().join("Library")).unwrap();
        let source = root.path().join("source.png");
        image::RgbImage::from_pixel(8, 4, image::Rgb([20u8, 100, 200]))
            .save(&source)
            .unwrap();
        let (ready, waiting) = tokio::sync::oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let worker = store.clone();
        let upload = tokio::spawn(async move {
            worker
                .import_with_hook(source, move || {
                    ready.send(()).unwrap();
                    gate.recv().unwrap();
                })
                .await
        });
        waiting.await.unwrap();
        upload.abort();
        let _ = upload.await;
        let owned = store.storage.try_write().is_err();
        release.send(()).unwrap();
        assert!(
            owned,
            "dropping the upload waiter must not release publication ownership"
        );
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if store.list().await.unwrap().len() == 1 && store.storage.try_write().is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let asset = store.list().await.unwrap().remove(0);
        assert!(store.thumbnail(&asset.id).await.is_ok());
        assert_eq!(
            fs::read_dir(store.root().join("staging")).unwrap().count(),
            0
        );
        let cleanup = super::super::cleanup::Cleanup(store.clone());
        let selection = super::super::cleanup::Selection {
            unused_assets: true,
            ..Default::default()
        };
        let preview = cleanup.preview(selection.clone()).await.unwrap();
        cleanup.confirm(selection, preview.token).await.unwrap();
        assert!(!owned_path(store.root(), &asset.sha256).unwrap().exists());
        assert!(
            !thumbnail_path(store.root(), &asset.sha256)
                .unwrap()
                .exists()
        );
        store.database.shutdown().await.unwrap();
    }
    #[test]
    fn a_thumbnail_publication_failure_does_not_publish_an_untracked_original() {
        let root = tempfile::tempdir().unwrap();
        let store = AssetStore::open(&root.path().join("Library")).unwrap();
        let source = root.path().join("source.png");
        image::RgbImage::from_pixel(8, 4, image::Rgb([20u8, 100, 200]))
            .save(&source)
            .unwrap();
        let digest = hash(&fs::read(&source).unwrap());
        let destination = owned_path(store.root(), &digest).unwrap();
        private_directory(destination.parent().unwrap()).unwrap();
        fs::create_dir(destination.with_extension("thumbnail.png")).unwrap();
        assert!(capture(store.root(), &source, || {}).is_err());
        assert!(!destination.exists());
    }
    #[tokio::test]
    async fn verified_cached_thumbnails_do_not_wait_behind_source_decodes() {
        let root = tempfile::tempdir().unwrap();
        let store = AssetStore::open(&root.path().join("Library")).unwrap();
        let source = root.path().join("source.png");
        image::RgbImage::from_pixel(8, 4, image::Rgb([20u8, 100, 200]))
            .save(&source)
            .unwrap();
        let asset = store.import_path(source).await.unwrap();
        let held = store.decoder.acquire().await.unwrap();
        let cached = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            store.thumbnail(&asset.id),
        )
        .await;
        drop(held);
        assert!(cached.unwrap().is_ok());
        store.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn shutdown_waits_for_admitted_imports_and_rejects_late_ones_before_publication() {
        let root = tempfile::tempdir().unwrap();
        let library = root.path().join("Library");
        let store = AssetStore::open(&library).unwrap();
        let source = root.path().join("source.png");
        image::RgbImage::from_pixel(8, 4, image::Rgb([20u8, 100, 200]))
            .save(&source)
            .unwrap();
        let (ready, waiting) = tokio::sync::oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let worker = store.clone();
        let path = source.clone();
        let upload = tokio::spawn(async move {
            worker
                .import_with_hook(path, move || {
                    ready.send(()).unwrap();
                    gate.recv().unwrap();
                })
                .await
        });
        waiting.await.unwrap();
        let worker = store.clone();
        let shutdown = tokio::spawn(async move { worker.shutdown().await });
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while !shutdown.is_finished() && store.storage.try_read().is_ok() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let retained = !shutdown.is_finished();
        release.send(()).unwrap();
        assert!(retained, "shutdown must wait until import metadata commits");
        let asset = upload.await.unwrap().unwrap();
        shutdown.await.unwrap().unwrap();
        assert!(store.import_path(source).await.is_err());
        let reopened = AssetStore::open(&library).unwrap();
        assert_eq!(reopened.list().await.unwrap()[0].id, asset.id);
        assert!(reopened.thumbnail(&asset.id).await.is_ok());
        reopened.shutdown().await.unwrap();
    }
}
