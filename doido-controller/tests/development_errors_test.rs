//! Development error page tests. Each case sets/restores `DOIDO_ENV` because the
//! environment is process-global.

use doido_controller::axum::body::Body;
use doido_controller::axum::response::Response;
use doido_controller::axum::{routing::get, Router};
use doido_controller::development_errors::{
    render_development_error_page, DevelopmentErrorContext,
};
use doido_controller::{IntoActionResponse, MiddlewareStack};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use std::sync::{Mutex, MutexGuard};
use tower::ServiceExt;

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvGuard {
    previous: Option<String>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn set(value: &str) -> Self {
        let lock = ENV_LOCK.lock().unwrap();
        let previous = std::env::var("DOIDO_ENV").ok();
        std::env::set_var("DOIDO_ENV", value);
        Self {
            previous,
            _lock: lock,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var("DOIDO_ENV", value),
            None => std::env::remove_var("DOIDO_ENV"),
        }
    }
}

async fn body_string(response: Response) -> String {
    response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .pipe(|b| String::from_utf8(b.to_vec()).unwrap())
}

trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(Self) -> R,
    {
        f(self)
    }
}

impl<T> Pipe for T {}

#[test]
fn development_error_page_contains_message_and_backtrace() {
    let context = DevelopmentErrorContext::new(500, "database connection failed")
        .with_request_info("GET", "/posts", None);
    let response = render_development_error_page(&context);
    let body = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(body_string(response));

    assert!(body.contains("500"));
    assert!(body.contains("Internal Server Error"));
    assert!(body.contains("database connection failed"));
    assert!(body.contains("GET /posts"));
    assert!(body.contains("Backtrace") || body.contains("backtrace unavailable"));
}

#[tokio::test]
async fn action_error_shows_html_in_development() {
    let _guard = EnvGuard::set("development");

    let app = MiddlewareStack::new().apply(Router::new().route(
        "/fail",
        get(|| async {
            Result::<Response, String>::Err("simulated db failure".to_string())
                .into_action_response()
        }),
    ));

    let req = Request::builder()
        .uri("/fail")
        .header(http::header::ACCEPT, "text/html")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = body_string(resp).await;
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("simulated db failure"));
    assert!(body.contains("doido"));
}

#[tokio::test]
async fn action_error_plain_in_production() {
    let _guard = EnvGuard::set("production");

    let app = MiddlewareStack::new().apply(Router::new().route(
        "/fail",
        get(|| async {
            Result::<Response, String>::Err("simulated db failure".to_string())
                .into_action_response()
        }),
    ));

    let req = Request::builder()
        .uri("/fail")
        .header(http::header::ACCEPT, "text/html")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = body_string(resp).await;
    assert_eq!(body, "Internal Server Error");
    assert!(!body.contains("<!DOCTYPE html>"));
}

async fn panicking_handler() -> Response {
    panic!("boom in handler");
}

#[tokio::test]
async fn panic_shows_html_in_development() {
    let _guard = EnvGuard::set("development");

    let app = MiddlewareStack::new().apply(Router::new().route("/panic", get(panicking_handler)));

    let req = Request::builder()
        .uri("/panic")
        .header(http::header::ACCEPT, "text/html")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = body_string(resp).await;
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("boom in handler"));
}

#[tokio::test]
async fn api_only_skips_development_page() {
    let _guard = EnvGuard::set("development");

    let app = MiddlewareStack::new()
        .with_api_only(true)
        .apply(Router::new().route(
            "/fail",
            get(|| async {
                Result::<Response, String>::Err("hidden failure".to_string()).into_action_response()
            }),
        ));

    let req = Request::builder()
        .uri("/fail")
        .header(http::header::ACCEPT, "text/html")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = body_string(resp).await;
    assert_eq!(body, "Internal Server Error");
    assert!(!body.contains("<!DOCTYPE html>"));
}

#[tokio::test]
async fn json_accept_skips_html_page() {
    let _guard = EnvGuard::set("development");

    let app = MiddlewareStack::new().apply(Router::new().route(
        "/fail",
        get(|| async {
            Result::<Response, String>::Err("json client failure".to_string())
                .into_action_response()
        }),
    ));

    let req = Request::builder()
        .uri("/fail")
        .header(http::header::ACCEPT, "application/json")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = body_string(resp).await;
    assert_eq!(body, "Internal Server Error");
    assert!(!body.contains("<!DOCTYPE html>"));
}

#[tokio::test]
async fn not_found_shows_routing_error_in_development() {
    let _guard = EnvGuard::set("development");

    let app = MiddlewareStack::new().apply(Router::new().route("/", get(|| async { "ok" })));

    let req = Request::builder()
        .uri("/missing")
        .header(http::header::ACCEPT, "text/html")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body = body_string(resp).await;
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("404"));
    assert!(body.contains("GET /missing"));
}
