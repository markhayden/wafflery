//! Ephemeral bounded upload review; publication and controls require a separate explicit import.
use super::run_api::ApiError;
use crate::{
    library::{bundle_archive::MAX_TOTAL, bundle_store::Record},
    tools::{
        Error,
        bundle_import::{ImportJob, Progress},
        bundle_reader::{self, ValidatedBundle},
        coordinator::Coordinator,
    },
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use futures_util::StreamExt;
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
#[derive(Clone)]
struct Reviews {
    coordinator: Arc<Coordinator>,
    slots: Arc<Semaphore>,
    items: Arc<Mutex<BTreeMap<String, Arc<Review>>>>,
}
struct Review {
    bundle: Arc<ValidatedBundle>,
    record: super::bundle_api::ReviewedRecord,
    state: Mutex<Lifecycle>,
    _slot: OwnedSemaphorePermit,
}
#[derive(Default)]
struct Lifecycle {
    job: Option<Arc<ImportJob>>,
    closed: bool,
}
#[derive(Serialize)]
struct Response {
    id: String,
    record: super::bundle_api::ReviewedRecord,
    import: Option<Progress>,
}
fn response(id: String, review: &Review, import: Option<Progress>) -> Response {
    Response {
        id,
        import,
        record: review.record.clone(),
    }
}
fn record(b: &ValidatedBundle) -> super::bundle_api::ReviewedRecord {
    Record {
        id: b.sha256().into(),
        sha256: b.sha256().into(),
        size: b.archive().len() as u64,
        manifest: b.manifest().clone(),
        members: b
            .members()
            .iter()
            .map(|(name, m)| crate::library::bundle_archive::Member {
                name: name.clone(),
                sha256: m.sha256().into(),
                bytes: m.bytes(),
            })
            .collect(),
        workspace_dependent: b.manifest()["steps"].as_array().unwrap().iter().any(|s| {
            !s["reference_only"].as_array().unwrap().is_empty()
                || !s["unportable"].as_array().unwrap().is_empty()
        }),
    }
    .into()
}
pub fn router(coordinator: Arc<Coordinator>) -> Router {
    Router::new()
        .route("/api/bundle-reviews", get(list).post(upload))
        .route("/api/bundle-reviews/{id}", get(read).delete(remove))
        .route("/api/bundle-reviews/{id}/progress", get(progress))
        .route("/api/bundle-reviews/{id}/import", post(import))
        .route("/api/bundle-reviews/{id}/replay", post(start_replay))
        .with_state(Reviews {
            coordinator,
            slots: Arc::new(Semaphore::new(2)),
            items: Arc::new(Mutex::new(BTreeMap::new())),
        })
}
async fn lookup(state: &Reviews, id: &str) -> Result<Arc<Review>, Error> {
    state
        .items
        .lock()
        .await
        .get(id)
        .cloned()
        .ok_or(Error::Missing(
            "Bundle review is unavailable. Upload it again.",
        ))
}
async fn list(State(state): State<Reviews>) -> Result<Json<serde_json::Value>, ApiError> {
    let items = state.items.lock().await.clone();
    let mut summaries = vec![];
    for (id, review) in items {
        let current = review.state.lock().await;
        summaries.push(serde_json::json!({"id":id,"bundle":review.bundle.sha256(),"phase":current.job.as_ref().map(|j|j.snapshot().phase).unwrap_or("review".into())}));
    }
    Ok(Json(serde_json::json!({"items":summaries})))
}
async fn read(
    State(state): State<Reviews>,
    Path(id): Path<String>,
) -> Result<Json<Response>, ApiError> {
    let review = lookup(&state, &id).await?;
    let current = review.state.lock().await;
    Ok(Json(response(
        id,
        &review,
        current.job.as_ref().map(|j| j.snapshot()),
    )))
}
async fn progress(
    State(state): State<Reviews>,
    Path(id): Path<String>,
) -> Result<Json<Option<Progress>>, ApiError> {
    let review = lookup(&state, &id).await?;
    let current = review.state.lock().await;
    Ok(Json(current.job.as_ref().map(|j| j.snapshot())))
}
async fn remove(
    State(state): State<Reviews>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let review = lookup(&state, &id).await?;
    let mut current = review.state.lock().await;
    if current
        .job
        .as_ref()
        .is_some_and(|j| j.snapshot().phase == "importing")
    {
        return Err(Error::Conflict(
            "Wait for the owned import to finish before dismissing this review",
        )
        .into());
    }
    current.closed = true;
    state.items.lock().await.remove(&id);
    Ok(StatusCode::NO_CONTENT)
}
async fn upload(
    State(state): State<Reviews>,
    headers: HeaderMap,
    body: Body,
) -> Result<(StatusCode, Json<Response>), ApiError> {
    if headers.get("content-type").and_then(|v| v.to_str().ok()) != Some("application/x-tar") {
        return Err(Error::Invalid("Upload an uncompressed invocation tar archive".into()).into());
    }
    let length = headers
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());
    if length.is_some_and(|n| n > MAX_TOTAL) {
        return Err(Error::Invalid("Invocation bundle exceeds 256 MiB".into()).into());
    }
    let slot = state.slots.clone().try_acquire_owned().map_err(|_| {
        Error::Conflict("Two bundle reviews are open. Dismiss one before uploading another.")
    })?;
    let bytes = tokio::time::timeout(Duration::from_secs(30), async {
        let mut stream = body.into_data_stream();
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length.unwrap_or(65536) as usize)
            .map_err(|_| Error::Invalid("Not enough memory to review this bundle".into()))?;
        while let Some(chunk) = stream.next().await {
            let chunk =
                chunk.map_err(|_| Error::Invalid("Bundle upload was interrupted".into()))?;
            if chunk.len() > MAX_TOTAL as usize - bytes.len() {
                return Err(Error::Invalid("Invocation bundle exceeds 256 MiB".into()));
            }
            if bytes.len() + chunk.len() > bytes.capacity() {
                let capacity = bytes
                    .capacity()
                    .max(65536)
                    .saturating_mul(2)
                    .max(bytes.len() + chunk.len())
                    .min(MAX_TOTAL as usize);
                bytes
                    .try_reserve_exact(capacity - bytes.len())
                    .map_err(|_| {
                        Error::Invalid("Not enough memory to review this bundle".into())
                    })?;
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok::<_, Error>(bytes)
    })
    .await
    .map_err(|_| {
        Error::Conflict("Bundle upload did not finish within 30 seconds. Retry explicitly.")
    })??;
    // Once parsing starts, retain the permit through completion even if the browser disconnects.
    let worker = tokio::spawn(async move {
        let (bundle, record) = tokio::task::spawn_blocking(move || {
            let bundle = bundle_reader::read(bytes)?;
            let record = record(&bundle);
            Ok::<_, Error>((bundle, record))
        })
        .await
        .map_err(|_| Error::Invalid("Bundle validation worker stopped".into()))??;
        let mut random = [0; 16];
        getrandom::fill(&mut random).map_err(std::io::Error::other)?;
        let id = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let review = Arc::new(Review {
            bundle: Arc::new(bundle),
            record,
            state: Mutex::new(Lifecycle::default()),
            _slot: slot,
        });
        let result = response(id.clone(), &review, None);
        state.items.lock().await.insert(id, review);
        Ok::<_, Error>(result)
    });
    Ok((
        StatusCode::CREATED,
        Json(
            worker
                .await
                .map_err(|_| Error::Invalid("Bundle review worker stopped".into()))??,
        ),
    ))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    step: usize,
    tool: String,
}
async fn import(
    State(state): State<Reviews>,
    Path(id): Path<String>,
    Json(request): Json<Import>,
) -> Result<(StatusCode, Json<Progress>), ApiError> {
    let review = lookup(&state, &id).await?;
    let mut current = review.state.lock().await;
    if current.closed {
        return Err(Error::Missing("Bundle review was dismissed").into());
    }
    if current
        .job
        .as_ref()
        .is_some_and(|j| j.snapshot().phase == "importing")
    {
        return Err(Error::Conflict("An import from this review is already running").into());
    }
    let job = state
        .coordinator
        .start_bundle_import(review.bundle.clone(), request.step, request.tool)
        .await?;
    let status = job.snapshot();
    current.job = Some(job);
    Ok((StatusCode::ACCEPTED, Json(status)))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ChainTools {
    tools: Vec<String>,
}
async fn start_replay(
    State(state): State<Reviews>,
    Path(id): Path<String>,
    Json(request): Json<ChainTools>,
) -> Result<(StatusCode, Json<crate::tools::chain_replay::ReplayStatus>), ApiError> {
    let review = lookup(&state, &id).await?;
    let current = review.state.lock().await;
    if current.closed {
        return Err(Error::Missing("Bundle review was dismissed").into());
    }
    if current
        .job
        .as_ref()
        .is_some_and(|job| job.snapshot().phase == "importing")
    {
        return Err(Error::Conflict(
            "Wait for this review's current import before starting a chain",
        )
        .into());
    }
    let status = state
        .coordinator
        .start_chain_replay(review.bundle.clone(), request.tools)
        .await?;
    Ok((StatusCode::CREATED, Json(status)))
}
