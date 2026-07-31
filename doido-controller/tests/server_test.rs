use doido_controller::axum::{routing::get, Router};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
async fn start_server_with_binds_and_serves_requests() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let app = Router::new().route("/", get(|| async { "hello" }));
    let handle = tokio::spawn(async move {
        doido_controller::server::start_server_with(app, Some("127.0.0.1".into()), Some(port)).await
    });

    for _ in 0..50 {
        if TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .is_ok()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .expect("server should accept connections");
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();

    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await.unwrap();
    let response = String::from_utf8_lossy(&buf[..n]);
    assert!(
        response.contains("200"),
        "expected HTTP 200, got: {response}"
    );

    handle.abort();
}
