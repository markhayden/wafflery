pub mod assets;
pub mod control;

pub async fn serve(root: &std::path::Path) -> std::io::Result<()> {
    use axum::{
        Router,
        body::Body,
        http::{StatusCode, Uri},
        response::Response,
    };
    let owner = control::Ownership::acquire(root)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let app = Router::new().fallback(axum::routing::get(|uri: Uri| async move {
        match assets::get(uri.path()) {
            Some(bytes) => {
                let content_type = if uri.path().ends_with(".js") {
                    "text/javascript"
                } else if uri.path().ends_with(".css") {
                    "text/css"
                } else {
                    "text/html; charset=utf-8"
                };
                Response::builder()
                    .header("Content-Type", content_type)
                    .header("X-Content-Type-Options", "nosniff")
                    .body(Body::from(bytes))
                    .expect("static response")
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .expect("static response"),
        }
    }));
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("signal handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => (), _ = terminate.recv() => () }
    });
    tokio::select! {
        result = server => result,
        result = control::listen(&owner, &origin) => result,
    }
}
