pub mod assets;
pub mod db;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("library schema {0} requires a newer Wafflehouse")]
    NewerSchema(i64),
    #[error("the library worker is unavailable")]
    Unavailable,
    #[error("{0}")]
    InvalidAsset(&'static str),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, Error>;
