use super::run_api::ApiError;
use crate::{
    library::bundle_store::{BundleStore, Page, Record},
    tools::{
        Error,
        bundles::{self, Options},
        coordinator::Coordinator,
    },
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use std::sync::Arc;
use tokio::{io::AsyncReadExt, sync::Semaphore};
#[derive(Clone)]
struct BundleState {
    coordinator: Arc<Coordinator>,
    create: Arc<Semaphore>,
    downloads: Arc<Semaphore>,
}
pub fn router(coordinator: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/attempts/{id}/bundles", get(for_attempt).post(create))
        .route("/api/bundles", get(list))
        .route("/api/bundles/{id}", get(read))
        .route("/api/bundles/{id}/download", get(download))
        .with_state(BundleState {
            coordinator,
            create: Arc::new(Semaphore::new(1)),
            downloads: Arc::new(Semaphore::new(2)),
        })
}
#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    cursor: Option<i64>,
}
async fn list(
    State(state): State<BundleState>,
    Query(query): Query<Cursor>,
) -> Result<Json<Page>, ApiError> {
    Ok(Json(
        BundleStore(state.coordinator.assets.clone())
            .list(query.cursor, None)
            .await?,
    ))
}
async fn for_attempt(
    State(state): State<BundleState>,
    Path(id): Path<String>,
    Query(query): Query<Cursor>,
) -> Result<Json<Page>, ApiError> {
    Ok(Json(
        BundleStore(state.coordinator.assets.clone())
            .list(query.cursor, Some(id))
            .await?,
    ))
}
async fn read(
    State(state): State<BundleState>,
    Path(id): Path<String>,
) -> Result<Json<Record>, ApiError> {
    Ok(Json(
        BundleStore(state.coordinator.assets.clone())
            .get(&id)
            .await?,
    ))
}
async fn create(
    State(state): State<BundleState>,
    Path(id): Path<String>,
    Json(options): Json<Options>,
) -> Result<(StatusCode, Json<Record>), ApiError> {
    let permit = state
        .create
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Conflict("Another invocation bundle is being assembled"))?;
    // Retain capacity and the commit through HTTP cancellation. No tool controls or actions run.
    let record = tokio::spawn(async move {
        let _permit = permit;
        let saved = state.coordinator.attempts.get(&id).await?;
        let bundle = bundles::build(&state.coordinator.assets, &saved, &options).await?;
        BundleStore(state.coordinator.assets.clone())
            .save(&id, bundle)
            .await
            .map_err(Error::from)
    })
    .await
    .map_err(|_| Error::Invalid("Invocation bundle worker stopped".into()))??;
    Ok((StatusCode::CREATED, Json(record)))
}
async fn download(
    State(state): State<BundleState>,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let permit = state
        .downloads
        .clone()
        .try_acquire_owned()
        .map_err(|_| Error::Conflict("Two invocation bundle downloads are active"))?;
    let (file, size, permit) = tokio::spawn(async move {
        let (file, size) = BundleStore(state.coordinator.assets.clone())
            .open(&id)
            .await?;
        Ok::<_, Error>((file, size, permit))
    })
    .await
    .map_err(|_| Error::Invalid("Invocation bundle verification worker stopped".into()))??;
    let reader = tokio::fs::File::from_std(file);
    let stream =
        futures_util::stream::try_unfold((reader, permit), |(mut reader, permit)| async move {
            let mut bytes = vec![0; 65536];
            let count = reader.read(&mut bytes).await?;
            if count == 0 {
                return Ok::<_, std::io::Error>(None);
            }
            bytes.truncate(count);
            Ok(Some((bytes, (reader, permit))))
        });
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/x-tar".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=wafflery-invocation.tar".to_owned(),
            ),
            (header::CONTENT_LENGTH, size.to_string()),
            (header::CACHE_CONTROL, "no-store".to_owned()),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}
