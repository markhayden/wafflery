pub mod assets;
pub mod attempts;
pub mod baselines;
mod blob_cleanup;
pub mod blob_links;
pub mod blobs;
pub mod cleanup;
pub mod db;
pub mod examples;
pub mod export;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Conflict(&'static str),
    #[error("{0}")]
    Missing(&'static str),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("library schema {0} requires a newer Wafflery")]
    NewerSchema(i64),
    #[error("library schema revision {0} is invalid")]
    InvalidSchema(i64),
    #[error("the library worker is unavailable")]
    Unavailable,
    #[error("{0}")]
    InvalidAsset(&'static str),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}

pub type Result<T> = std::result::Result<T, Error>;
