use doido_mailer::smtp::{build_message, SmtpDeliverer};
use doido_mailer::{Deliverer, Mail};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

/// A minimal mock SMTP server: play the conversation, capture the DATA payload.
async fn mock_smtp(listener: TcpListener) -> String {
    let (mut sock, _) = listener.accept().await.unwrap();
    let (r, mut w) = sock.split();
    let mut reader = BufReader::new(r);
    let mut line = String::new();

    w.write_all(b"220 mock\r\n").await.unwrap();
    for reply in ["250 ok\r\n", "250 ok\r\n", "250 ok\r\n"] {
        line.clear();
        reader.read_line(&mut line).await.unwrap(); // EHLO / MAIL FROM / RCPT TO
        w.write_all(reply.as_bytes()).await.unwrap();
    }
    line.clear();
    reader.read_line(&mut line).await.unwrap(); // DATA
    w.write_all(b"354 go\r\n").await.unwrap();

    let mut data = String::new();
    loop {
        line.clear();
        reader.read_line(&mut line).await.unwrap();
        if line.trim_end() == "." {
            break;
        }
        data.push_str(&line);
    }
    w.write_all(b"250 ok\r\n").await.unwrap();
    data
}

#[tokio::test]
async fn smtp_deliverer_sends_the_message() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let server = tokio::spawn(mock_smtp(listener));

    let mail = Mail::new()
        .from("a@x.com")
        .to("b@y.com")
        .subject("Hi")
        .body_text("Hello there");
    SmtpDeliverer::new(addr).deliver(&mail).await.unwrap();

    let data = server.await.unwrap();
    assert!(data.contains("To: b@y.com"), "captured: {data}");
    assert!(data.contains("Subject: Hi"));
    assert!(data.contains("Hello there"));
}

#[test]
fn build_message_has_headers_and_body() {
    let mail = Mail::new().to("b@y.com").subject("Hi").body_text("Hello");
    let msg = build_message(&mail);
    assert!(msg.contains("To: b@y.com"));
    assert!(msg.contains("Subject: Hi"));
    assert!(msg.contains("Content-Type: text/plain"));
    assert!(msg.ends_with("Hello"));
}

#[tokio::test]
async fn smtp_deliverer_errors_when_unreachable() {
    let mail = Mail::new().to("b@y.com").subject("x");
    assert!(SmtpDeliverer::new("127.0.0.1:1")
        .deliver(&mail)
        .await
        .is_err());
}
