//! HTTP dashboard for inspecting and managing background job queues.

mod auth;
mod bulk;
mod dashboard;
mod enqueue;
mod framework_views;

pub use framework_views::register_views;

use crate::config::JobsConfig;
use crate::inspect::QueueCounts;
use crate::queue::JobQueue;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use doido_core::Environment;
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
}

/// Build the jobs dashboard router (not merged into the app yet).
pub fn routes(state: JobsWebState) -> Router {
    let mount = normalize_mount(&state.config.webui.mount_path);
    let mut router = Router::new()
        .route("/", get(dashboard::index))
        .route("/retry/{queue}/{job_id}", get(dashboard::retry_one))
        .route("/discard/{queue}/{job_id}", get(dashboard::discard_one))
        .route("/retry_all", get(dashboard::retry_all))
        .route("/discard_all", get(dashboard::discard_all))
        .route("/bulk", post(bulk::submit))
        .route("/enqueue", post(enqueue::submit))
        .with_state(state.clone());

    if let (Some(user), Some(pass)) = (
        state.config.webui.username.clone(),
        state.config.webui.password.clone(),
    ) {
        if auth::credentials_configured(&state.config.webui) {
            let creds = Arc::new(auth::BasicAuthCredentials {
                username: user,
                password: pass,
            });
            router = router.layer(middleware::from_fn_with_state(
                creds,
                auth::basic_auth_middleware,
            ));
        }
    }

    Router::new().nest(&mount, router)
}

/// Mount the dashboard when [`WebUiConfig::safe_to_mount`] allows it.
pub fn try_merge_routes(router: Router, state: JobsWebState) -> Router {
    let env = Environment::get_env();
    if state.config.webui.safe_to_mount(env) {
        router.merge(routes(state))
    } else if state.config.webui.enabled {
        if state.config.webui.requires_credentials(env) {
            doido_core::tracing::warn!(
                "jobs.webui.enabled is true but username/password are required in production; \
                 dashboard not mounted"
            );
        }
        router
    } else {
        router
    }
}

fn normalize_mount(path: &str) -> String {
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
