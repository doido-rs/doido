use criterion::{black_box, criterion_group, criterion_main, Criterion};
use doido_controller::axum::{routing::get, Router};
use doido_controller::{EncryptedCookieSessionStore, MiddlewareStack, Session};
use http::{Request, StatusCode};
use tower::ServiceExt;

fn route_dispatch(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let app = MiddlewareStack::new().apply(
        Router::new().route(
            "/hello",
            get(|| async { (StatusCode::OK, "ok") }),
        ),
    );

    c.bench_function("middleware_stack_oneshot", |b| {
        b.to_async(&rt).iter(|| async {
            let req = Request::builder()
                .uri("/hello")
                .body(doido_controller::axum::body::Body::empty())
                .unwrap();
            let resp = app.clone().oneshot(req).await.unwrap();
            black_box(resp.status());
        });
    });
}

fn session_encrypt(c: &mut Criterion) {
    let store = EncryptedCookieSessionStore::new(b"bench-secret-key-32-bytes-long!!");
    let mut session = Session::new();
    session.set("user_id", 42_i64);
    session.set("roles", vec!["admin", "editor"]);

    c.bench_function("encrypted_session_encode", |b| {
        b.iter(|| {
            let cookie = store.encode(black_box(&session));
            black_box(cookie);
        });
    });

    let cookie = store.encode(&session);
    c.bench_function("encrypted_session_decode", |b| {
        b.iter(|| {
            let decoded = store.decode(black_box(&cookie));
            black_box(decoded);
        });
    });
}

criterion_group!(benches, route_dispatch, session_encrypt);
criterion_main!(benches);
