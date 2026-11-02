use futures_util::FutureExt;
use serde_json::Value;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wafflery::{library::attempts::Attempt, tools::coordinator::Coordinator};

pub fn binary(name: &str) -> PathBuf {
    let path = PathBuf::from(std::env::var_os(name).expect("Select the explicit real-peer binary"));
    assert!(path.is_absolute() && path.is_file(), "Invalid {name}");
    for name in ["XDG_CONFIG_HOME", "XDG_CACHE_HOME", "XDG_DATA_HOME"] {
        let path = PathBuf::from(
            std::env::var_os(name).expect("Real-peer probes need isolated XDG directories"),
        );
        assert!(path.is_absolute() && path.is_dir(), "Invalid {name}");
    }
    path
}
pub async fn settled(c: &Arc<Coordinator>, id: &str) -> Attempt {
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let attempt = c.attempts.get(id).await.unwrap();
            let snapshot = c.snapshot().await.unwrap();
            if attempt.preservation["contract"] != "pending"
                && snapshot["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|tool| tool["busy"] == false)
            {
                return attempt;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("real peer did not settle")
}
// A failed probe must still observe tool/report shutdown before its runtime disappears.
pub async fn run<T>(c: &Arc<Coordinator>, probe: impl std::future::Future<Output = T>) -> T {
    let result = std::panic::AssertUnwindSafe(probe).catch_unwind().await;
    let cleanup = std::panic::AssertUnwindSafe(finish(c)).catch_unwind().await;
    match result {
        Err(error) => std::panic::resume_unwind(error),
        Ok(value) => {
            if let Err(error) = cleanup {
                std::panic::resume_unwind(error);
            }
            value
        }
    }
}
async fn finish(c: &Arc<Coordinator>) {
    let (quit, mut stopped) = tokio::sync::watch::channel(false);
    c.begin_shutdown(true, quit).await.unwrap();
    tokio::time::timeout(Duration::from_secs(10), stopped.wait_for(|value| *value))
        .await
        .unwrap()
        .unwrap();
    c.assets.database.shutdown().await.unwrap();
}
pub async fn reports(c: &Arc<Coordinator>, attempt: &Attempt) {
    let origin = "http://127.0.0.1:4811";
    let actions = attempt.declaration["document"]["actions"]
        .as_array()
        .unwrap();
    let action = actions
        .iter()
        .find(|action| action["name"] == attempt.request.action)
        .unwrap();
    let renders = action["renders"].as_array().unwrap();
    assert_eq!(
        renders
            .iter()
            .map(|render| render["kind"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["document", "server"])
    );
    let before = serde_json::to_value(attempt).unwrap();
    for render in renders {
        let opened = c
            .open_render(&attempt.id, render["name"].as_str().unwrap(), origin)
            .await
            .unwrap();
        let id = opened["id"].as_str().unwrap();
        let ready = phase(c, id, "ready").await;
        assert!(ready["error"].is_null(), "{ready}");
        let url = c.frame_target(id, origin).await.unwrap();
        assert_ne!(url.origin().ascii_serialization(), origin);
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .unwrap();
        stream
            .write_all(
                format!(
                    "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                    url.path(),
                    url.authority()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let mut bytes = Vec::new();
        tokio::time::timeout(
            Duration::from_secs(10),
            stream.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(bytes.len() < 32 * 1024 * 1024);
        let response = String::from_utf8(bytes).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.to_ascii_lowercase().contains("text/html"));
        assert!(response.contains(&format!("frame-ancestors {origin}")));
        assert!(response.contains("<html") || response.contains("<!doctype html"));
        c.close_render(id, false, false).await.unwrap();
        phase(c, id, "exited").await;
        assert!(c.frame_target(id, origin).await.is_err());
        assert_eq!(
            serde_json::to_value(c.attempts.get(&attempt.id).await.unwrap()).unwrap(),
            before
        );
    }
}
async fn phase(c: &Arc<Coordinator>, id: &str, expected: &str) -> Value {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let view = c.render_state(id).await.unwrap();
            if view["phase"] == expected {
                return view;
            }
            assert!(
                view["error"].is_null(),
                "report failed before {expected}: {view}"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("report did not reach the expected state")
}
