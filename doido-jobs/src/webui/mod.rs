//! HTTP dashboard for inspecting and managing background job queues.

mod bulk;
mod dashboard;
mod enqueue;
mod framework_views;
mod time_format;

pub use framework_views::register_views;

use crate::config::JobsConfig;
use crate::inspect::QueueCounts;
use crate::queue::JobQueue;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

type QueueCountsSnapshot = Vec<(String, QueueCounts)>;
type StatsCache = Arc<Mutex<Option<(Instant, QueueCountsSnapshot)>>>;

/// Shared state for dashboard routes.
#[derive(Clone)]
pub struct JobsWebState {
    pub queue: Arc<dyn JobQueue>,
    pub config: JobsConfig,
    pub(crate) stats_cache: StatsCache,
}

impl JobsWebState {
    pub fn new(queue: Arc<dyn JobQueue>, config: JobsConfig) -> Self {
        Self {
            queue,
            config,
            stats_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Construct dashboard state with default configuration for the given queue backend.
    pub fn from_queue(queue: Arc<dyn JobQueue>) -> Self {
        Self::new(queue, crate::config::load())
    }

    /// Set a custom mount path for the dashboard (used for URL helpers, form actions, and redirects).
    pub fn with_mount_path(mut self, mount_path: impl Into<String>) -> Self {
        self.config.webui.mount_path = normalize_mount(&mount_path.into());
        self
    }
}

/// Build the jobs dashboard inner router (un-nested, routes at `/`, `/retry/...`, `/bulk`, etc.).
///
/// Use this if you are mounting under a custom path via `Router::nest` or `mount!`, but ensure
/// `state.config.webui.mount_path` is configured to match the mount endpoint so links and redirects work.
pub fn router(state: JobsWebState) -> Router {
    register_views();
    Router::new()
        .route("/", get(dashboard::index))
        .route("/retry/{queue}/{job_id}", get(dashboard::retry_one))
        .route("/discard/{queue}/{job_id}", get(dashboard::discard_one))
        .route("/retry_all", get(dashboard::retry_all))
        .route("/discard_all", get(dashboard::discard_all))
        .route("/bulk", post(bulk::submit))
        .route("/enqueue", post(enqueue::submit))
        .with_state(state)
}

/// Build the jobs dashboard router nested under `state.config.webui.mount_path` (default `/doido/jobs`).
///
/// Ready to be merged into your application router via `app.merge(routes(state))`.
pub fn routes(state: JobsWebState) -> Router {
    let mount = normalize_mount(&state.config.webui.mount_path);
    Router::new().nest(&mount, router(state))
}

/// Build the jobs dashboard router nested under the specified mount path.
///
/// Updates `mount_path` in `state` to ensure internal links, forms, and redirects match the endpoint.
/// Ready to be merged into your application router via `app.merge(routes_at("/custom/path", state))`.
pub fn routes_at(mount_path: &str, mut state: JobsWebState) -> Router {
    let mount = normalize_mount(mount_path);
    state.config.webui.mount_path = mount.clone();
    Router::new().nest(&mount, router(state))
}

/// Normalize mount path to have a leading slash and no trailing slash.
pub fn normalize_mount(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "/doido/jobs".to_string();
    }
    if trimmed.starts_with('/') {
        trimmed.trim_end_matches('/').to_string()
    } else {
        format!("/{}", trimmed.trim_end_matches('/'))
    }
}
