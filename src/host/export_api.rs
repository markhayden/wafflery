use super::run_api::ApiError;
use crate::{
    library::export,
    tools::{Error, coordinator::Coordinator},
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use std::sync::Arc;
use tokio::{io::AsyncReadExt, sync::Semaphore};
#[derive(Clone)]
struct ExportState {
    coordinator: Arc<Coordinator>,
    capacity: Arc<Semaphore>,
}
pub fn router(coordinator: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/attempts/{id}/export", post(download))
        .with_state(ExportState {
            coordinator,
            capacity: Arc::new(Semaphore::new(1)),
        })
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportRequest {
    include_images: bool,
}
async fn download(
    State(state): State<ExportState>,
    Path(id): Path<String>,
    Json(request): Json<ExportRequest>,
) -> Result<Response, ApiError> {
    let permit = state
        .capacity
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Conflict("Another evidence download is in progress"))?;
    let attempt = state.coordinator.attempts.get(&id).await?;
    let archive =
        export::archive(&state.coordinator.assets, attempt, request.include_images).await?;
    let reader = tokio::fs::File::from_std(archive.file.reopen().map_err(Error::Io)?);
    let stream = futures_util::stream::try_unfold(
        (reader, archive.file, permit),
        |(mut reader, file, permit)| async move {
            let mut bytes = vec![0; 64 * 1024];
            let count = reader.read(&mut bytes).await?;
            if count == 0 {
                return Ok::<_, std::io::Error>(None);
            }
            bytes.truncate(count);
            Ok(Some((bytes, (reader, file, permit))))
        },
    );
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/gzip".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=wafflehouse-evidence.tar.gz".to_owned(),
            ),
            (header::CONTENT_LENGTH, archive.size.to_string()),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}
