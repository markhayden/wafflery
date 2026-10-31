use crate::events::Events;
use axum::{
    Router, body::Body, extract::State, http::header, response::IntoResponse, routing::get,
};
pub fn router(events: Events) -> Router {
    Router::new()
        .route("/api/events", get(stream))
        .with_state(events)
}
async fn stream(State(events): State<Events>) -> impl IntoResponse {
    let stream = futures_util::stream::unfold(events.subscribe(), |mut subscription| async move {
        let event = subscription.next().await?;
        let mut bytes = serde_json::to_vec(&event).ok()?;
        bytes.push(b'\n');
        Some((Ok::<_, std::convert::Infallible>(bytes), subscription))
    });
    (
        [
            (header::CONTENT_TYPE, "application/x-ndjson"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        Body::from_stream(stream),
    )
}
