use doido_controller::axum;

/// Boots the HTTP server with the application's `routes`, including optional auth
/// wiring when the `auth` feature is enabled and the project depends on it.
pub async fn run(routes: Option<axum::Router>, env: Option<String>, port: Option<u16>) {
    match routes {
        Some(router) => {
            if let Some(env) = env.as_deref() {
                std::env::set_var("DOIDO_ENV", env);
            }

            if let Err(e) = doido_model::pool::init().await {
                doido_core::tracing::error!("failed to connect to the database: {e}");
                return;
            }

            #[cfg(feature = "auth")]
            if doido_generators::commands::generate::project_has_doido_auth() {
                doido_auth::register_views();
            }

            if let Err(e) = doido_view::init("app/views") {
                doido_core::tracing::warn!("failed to load views from app/views: {e}");
            }

            #[cfg(feature = "auth")]
            if doido_generators::commands::generate::project_has_doido_auth() {
                let config = doido_auth::load_config();
                if let Err(e) = doido_auth::init(doido_model::pool::pool().clone(), &config).await {
                    doido_core::tracing::warn!("failed to initialise auth: {e}");
                }
            }

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
