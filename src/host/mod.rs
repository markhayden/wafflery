pub mod api;
mod asset_api;
pub mod assets;
pub mod auth;
mod cleanup_api;
pub mod control;
mod event_api;
mod example_api;
mod export_api;
pub mod launch;
mod run_api;

pub async fn serve(root: &std::path::Path) -> std::io::Result<()> {
    let owner = control::Ownership::acquire(root)?;
    let store = crate::library::assets::AssetStore::open(&owner.root.join("Library"))
        .map_err(std::io::Error::other)?;
    let descriptors = crate::tools::registration::load(&owner.root.join("tools.json"))?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    println!("{origin}");
    let (shutdown, mut stopping) = tokio::sync::watch::channel(false);
    let auth = auth::Auth::new()?;
    let coordinator = std::sync::Arc::new(
        crate::tools::coordinator::Coordinator::open(store.clone(), descriptors)
            .await
            .map_err(std::io::Error::other)?,
    );
    let app = api::router_with_tools(
        &origin,
        auth.clone(),
        shutdown.clone(),
        store.clone(),
        Some(coordinator.clone()),
    );
    let event_shutdown = coordinator.events.clone();
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("signal handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => (), _ = terminate.recv() => (), _ = stopping.changed() => () }
        event_shutdown.close();
    });
    let result = tokio::select! {
        result = server => result,
        result = control::listen(&owner, &origin, shutdown, auth, coordinator.clone()) => result,
    };
    if !coordinator.close().await {
        return Err(std::io::Error::other(
            "A tool did not stop after EOF; no force kill was performed",
        ));
    }
    store
        .database
        .shutdown()
        .await
        .map_err(std::io::Error::other)?;
    result
}
