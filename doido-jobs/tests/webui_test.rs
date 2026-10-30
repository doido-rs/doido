use axum::body::Body;
use axum::Router;
use chrono::{Duration, Utc};
use doido_jobs::config::{JobsConfig, WebUiConfig};
use doido_jobs::inspect::JobBucket;
use doido_jobs::queue::{JobPayload, JobQueue};
use doido_jobs::webui::{self, JobsWebState};
use doido_jobs::MemoryQueue;
use doido_view;
use http_body_util::BodyExt;
use serde_json::json;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tower::ServiceExt;

static WEBUI_PROBE_RAN: AtomicU32 = AtomicU32::new(0);

#[doido_jobs::job]
async fn webui_enqueue_probe(n: u32) -> doido_core::Result<()> {
    WEBUI_PROBE_RAN.fetch_add(n, Ordering::SeqCst);
    Ok(())
}

#[doido_jobs::job]
async fn webui_perform_ping() -> doido_core::Result<()> {
    WEBUI_PROBE_RAN.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

fn test_jobs_config() -> JobsConfig {
    JobsConfig {
        webui: WebUiConfig {
            enabled: true,
            mount_path: "/doido/jobs".to_string(),
            username: None,
            password: None,
            stats_ttl_secs: None,
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
    assert!(html.contains("Show payload"));
    assert!(!html.contains("<pre class=\"payload\">"));
}

#[tokio::test]
async fn webui_include_payload_renders_pre() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    queue
        .enqueue(JobPayload::new("default", json!({"x": 1}), 3))
        .await
        .unwrap();

    let state = JobsWebState::new(queue, test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=in_queue&queue=default&include_payload=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("<pre class=\"payload\">"));
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

fn form_post(uri: &str, fields: &str) -> http::Request<Body> {
    http::Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(fields.to_string()))
        .unwrap()
}

#[tokio::test]
async fn webui_enqueue_tab_lists_registered_job() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let state = JobsWebState::new(
        Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>,
        test_jobs_config(),
    );
    let app = webui::routes(state);

    let response = app
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=enqueue")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("webui_enqueue_probe"));
    assert!(html.contains("Enqueue now"));
}

#[tokio::test]
async fn webui_enqueue_now_inserts_job() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .oneshot(form_post(
            "/doido/jobs/enqueue",
            "job_name=webui_enqueue_probe&payload=1&mode=enqueue_now",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);

    let page = queue
        .list_jobs("default", JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert_eq!(page.jobs.len(), 1);
    assert_eq!(page.jobs[0].job_name, "webui_enqueue_probe");
}

#[tokio::test]
async fn webui_enqueue_at_schedules_job() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let at = Utc::now() + Duration::hours(2);
    let run_at = at.format("%Y-%m-%dT%H:%M").to_string();
    let body = format!("job_name=webui_enqueue_probe&payload=1&mode=enqueue_at&run_at={run_at}");

    let response = app
        .oneshot(form_post("/doido/jobs/enqueue", &body))
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);

    let counts = queue.queue_counts(&["default"]).await.unwrap();
    assert_eq!(counts[0].1.scheduled, 1);
}

#[tokio::test]
async fn webui_perform_now_runs_without_queue_row() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    WEBUI_PROBE_RAN.store(0, Ordering::SeqCst);
    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .oneshot(form_post(
            "/doido/jobs/enqueue",
            "job_name=webui_perform_ping&mode=perform_now",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(WEBUI_PROBE_RAN.load(Ordering::SeqCst), 1);

    let page = queue
        .list_jobs("default", JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert!(page.jobs.is_empty());
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
