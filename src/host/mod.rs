pub mod api;
mod asset_api;
pub mod assets;
pub mod auth;
mod bundle_api;
mod bundle_review_api;
mod chain_replay_api;
mod cleanup_api;
pub mod control;
mod event_api;
mod example_api;
mod export_api;
mod frame_api;
pub mod frame_grants;
pub mod launch;
mod render_api;
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
    let signal_coordinator = coordinator.clone();
    let signal_quit = shutdown.clone();
    let signals = tokio::spawn(async move {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("signal handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => (), _ = terminate.recv() => () }
        let _ = signal_coordinator.begin_shutdown(true, signal_quit).await;
    });
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        if !*stopping.borrow() {
            let _ = stopping.changed().await;
        }
        event_shutdown.close();
    });
    let result = tokio::select! {
        result = server => result,
        result = control::listen(&owner, &origin, shutdown, auth, coordinator.clone()) => result,
    };
    signals.abort();
    // Keep ownership on an unexpected server failure; never orphan a still-running tool.
    while !coordinator.close().await {
        eprintln!("Waiting for an owned tool to exit; no automatic force kill was performed.");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    store
        .database
        .shutdown()
        .await
        .map_err(std::io::Error::other)?;
    result
}
