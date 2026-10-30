use axum::body::Body;
use axum::Router;
use doido_jobs::config::{JobsConfig, WebUiConfig};
use doido_jobs::queue::{JobPayload, JobQueue};
use doido_jobs::webui::{self, JobsWebState};
use doido_jobs::MemoryQueue;
use doido_view;
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::Arc;
use tower::ServiceExt;

fn test_jobs_config() -> JobsConfig {
    JobsConfig {
        webui: WebUiConfig {
            enabled: true,
            mount_path: "/doido/jobs".to_string(),
            username: None,
            password: None,
        },
        ..JobsConfig::default()
    }
}

#[tokio::test]
async fn webui_renders_queues_tab() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    queue
        .enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();

    let state = JobsWebState::new(queue, test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("Doido Jobs"));
    assert!(html.contains("pending_ready") || html.contains("Ready"));
    assert!(html.contains("aria-label=\"Queues pagination\""));
    assert!(html.contains("Page 1 of 1"));
    assert!(html.contains("aria-disabled=\"true\""));
}

#[tokio::test]
async fn webui_job_tab_exposes_cursor_pagination() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    for _ in 0..55 {
        queue
            .enqueue(JobPayload::new("default", json!({}), 3))
            .await
            .unwrap();
    }

    let state = JobsWebState::new(queue, test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=in_queue&queue=default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("aria-label=\"Jobs pagination\""));
    assert!(html.contains("page_token="));
    assert!(!html.contains("aria-disabled=\"true\">Next</span>"));
    assert!(!html.contains("offset="));
}

#[tokio::test]
async fn webui_retry_dead_job_via_http() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let id = queue
        .enqueue(JobPayload::new("default", json!({}), 0))
        .await
        .unwrap();
    let r = queue
        .reserve(&["default"], std::time::Duration::ZERO)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.job.id, id);
    queue.dead_letter(&id, "fail").await.unwrap();

    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .oneshot(
            http::Request::builder()
                .uri(format!("/doido/jobs/retry/default/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    assert!(queue.dead_jobs("default").await.unwrap().is_empty());
}

#[tokio::test]
async fn webui_try_merge_respects_production_credentials() {
    use doido_core::Environment;
    let mut cfg = test_jobs_config();
    cfg.webui.username = None;
    cfg.webui.password = None;
    let state = JobsWebState::new(Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>, cfg);
    std::env::set_var("DOIDO_ENV", "production");
    let merged = webui::try_merge_routes(Router::new(), state);
    std::env::set_var("DOIDO_ENV", Environment::Development.as_str());
    let response = merged
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::NOT_FOUND);
}
