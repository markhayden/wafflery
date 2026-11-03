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
    routing::{get, post},
};
use std::sync::Arc;
use tokio::{io::AsyncReadExt, sync::Semaphore};
#[derive(Clone)]
struct ExportState {
    coordinator: Arc<Coordinator>,
    capacity: Arc<Semaphore>,
    handoff_capacity: Arc<Semaphore>,
}
pub fn router(coordinator: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/attempts/{id}/export", post(download))
        .route("/api/attempts/{id}/handoff", get(handoff))
        .route("/api/attempts/{id}/handoff/arguments", post(arguments))
        .with_state(ExportState {
            coordinator,
            capacity: Arc::new(Semaphore::new(1)),
            handoff_capacity: Arc::new(Semaphore::new(2)),
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
                "attachment; filename=wafflery-evidence.tar.gz".to_owned(),
            ),
            (header::CONTENT_LENGTH, archive.size.to_string()),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}

async fn handoff(
    State(state): State<ExportState>,
    Path(id): Path<String>,
) -> Result<Json<crate::tools::handoff::Handoff>, ApiError> {
    Ok(Json(saved_handoff(state, id).await?))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}
async fn arguments(
    State(state): State<ExportState>,
    Path(id): Path<String>,
    Json(_): Json<Empty>,
) -> Result<Response, ApiError> {
    let handoff = saved_handoff(state, id).await?;
    if handoff.state != "local" {
        return Err(Error::Invalid(handoff.blockers.join(" ")).into());
    }
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=wafflery-arguments.json",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        serde_json::to_vec_pretty(&handoff.arguments).map_err(Error::Json)?,
    )
        .into_response())
}

async fn saved_handoff(
    state: ExportState,
    id: String,
) -> Result<crate::tools::handoff::Handoff, Error> {
    let permit = state
        .handoff_capacity
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            Error::Conflict("Two CLI handoff checks are active. Wait before trying again.")
        })?;
    // A dropped HTTP waiter cannot release the capacity while its blocking byte verification runs.
    tokio::spawn(async move {
        let _permit = permit;
        let attempt = state.coordinator.attempts.get(&id).await?;
        crate::tools::handoff::build(&state.coordinator.assets, &attempt).await
    })
    .await
    .map_err(|_| Error::Invalid("CLI handoff worker stopped".into()))?
}
