use doido_controller::axum;

/// Boots the HTTP server with the application's `routes`.
///
/// `env` (from `--env`) and `port` (from `--port`) override the environment and
/// listen port for this run. When `routes` is `None` — as it is for the
/// standalone `doido-generators` binary, which carries no application routes —
/// the server is not started.
pub async fn run(routes: Option<axum::Router>, env: Option<String>, port: Option<u16>) {
    match routes {
        Some(router) => {
            // The global tracing subscriber is installed by `cli::run` before
            // dispatch, so the steps below already log through the centralized
            // logger.

            // Apply `--env` before any config-driven boot step, so the DB pool,
            // cache, and server bind all read `config/<env>.yml` for that env.
            // (The logger's verbosity was already installed from the ambient env.)
            if let Some(env) = env.as_deref() {
                std::env::set_var("DOIDO_ENV", env);
            }

            // Install the global DB pool before serving so controllers can reach
            // it via `Context::db()`. A failure here is fatal — handlers would
            // otherwise panic on first use.
            if let Err(e) = doido_model::pool::init().await {
                doido_core::tracing::error!("failed to connect to the database: {e}");
                return;
            }

            // Auth apps use the framework's built-in controllers/views (nothing is
            // copied into the project by default). Register the built-in auth views
            // *before* the view engine loads so the app can override them by name,
            // and install auth state so `CurrentUser`/`RequireAuth` guards resolve.
            #[cfg(feature = "auth-generators")]
            let auth_app = crate::commands::generate::project_has_doido_auth();
            #[cfg(feature = "auth-generators")]
            if auth_app {
                doido_auth::register_views();
            }

            // Install the Tera view engine over `app/views` so `Context::render`
            // works. Non-fatal: an app may serve JSON only.
            if let Err(e) = doido_view::init("app/views") {
                doido_core::tracing::warn!("failed to load views from app/views: {e}");
            }

            #[cfg(feature = "auth-generators")]
            if auth_app {
                let config = doido_auth::load_config();
                if let Err(e) =
                    doido_auth::init(doido_model::pool::pool().clone(), &config).await
                {
                    doido_core::tracing::warn!("failed to initialise auth: {e}");
                }
            }
            // Install the global cache store from the `cache` config section
            // (in-memory by default). Non-fatal: an app may not use the cache,
            // and the memory backend never fails.
            if let Err(e) = doido_cache::init_cache().await {
                doido_core::tracing::warn!("failed to initialize cache: {e}");
            }
            if let Err(e) = doido_controller::start_server_with(router, None, port).await {
                doido_core::tracing::error!("server error: {e}");
            }
        }
        None => {
            doido_core::tracing::warn!("no routes configured; server not started");
        }
    }
}
