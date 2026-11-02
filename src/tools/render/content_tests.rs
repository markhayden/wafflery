use super::*;
use crate::tools::render::{
    document::DocumentGrant,
    process::{Output, ProcessPhase, ProcessState},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn document() -> (tempfile::TempDir, Document) {
    let root = tempfile::tempdir().unwrap();
    let run = root.path().join("run");
    std::fs::create_dir(&run).unwrap();
    let output = root.path().join("report.html");
    std::fs::write(&output, "<!doctype html><h1>Saved report</h1>").unwrap();
    std::fs::write(root.path().join("neighbor.txt"), "never serve this").unwrap();
    let state = ProcessState {
        phase: ProcessPhase::Exited,
        close_requested: false,
        exit_code: Some(0),
        error: None,
        stdout: Output {
            bytes: format!("{}\n", output.display()).into_bytes(),
            finished: true,
            ..Default::default()
        },
        stderr: Output {
            finished: true,
            ..Default::default()
        },
    };
    let doc = DocumentGrant::new(&run)
        .unwrap()
        .read(&state)
        .await
        .unwrap();
    (root, doc)
}
async fn request(url: &Url, method: &str, path: &str, extra: &str, body: &str) -> String {
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
        .await
        .unwrap();
    stream.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n{extra}\r\n{body}",url.authority(),body.len()).as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(Duration::from_secs(3), stream.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    String::from_utf8(bytes).unwrap()
}
#[tokio::test]
async fn only_the_immutable_document_is_served_with_an_exact_frame_origin() {
    let (root, doc) = document().await;
    let server = DocumentServer::start(doc, "http://127.0.0.1:4811")
        .await
        .unwrap();
    let url = server.url().unwrap();
    assert_ne!(url.origin().ascii_serialization(), "http://127.0.0.1:4811");
    std::fs::write(root.path().join("report.html"), "mutated").unwrap();
    let response = request(&url, "GET", url.path(), "", "").await;
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.ends_with("<!doctype html><h1>Saved report</h1>"));
    assert!(response.contains("content-type: text/html; charset=utf-8"));
    assert!(response.contains("frame-ancestors http://127.0.0.1:4811;"));
    for policy in [
        "default-src 'none'",
        "connect-src 'none'",
        "base-uri 'none'",
        "form-action 'none'",
        "sandbox allow-scripts allow-same-origin",
        "referrer-policy: no-referrer",
        "x-content-type-options: nosniff",
        "cache-control: no-store",
    ] {
        assert!(response.contains(policy), "{policy}");
    }
    assert!(!response.contains("access-control-allow-origin"));
    for path in [
        "/",
        "/neighbor.txt",
        "/api/state",
        "/../neighbor.txt",
        &format!("{}?extra", url.path()),
    ] {
        let response = request(&url, "GET", path, "", "").await;
        assert!(response.starts_with("HTTP/1.1 404"));
        assert!(!response.contains("never serve"));
    }
    let response = request(
        &url,
        "GET",
        url.path(),
        "Authorization: Bearer synthetic\r\n",
        "",
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 403"));
    let mut wrong_host = url.clone();
    wrong_host.set_host(Some("localhost")).unwrap();
    assert!(
        request(&wrong_host, "GET", url.path(), "", "")
            .await
            .starts_with("HTTP/1.1 403")
    );
    assert!(
        request(&url, "GET", url.path(), "Host: example.invalid\r\n", "")
            .await
            .starts_with("HTTP/1.1 403")
    );
    let response = request(
        &url,
        "OPTIONS",
        url.path(),
        "Origin: https://example.invalid\r\n",
        "",
    )
    .await;
    assert!(!response.contains("access-control-allow-origin"));
    server.close().await.unwrap();
    assert!(server.snapshot().closed);
    assert!(server.url().is_err());
    assert!(
        tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .is_err()
    );
}
#[tokio::test]
async fn csp_reports_mark_nonconforming_content_without_retaining_report_details() {
    let (_root, doc) = document().await;
    let server = DocumentServer::start(doc, "http://127.0.0.1:4811")
        .await
        .unwrap();
    let url = server.url().unwrap();
    let path = format!("{}/violations", url.path());
    let headers = "Content-Type: application/csp-report\r\n";
    let bad = request(&url, "POST", &path, headers, "{}").await;
    assert!(bad.starts_with("HTTP/1.1 400"));
    assert_eq!(server.snapshot().error, None);
    let oversized = request(&url, "POST", &path, headers, &"x".repeat(16 * 1024 + 1)).await;
    assert!(oversized.starts_with("HTTP/1.1 413"));
    assert_eq!(server.snapshot().error, None);
    let report=serde_json::json!({"csp-report":{"document-uri":url.as_str(),"effective-directive":"img-src","blocked-uri":"https://example.invalid/private-path"}}).to_string();
    let wrong_origin = request(
        &url,
        "POST",
        &path,
        "Content-Type: application/csp-report\r\nOrigin: https://example.invalid\r\n",
        &report,
    )
    .await;
    assert!(wrong_origin.starts_with("HTTP/1.1 403"));
    assert_eq!(server.snapshot().error, None);
    let response = request(
        &url,
        "POST",
        &path,
        &format!("{headers}Origin: null\r\n"),
        &report,
    )
    .await;
    assert!(response.starts_with("HTTP/1.1 204"));
    assert_eq!(
        server.snapshot().error,
        Some("Report requires content blocked by its isolation policy")
    );
    assert!(server.url().is_err());
    let failed = request(&url, "GET", url.path(), "", "").await;
    assert!(failed.starts_with("HTTP/1.1 422"));
    assert!(!failed.contains("private-path"));
    server.close().await.unwrap();
}
#[tokio::test]
async fn closing_or_dropping_the_owner_closes_partial_http_connections() {
    for explicit in [false, true] {
        let (_root, doc) = document().await;
        let server = DocumentServer::start(doc, "http://127.0.0.1:4811")
            .await
            .unwrap();
        let url = server.url().unwrap();
        let mut observed = server.subscribe();
        let mut slow = tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
            .await
            .unwrap();
        slow.write_all(b"GET / HTTP/1.1\r\nHost: ").await.unwrap();
        if explicit {
            server.close().await.unwrap();
        }
        drop(server);
        tokio::time::timeout(Duration::from_secs(2), observed.wait_for(|s| s.closed))
            .await
            .unwrap()
            .unwrap();
        let mut bytes = Vec::new();
        let result = tokio::time::timeout(Duration::from_secs(2), slow.read_to_end(&mut bytes))
            .await
            .unwrap();
        assert!(
            result.is_ok()
                || matches!(
                    result.unwrap_err().kind(),
                    std::io::ErrorKind::ConnectionReset
                )
        );
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", url.port().unwrap()))
                .await
                .is_err()
        );
    }
}
#[tokio::test]
async fn framing_requires_an_exact_workspace_origin() {
    let (_root, doc) = document().await;
    for origin in [
        "http://localhost:4811",
        "http://127.0.0.1:*",
        "https://example.invalid",
        "http://127.0.0.1:4811/path",
    ] {
        assert!(DocumentServer::start(doc.clone(), origin).await.is_err());
    }
}
