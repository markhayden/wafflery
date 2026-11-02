//! A completed TCP handshake is not evidence that a closed content server is usable.
use std::{io::ErrorKind, time::Duration};
use tokio::{io::AsyncReadExt, net::TcpStream};

pub(super) async fn assert_closed(url: &url::Url) {
    match TcpStream::connect(("127.0.0.1", url.port().unwrap())).await {
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionRefused
                    | ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionAborted
            ) => {}
        Ok(stream) => assert_disconnected(stream).await,
        other => panic!("unexpected connection result after closure: {other:?}"),
    }
}
async fn assert_disconnected(mut stream: TcpStream) {
    // macOS may finish connect successfully while the peer is already CLOSED.
    // Require EOF/reset, never a live socket that merely sends no data.
    let result = tokio::time::timeout(Duration::from_secs(2), stream.read(&mut [0; 1]))
        .await
        .expect("closed content socket remained open");
    match result {
        Ok(0) => (),
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted | ErrorKind::NotConnected
            ) => {}
        other => panic!("closed content socket remained usable: {other:?}"),
    }
}
#[tokio::test]
async fn an_already_connected_peer_is_closed_after_eof() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap())
        .await
        .unwrap();
    let (peer, _) = listener.accept().await.unwrap();
    drop(peer);
    drop(listener);
    assert_disconnected(client).await;
}
