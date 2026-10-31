pub mod api;
mod asset_api;
pub mod assets;
pub mod auth;
pub mod control;
pub mod launch;

pub async fn serve(root: &std::path::Path) -> std::io::Result<()> {
    let owner = control::Ownership::acquire(root)?;
    let store = crate::library::assets::AssetStore::open(&owner.root.join("Library"))
        .map_err(std::io::Error::other)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    println!("{origin}");
    let (shutdown, mut stopping) = tokio::sync::watch::channel(false);
    let auth = auth::Auth::new()?;
    let app = api::router(&origin, auth.clone(), shutdown.clone(), store.clone());
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("signal handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => (), _ = terminate.recv() => (), _ = stopping.changed() => () }
    });
    let result = tokio::select! {
        result = server => result,
        result = control::listen(&owner, &origin, shutdown, auth) => result,
    };
    store
        .database
        .shutdown()
        .await
        .map_err(std::io::Error::other)?;
    result
}
