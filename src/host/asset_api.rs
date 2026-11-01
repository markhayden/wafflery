use crate::library::{
    Error,
    assets::{Asset, AssetStore, MAX_BYTES},
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use http_body_util::BodyExt;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

#[derive(Clone)]
struct Assets {
    store: AssetStore,
    uploads: Arc<tokio::sync::Semaphore>,
}

pub fn router(store: AssetStore) -> Router {
    Router::new()
        .route("/api/assets", get(list).post(upload))
        .route("/api/assets/{id}", get(preview))
        .route("/api/assets/{id}/download", get(download))
        .with_state(Assets {
            store,
            uploads: Arc::new(tokio::sync::Semaphore::new(2)),
        })
}

struct ApiError(StatusCode, &'static str, &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        super::api::failure(self.0, self.1, self.2)
    }
}
impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        match error {
            Error::InvalidAsset(message) => {
                Self(StatusCode::UNPROCESSABLE_ENTITY, "invalid_image", message)
            }
            Error::Image(_) => Self(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_image",
                "Choose a complete PNG or JPEG image within the image limits.",
            ),
            Error::Sql(rusqlite::Error::QueryReturnedNoRows) => Self(
                StatusCode::NOT_FOUND,
                "image_missing",
                "This image is no longer available.",
            ),
            Error::Io(error) if error.kind() == std::io::ErrorKind::NotFound => Self(
                StatusCode::NOT_FOUND,
                "image_missing",
                "This image is no longer available.",
            ),
            _ => Self(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_failure",
                "The image could not be saved or read. Check available disk space and folder access.",
            ),
        }
    }
}

async fn list(State(state): State<Assets>) -> Result<Json<Vec<Asset>>, ApiError> {
    Ok(Json(state.store.list().await?))
}

async fn upload(
    State(state): State<Assets>,
    request: Request,
) -> Result<(StatusCode, Json<Asset>), ApiError> {
    let oversized = || {
        ApiError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "image_too_large",
            "Images must be no larger than 128 MiB.",
        )
    };
    if request
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|size| size > MAX_BYTES)
    {
        return Err(oversized());
    }
    if request.headers().get("x-wafflery-upload") != Some(&HeaderValue::from_static("1")) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "upload_header_required",
            "Use the image upload control.",
        ));
    }
    let _permit = state.uploads.try_acquire().map_err(|_| {
        ApiError(
            StatusCode::CONFLICT,
            "upload_busy",
            "Two images are already importing. Try again when they finish.",
        )
    })?;
    let staged =
        tempfile::NamedTempFile::new_in(state.store.root().join("staging")).map_err(Error::from)?;
    let mut file = tokio::fs::File::from_std(staged.reopen().map_err(Error::from)?);
    let mut body = request.into_body();
    let mut size = 0_u64;
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|_| {
            ApiError(
                StatusCode::BAD_REQUEST,
                "upload_interrupted",
                "The image upload was interrupted.",
            )
        })?;
        if let Ok(bytes) = frame.into_data() {
            size += bytes.len() as u64;
            if size > MAX_BYTES {
                return Err(oversized());
            }
            file.write_all(&bytes).await.map_err(Error::from)?;
        }
    }
    file.flush().await.map_err(Error::from)?;
    drop(file);
    let asset = state.store.import_path(staged.path().to_owned()).await?;
    Ok((StatusCode::CREATED, Json(asset)))
}

async fn preview(
    State(state): State<Assets>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    image_response(state, id, false).await
}

async fn download(
    State(state): State<Assets>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    image_response(state, id, true).await
}

async fn image_response(state: Assets, id: String, attachment: bool) -> Result<Response, ApiError> {
    if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "image_missing",
            "This image is no longer available.",
        ));
    }
    let (asset, bytes) = state.store.read(&id).await?;
    let mut response = Response::new(Body::from(bytes));
    let headers = response.headers_mut();
    headers.insert(
        "content-type",
        HeaderValue::from_static(if asset.media_type == "image/png" {
            "image/png"
        } else {
            "image/jpeg"
        }),
    );
    let extension = if asset.media_type == "image/png" {
        "png"
    } else {
        "jpg"
    };
    let disposition = if attachment { "attachment" } else { "inline" };
    headers.insert(
        "content-disposition",
        HeaderValue::from_str(&format!("{disposition}; filename=\"{id}.{extension}\""))
            .expect("validated asset name"),
    );
    Ok(response)
}
