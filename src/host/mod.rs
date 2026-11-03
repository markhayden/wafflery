use std::io::Write;

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
pub(crate) mod terminal;

pub async fn serve(root: &std::path::Path) -> std::io::Result<()> {
    serve_mode(root, None).await
}

pub async fn start(root: &std::path::Path, no_open: bool) -> std::io::Result<()> {
    terminal::enable();
    serve_mode(root, Some(no_open)).await
}

async fn serve_mode(root: &std::path::Path, attached: Option<bool>) -> std::io::Result<()> {
    // Register before initialization: an interrupt during a tool probe must still close it.
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    let mut hangup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;
    let owner = control::Ownership::acquire(root)?;
    let store = crate::library::assets::AssetStore::open(&owner.root.join("Library"))
        .map_err(std::io::Error::other)?;
    let descriptors = crate::tools::registration::load(&owner.root.join("tools.json"))?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let (shutdown, mut stopping) = tokio::sync::watch::channel(false);
    let auth = auth::Auth::new()?;
    let control_listener = tokio::net::UnixListener::bind(&owner.socket)?;
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
        tokio::select! { _ = interrupt.recv() => (), _ = terminate.recv() => (), _ = hangup.recv() => () }
        terminal::event(format_args!(
            "Shutdown requested; stopping owned work and tools"
        ));
        let _ = signal_coordinator.begin_shutdown(true, signal_quit).await;
    });
    // Bind and publish before announcing readiness or handing the browser its bootstrap.
    let published = owner.publish();
    if published.is_ok() {
        terminal::event(format_args!("Listening at {origin}"));
    }
    let browser = attached.filter(|_| published.is_ok()).map(|no_open| {
        let auth = auth.clone();
        let origin = origin.clone();
        let root = owner.root.clone();
        tokio::spawn(async move {
            let result = tokio::task::spawn_blocking(move || -> std::io::Result<()> {
                let url = format!("{origin}/#bootstrap={}", auth.issue()?);
                let _ = writeln!(std::io::stdout().lock(), "{url}");
                terminal::event(format_args!(
                    "Browser link is single-use (60 seconds). Reopen later: {} --data-dir {}",
                    launch::shell_quote(&std::env::current_exe()?),
                    launch::shell_quote(&root)
                ));
                if !no_open {
                    launch::open_browser(&url)?;
                }
                Ok(())
            })
            .await;
            if let Err(error) = result.unwrap_or_else(|e| Err(std::io::Error::other(e))) {
                terminal::event(format_args!("Browser handoff failed: {error}"));
            }
        })
    });
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        if !*stopping.borrow() {
            let _ = stopping.changed().await;
        }
        event_shutdown.close();
    });
    let result = match published {
        Ok(()) => tokio::select! {
            result = server => result,
            result = control::listen(control_listener, &owner, &origin, shutdown, auth, coordinator.clone()) => result,
        },
        Err(error) => Err(error),
    };
    signals.abort();
    if let Some(browser) = browser {
        let _ = browser.await;
    }
    terminal::event(format_args!("Stopping host; waiting for owned tools"));
    // Keep ownership on an unexpected server failure; never orphan a still-running tool.
    while !coordinator.close().await {
        let _ = writeln!(
            std::io::stderr().lock(),
            "Waiting for an owned tool to exit; no automatic force kill was performed."
        );
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    store.shutdown().await.map_err(std::io::Error::other)?;
    terminal::event(format_args!("Shutdown complete"));
    result
}
