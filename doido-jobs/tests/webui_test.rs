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
        .clone()
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
    assert!(html.contains("id=\"live-poll-btn\""));
    assert!(html.contains("Live poll"));
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
        .clone()
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
    assert!(html.contains("class=\"relative-time"));
    assert!(html.contains(" seconds ago") || html.contains("just now"));
    assert!(html.contains("title=\""));
    assert!(html.contains(" - ") && html.contains(":"));
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
        .clone()
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
        .clone()
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
        .clone()
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
        .clone()
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
async fn webui_enqueue_payload_array_inserts_multiple_jobs() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .clone()
        .oneshot(form_post(
            "/doido/jobs/enqueue",
            "job_name=webui_enqueue_probe&payload=%5B1%2C2%2C3%5D&mode=enqueue_now",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);

    let page = queue
        .list_jobs("default", JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert_eq!(page.jobs.len(), 3);
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
        .clone()
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
        .clone()
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
async fn webui_in_queue_tab_has_bulk_form() {
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
        .clone()
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
    assert!(html.contains("action=\"/doido/jobs/bulk\""));
    assert!(html.contains("name=\"job_ids\""));
    assert!(html.contains("Delete selected"));
}

#[tokio::test]
async fn webui_bulk_delete_selected() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let id1 = queue
        .enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();
    let id2 = queue
        .enqueue(JobPayload::new("default", json!({}), 3))
        .await
        .unwrap();

    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .clone()
        .oneshot(form_post(
            "/doido/jobs/bulk",
            &format!("action=delete_selected&tab=in_queue&queue=default&job_ids={id1}"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);

    let page = queue
        .list_jobs("default", JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert_eq!(page.jobs.len(), 1);
    assert_eq!(page.jobs[0].id, id2);
}

#[tokio::test]
async fn webui_bulk_discard_selected_dead() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let id1 = queue
        .enqueue(JobPayload::new("default", json!({}), 0))
        .await
        .unwrap();
    let id2 = queue
        .enqueue(JobPayload::new("default", json!({}), 0))
        .await
        .unwrap();
    for id in [&id1, &id2] {
        let r = queue
            .reserve(&["default"], std::time::Duration::ZERO)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.job.id, *id);
        queue.dead_letter(id, "fail").await.unwrap();
    }

    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .clone()
        .oneshot(form_post(
            "/doido/jobs/bulk",
            &format!("action=discard_selected&tab=dead&queue=default&job_ids={id1}"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    assert_eq!(queue.dead_jobs("default").await.unwrap().len(), 1);
    assert_eq!(queue.dead_jobs("default").await.unwrap()[0].id, id2);
}

#[tokio::test]
async fn webui_bulk_retry_all_via_post() {
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
    queue.dead_letter(&r.job.id, "fail").await.unwrap();

    let state = JobsWebState::new(queue.clone(), test_jobs_config());
    let app = webui::routes(state);

    let response = app
        .clone()
        .oneshot(form_post(
            "/doido/jobs/bulk",
            "action=retry_all&tab=dead&queue=default",
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::SEE_OTHER);
    assert!(queue.dead_jobs("default").await.unwrap().is_empty());
    let pending = queue
        .list_jobs("default", JobBucket::InQueue, 10, None, true)
        .await
        .unwrap();
    assert!(!pending.jobs.is_empty());
    let _ = id;
}

#[tokio::test]
async fn webui_routes_can_be_merged_into_app() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let state = JobsWebState::new(queue, test_jobs_config());

    // Developer merges the webui router into their application router
    let app = Router::new().merge(webui::routes(state));

    let response = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);
}

#[tokio::test]
async fn webui_routes_at_directs_custom_endpoint() {
    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let state = JobsWebState::new(queue, test_jobs_config());

    // Developer directs the endpoint to a custom path (e.g. /admin/queues)
    let app = Router::new().merge(webui::routes_at("/admin/queues", state));

    let response = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri("/admin/queues?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), http::StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let html = String::from_utf8(body.to_vec()).unwrap();
    // Links inside the page now direct to the custom endpoint
    assert!(html.contains("href=\"/admin/queues?tab=queues\""));
    assert!(html.contains("href=\"/admin/queues?tab=enqueue\""));

    let enqueue_resp = app
        .oneshot(
            http::Request::builder()
                .uri("/admin/queues?tab=enqueue")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(enqueue_resp.status(), http::StatusCode::OK);
    let enqueue_body = enqueue_resp.into_body().collect().await.unwrap().to_bytes();
    let enqueue_html = String::from_utf8(enqueue_body.to_vec()).unwrap();
    assert!(enqueue_html.contains("action=\"/admin/queues/enqueue\""));
}

#[tokio::test]
async fn webui_developer_can_attach_custom_auth_layer() {
    use axum::http::StatusCode;
    use axum::middleware::{self, Next};
    use axum::response::Response;

    async fn custom_auth(req: http::Request<Body>, next: Next) -> Result<Response, StatusCode> {
        if req.headers().contains_key("x-admin-token") {
            Ok(next.run(req).await)
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }

    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let queue = Arc::new(MemoryQueue::new()) as Arc<dyn JobQueue>;
    let state = JobsWebState::new(queue, test_jobs_config());

    // Developer adds their own auth layer to the jobs dashboard router
    let jobs_router = webui::routes(state).layer(middleware::from_fn(custom_auth));
    let app = Router::new().merge(jobs_router);

    // Request without auth is rejected by developer's middleware
    let unauth_resp = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);

    // Request with auth passes through to the dashboard
    let auth_resp = app
        .oneshot(
            http::Request::builder()
                .uri("/doido/jobs?tab=queues")
                .header("x-admin-token", "secret123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(auth_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn build_jobs_webui_router_encapsulates_mount_and_auth_layer() {
    use axum::http::StatusCode;
    use axum::middleware::{self, Next};
    use axum::response::Response;

    async fn custom_auth(req: http::Request<Body>, next: Next) -> Result<Response, StatusCode> {
        if req.headers().contains_key("x-admin-token") {
            Ok(next.run(req).await)
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }

    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let is_dev = false;
    let auth_layer = (!is_dev).then(|| middleware::from_fn(custom_auth));

    let config = test_jobs_config();
    let app =
        webui::build_jobs_webui_router_with_config(config, Some("/custom/admin/jobs"), auth_layer);

    // Request without auth is rejected
    let unauth_resp = app
        .clone()
        .oneshot(
            http::Request::builder()
                .uri("/custom/admin/jobs?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth_resp.status(), StatusCode::UNAUTHORIZED);

    // Request with auth passes through
    let auth_resp = app
        .oneshot(
            http::Request::builder()
                .uri("/custom/admin/jobs?tab=queues")
                .header("x-admin-token", "valid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(auth_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn build_jobs_webui_router_defaults_mount_path_from_config() {
    use axum::http::StatusCode;

    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let mut config = test_jobs_config();
    config.webui.mount_path = "/configured/jobs".to_string();

    let app = webui::build_jobs_webui_router_with_config::<tower::layer::util::Identity>(
        config, None, None,
    );

    let resp = app
        .oneshot(
            http::Request::builder()
                .uri("/configured/jobs?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn build_unprotected_jobs_webui_router_serves_requests_without_type_annotations() {
    use axum::http::StatusCode;

    doido_jobs::webui::register_views();
    let _ = doido_view::init("app/views");

    let app = webui::build_unprotected_jobs_webui_router(Some("/unprotected/jobs"));

    let resp = app
        .oneshot(
            http::Request::builder()
                .uri("/unprotected/jobs?tab=queues")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn default_sync_queue_resolves_memory_and_db_pool() {
    use doido_jobs::config::{default_sync_queue, Backend};

    // 1. Memory backend returns an operational queue
    let memory_config = JobsConfig {
        backend: Backend::Memory,
        enqueue_batch_size: 42,
        ..JobsConfig::default()
    };
    let q_mem = default_sync_queue(&memory_config);
    let job_id = q_mem
        .enqueue(JobPayload::new("default", json!({}), 1))
        .await
        .unwrap();
    assert!(!job_id.is_empty());

    // 2. Db backend with pool initialized connects db queue
    let _pool_guard = doido_model::pool::test_lock();
    if doido_model::pool::try_pool().is_none() {
        let conn = doido_model::connect_with_url("sqlite::memory:")
            .await
            .unwrap();
        let _ = doido_model::pool::set_pool(conn);
    }
    if let Some(conn) = doido_model::pool::try_pool() {
        doido_jobs::DbQueue::new(conn.clone())
            .migrate()
            .await
            .unwrap();
    }
    let db_config = JobsConfig {
        backend: Backend::Db,
        enqueue_batch_size: 10,
        ..JobsConfig::default()
    };
    let q_db = default_sync_queue(&db_config);
    let db_job_id = q_db
        .enqueue(JobPayload::new("default", json!({}), 1))
        .await
        .unwrap();
    assert!(!db_job_id.is_empty());
}
