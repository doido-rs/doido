//! Redis pub/sub integration test — runs only with `--features cable-redis` and
//! a reachable `REDIS_URL` (see `make test-backends`); otherwise it self-skips.
#![cfg(feature = "cable-redis")]

use doido_cable::{PubSub, RedisPubSub};
use std::time::Duration;

#[tokio::test]
async fn redis_pubsub_delivers_across_a_stream() {
    let Ok(url) = std::env::var("REDIS_URL") else {
        eprintln!("REDIS_URL unset; skipping");
        return;
    };
    let ps = RedisPubSub::connect(&url).await.unwrap();
    let mut rx = ps.subscribe("doido:test:stream").await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await; // let the bridge subscribe
    ps.publish("doido:test:stream", "hello").await.unwrap();

    let msg = tokio::time::timeout(Duration::from_secs(2), rx.recv())
        .await
        .expect("no message within timeout")
        .unwrap();
    assert_eq!(msg, "hello");
}
