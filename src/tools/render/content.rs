//! Isolated, immutable document origin with owned HTTP connections.
use super::{checked_origin, document::Document};
use crate::tools::{Error, Result};
use axum::{
    body::{Body, Bytes, to_bytes},
    http::{Request, Response, StatusCode},
};
use hyper::{body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::watch, task::JoinSet};
use url::Url;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContentState {
    pub closed: bool,
    pub error: Option<&'static str>,
}
#[derive(Clone)]
pub struct DocumentServer {
    url: Url,
    stop: watch::Sender<bool>,
    state: watch::Receiver<ContentState>,
}
impl DocumentServer {
    pub async fn start(document: Document, host_origin: &str) -> Result<Self> {
        let host_origin = checked_origin(host_origin)?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let mut token = [0_u8; 32];
        getrandom::fill(&mut token).map_err(std::io::Error::other)?;
        let token: String = token.iter().map(|byte| format!("{byte:02x}")).collect();
        let url = Url::parse(&format!("http://{address}/{token}")).expect("literal loopback URL");
        if url.origin().ascii_serialization() == host_origin {
            return Err(Error::Invalid("Document must use a separate origin".into()));
        }
        let (state_tx, state) = watch::channel(ContentState::default());
        let (stop, stop_rx) = watch::channel(false);
        let data = Arc::new(Content {
            authority: address.to_string(),
            policy: format!(
                "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: blob:; font-src data:; media-src data: blob:; connect-src 'none'; base-uri 'none'; form-action 'none'; sandbox allow-scripts allow-same-origin; frame-ancestors {host_origin}; report-uri {}/violations",
                url.path()
            ),
            media_type: media_type(document.name()),
            bytes: Bytes::from_owner(DocumentBytes(document)),
            url: url.clone(),
            state: state_tx.clone(),
        });
        tokio::spawn(serve(listener, data, stop_rx, state_tx));
        Ok(Self { url, stop, state })
    }
    pub fn snapshot(&self) -> ContentState {
        self.state.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<ContentState> {
        self.state.clone()
    }
    pub fn url(&self) -> Result<Url> {
        let state = self.state.borrow();
        if let Some(error) = state.error {
            return Err(Error::Invalid(error.into()));
        }
        if state.closed || *self.stop.borrow() {
            return Err(Error::Conflict("Document viewer is closed"));
        }
        Ok(self.url.clone())
    }
    pub async fn close(&self) -> Result<()> {
        self.stop.send_replace(true);
        self.state
            .clone()
            .wait_for(|s| s.closed)
            .await
            .map_err(|_| {
                Error::Invalid("Document server stopped without confirming closure".into())
            })?;
        Ok(())
    }
}
struct DocumentBytes(Document);
impl AsRef<[u8]> for DocumentBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.bytes()
    }
}
struct Content {
    url: Url,
    authority: String,
    policy: String,
    media_type: &'static str,
    bytes: Bytes,
    state: watch::Sender<ContentState>,
}
async fn serve(
    listener: TcpListener,
    data: Arc<Content>,
    mut stop: watch::Receiver<bool>,
    state: watch::Sender<ContentState>,
) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            _ = stop.changed() => break, // Also closes when the last owner disappears.
            _ = connections.join_next(), if !connections.is_empty() => {},
            accepted = listener.accept() => {
                let Ok((stream,_)) = accepted else {
                    state.send_modify(|s|s.error=Some("Document listener failed"));
                    break;
                };
                if connections.len() >= 8 { drop(stream); continue; }
                let data = data.clone();
                connections.spawn(async move {
                    let service=service_fn(move |request| {
                        let data=data.clone();
                        async move {Ok::<_,Infallible>(data.respond(request).await)}
                    });
                    let mut builder=http1::Builder::new();
                    builder.keep_alive(false).max_headers(32).max_buf_size(8192).max_header_size(8192);
                    // Bounds partial headers, bodies and clients that stop reading the response.
                    let _ = tokio::time::timeout(Duration::from_secs(10),
                        builder.serve_connection(TokioIo::new(stream),service)).await;
                });
            }
        }
    }
    drop(listener);
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    state.send_modify(|s| s.closed = true);
}
impl Content {
    async fn respond(&self, request: Request<Incoming>) -> Response<Body> {
        let headers = request.headers();
        if headers.get_all("host").iter().count() != 1
            || headers.get("host").and_then(|v| v.to_str().ok()) != Some(self.authority.as_str())
            || headers.contains_key("authorization")
        {
            return self.response(StatusCode::FORBIDDEN, Body::empty(), "text/plain");
        }
        if request.uri().scheme().is_some()
            || request.uri().authority().is_some()
            || request.uri().query().is_some()
        {
            return self.response(StatusCode::NOT_FOUND, Body::empty(), "text/plain");
        }
        if request.method() == "GET" && request.uri().path() == self.url.path() {
            if self.state.borrow().error.is_some() {
                return self.response(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Body::from("Report content is unavailable."),
                    "text/plain",
                );
            }
            return self.response(
                StatusCode::OK,
                Body::from(self.bytes.clone()),
                self.media_type,
            );
        }
        if request.method() == "POST"
            && request.uri().path() == format!("{}/violations", self.url.path())
        {
            return self.violation(request).await;
        }
        self.response(StatusCode::NOT_FOUND, Body::empty(), "text/plain")
    }
    async fn violation(&self, request: Request<Incoming>) -> Response<Body> {
        // A no-referrer report POST can carry Origin: null (WebKit). The private
        // document capability and matching document-uri scope this notification;
        // this endpoint grants no CORS access or authenticated workspace operation.
        // https://fetch.spec.whatwg.org/#append-a-request-origin-header
        if request.headers().get_all("origin").iter().count() > 1
            || request.headers().get("origin").is_some_and(|v| {
                v.as_bytes() != b"null"
                    && v.as_bytes() != self.url.origin().ascii_serialization().as_bytes()
            })
        {
            return self.response(StatusCode::FORBIDDEN, Body::empty(), "text/plain");
        }
        if request
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            != Some("application/csp-report")
        {
            return self.response(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Body::empty(),
                "text/plain",
            );
        }
        let Ok(bytes) = to_bytes(Body::new(request.into_body()), 16 * 1024).await else {
            return self.response(StatusCode::PAYLOAD_TOO_LARGE, Body::empty(), "text/plain");
        };
        let value = serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_default();
        let report = &value["csp-report"];
        if report["document-uri"].as_str() != Some(self.url.as_str())
            || !matches!(
                report["effective-directive"].as_str(),
                Some(
                    "default-src"
                        | "img-src"
                        | "script-src"
                        | "script-src-elem"
                        | "script-src-attr"
                        | "style-src"
                        | "style-src-elem"
                        | "style-src-attr"
                        | "font-src"
                        | "media-src"
                        | "connect-src"
                        | "frame-src"
                        | "object-src"
                        | "worker-src"
                        | "base-uri"
                        | "form-action"
                )
            )
        {
            return self.response(StatusCode::BAD_REQUEST, Body::empty(), "text/plain");
        }
        // Never store/log the blocked URI, source snippet or original-policy payload.
        self.state.send_modify(|s| {
            s.error = Some("Report requires content blocked by its isolation policy")
        });
        self.response(StatusCode::NO_CONTENT, Body::empty(), "text/plain")
    }
    fn response(&self, status: StatusCode, body: Body, media_type: &str) -> Response<Body> {
        Response::builder()
            .status(status)
            .header("Content-Type", media_type)
            .header("Content-Security-Policy", &self.policy)
            .header("Cache-Control", "no-store")
            .header("Referrer-Policy", "no-referrer")
            .header("X-Content-Type-Options", "nosniff")
            .header("Cross-Origin-Resource-Policy", "same-origin")
            .header("Connection", "close")
            .body(body)
            .expect("validated document response headers")
    }
}
fn media_type(name: &str) -> &'static str {
    match name
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "json" => "application/json",
        _ => "text/plain; charset=utf-8",
    }
}
#[cfg(test)]
#[path = "content_tests.rs"]
mod tests;
