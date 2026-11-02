use super::*;
use crate::tools::render::process::tests::setup;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn observed(
    monitor: &DocumentMonitor,
    predicate: impl Fn(&DocumentState) -> bool,
) -> DocumentState {
    tokio::time::timeout(
        Duration::from_secs(6),
        monitor.subscribe().wait_for(predicate),
    )
    .await
    .unwrap()
    .unwrap()
    .clone()
}
fn start(mode: &str) -> (tempfile::TempDir, RenderProcess, IdleLease, DocumentMonitor) {
    let (root, plan) = setup(mode);
    let grant = DocumentGrant::new(plan.run_dir()).unwrap();
    let process = RenderProcess::spawn(&plan).unwrap();
    let idle = IdleLease::document();
    let monitor = DocumentMonitor::start(
        process.clone(),
        grant,
        "http://127.0.0.1:4811",
        idle.clone(),
    );
    (root, process, idle, monitor)
}
async fn ready(monitor: &DocumentMonitor) -> url::Url {
    let state = observed(monitor, |s| matches!(s.readiness, Readiness::Ready(_))).await;
    let Readiness::Ready(url) = state.readiness else {
        panic!("expected document")
    };
    url
}
#[tokio::test]
async fn finished_renderer_keeps_its_document_live_until_close() {
    let (root, process, _, monitor) = start("document");
    let url = ready(&monitor).await;
    assert_eq!(process.snapshot().phase, ProcessPhase::Exited);
    assert!(!monitor.snapshot().closed);
    assert!(!monitor.snapshot().closing);
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
        .await
        .unwrap();
    stream
        .write_all(
            format!(
                "GET {} HTTP/1.1\r\nHost: {}\r\n\r\n",
                url.path(),
                url.authority()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.contains("Saved document"));
    assert_eq!(
        std::fs::read_to_string(root.path().join("saved run/untouched")).unwrap(),
        "saved evidence"
    );
    assert!(
        std::fs::read_dir(root.path())
            .unwrap()
            .flatten()
            .any(|e| e.file_name().to_string_lossy().starts_with("document-"))
    );
    assert!(monitor.close(false).await.unwrap());
    assert!(monitor.snapshot().closed);
    assert!(!matches!(monitor.snapshot().readiness, Readiness::Ready(_)));
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn a_content_policy_failure_closes_the_listener_without_changing_saved_evidence() {
    let (root, _, _, monitor) = start("document");
    let url = ready(&monitor).await;
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
        .await
        .unwrap();
    let body = serde_json::json!({"csp-report":{"document-uri":url.as_str(),"effective-directive":"img-src"}}).to_string();
    stream.write_all(format!("POST {}/violations HTTP/1.1\r\nHost: {}\r\nContent-Type: application/csp-report\r\nContent-Length: {}\r\n\r\n{body}",url.path(),url.authority(),body.len()).as_bytes()).await.unwrap();
    let state = observed(&monitor, |s| s.closed).await;
    assert_eq!(
        state.readiness,
        Readiness::Failed("Report requires content blocked by its isolation policy")
    );
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("saved run/untouched")).unwrap(),
        "saved evidence"
    );
}
#[tokio::test]
async fn invalid_document_outputs_fail_without_publishing_content() {
    for mode in [
        "early-error",
        "document-missing",
        "document-inside",
        "document-extra",
    ] {
        let (_root, process, _, monitor) = start(mode);
        let state = observed(&monitor, |s| s.closed).await;
        assert!(matches!(state.readiness, Readiness::Failed(_)), "{mode}");
        assert_eq!(process.snapshot().phase, ProcessPhase::Exited);
    }
}
#[tokio::test]
async fn idle_expiry_outlives_the_renderer_and_closes_the_content_server() {
    let (_root, process, idle, monitor) = start("document");
    let url = ready(&monitor).await;
    assert_eq!(process.snapshot().phase, ProcessPhase::Exited);
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(301)).await;
    tokio::time::resume();
    observed(&monitor, |s| s.closed).await;
    assert!(idle.expired());
    assert!(!idle.touch());
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn generation_has_a_deadline_and_never_auto_forces_a_stuck_process() {
    assert_eq!(GENERATION_TIMEOUT, Duration::from_secs(30));
    let (root, plan) = setup("hold");
    let grant = DocumentGrant::new(plan.run_dir()).unwrap();
    let process = RenderProcess::spawn(&plan).unwrap();
    let monitor = DocumentMonitor::start_with_timeout(
        process.clone(),
        grant,
        "http://127.0.0.1:4811",
        IdleLease::document(),
        Duration::from_millis(50),
    );
    let state = observed(&monitor, |s| s.closed).await;
    assert_eq!(
        state.readiness,
        Readiness::Failed("Document generation exceeded its deadline")
    );
    assert_eq!(process.snapshot().exit_code, Some(0));
    drop(root);
    let (_root, process, _, monitor) = start("stuck");
    assert!(monitor.close(true).await.is_err());
    assert!(
        !monitor.snapshot().closing,
        "a refused force request must not start closing"
    );
    assert!(!monitor.close(false).await.unwrap());
    assert_eq!(process.snapshot().phase, ProcessPhase::NeedsForce);
    assert!(!monitor.snapshot().closed);
    assert!(monitor.close(true).await.unwrap());
    assert!(monitor.snapshot().closed);
}
#[tokio::test]
async fn close_during_generation_and_last_owner_drop_cannot_publish_a_late_view() {
    let (root, process, _, monitor) = start("document-gated");
    assert!(monitor.close(false).await.unwrap());
    std::fs::write(root.path().join("release-document"), "release").unwrap();
    assert!(!matches!(monitor.snapshot().readiness, Readiness::Ready(_)));
    assert_eq!(process.snapshot().phase, ProcessPhase::Exited);
    let (_root, _, _, monitor) = start("document");
    let url = ready(&monitor).await;
    let mut state = monitor.subscribe();
    drop(monitor);
    tokio::time::timeout(Duration::from_secs(5), state.wait_for(|s| s.closed))
        .await
        .unwrap()
        .unwrap();
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .is_err()
    );
}
