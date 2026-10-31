//! Capture bytes first, validate and publish without clobbering, then commit the reference.
use super::{Error, Result, db::Database};
use image::ImageDecoder;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};

pub const MAX_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PIXELS: u64 = 128_000_000;

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

fn private_directory(path: &Path) -> Result<()> {
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
        for directory in ["assets", "assets/sha256", "staging"] {
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
        let _storage = self.storage.read().await;
        let permit = self
            .decoder
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| Error::Unavailable)?;
        let root = self.root.clone();
        let asset = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            capture(&root, &path, || {})
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        self.database.call(move |connection| {
            connection.execute("INSERT INTO assets(id, sha256, size, width, height, media_type) VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(sha256) DO NOTHING",
                rusqlite::params![asset.id, asset.sha256, asset.size, asset.width, asset.height, asset.media_type])?;
            Ok(connection.query_row("SELECT id, sha256, size, width, height, media_type FROM assets WHERE sha256=?1", [asset.sha256], from_row)?)
        }).await
    }

    pub async fn list(&self) -> Result<Vec<Asset>> {
        self.database.call(|connection| {
            let mut statement = connection.prepare("SELECT id, sha256, size, width, height, media_type FROM assets ORDER BY rowid DESC LIMIT 100")?;
            Ok(statement.query_map([], from_row)?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await
    }

    pub async fn read(&self, id: &str) -> Result<(Asset, Vec<u8>)> {
        if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Error::InvalidAsset("unknown image"));
        }
        let id = id.to_owned();
        let asset = self
            .database
            .call(move |connection| {
                Ok(connection.query_row(
                    "SELECT id, sha256, size, width, height, media_type FROM assets WHERE id=?1",
                    [id],
                    from_row,
                )?)
            })
            .await?;
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
    let reader = image::ImageReader::new(BufReader::new(File::open(staged.path())?))
        .with_guessed_format()?;
    let media_type = match reader.format() {
        Some(image::ImageFormat::Png) => "image/png",
        Some(image::ImageFormat::Jpeg) => "image/jpeg",
        _ => {
            return Err(Error::InvalidAsset(
                "only PNG and JPEG images are supported",
            ));
        }
    };
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Error::InvalidAsset("image exceeds 128 megapixels"));
    }
    let mut limits = image::Limits::default(); // 512 MiB, including the output buffer.
    limits.reserve(decoder.total_bytes())?;
    decoder.set_limits(limits)?;
    drop(image::DynamicImage::from_decoder(decoder)?); // Decode completely before trusting the file.
    staged.as_file().sync_all()?;
    staged
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o444))?;
    let destination = owned_path(root, &digest)?;
    let directory = destination.parent().expect("owned asset parent");
    private_directory(directory)?;
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
}
