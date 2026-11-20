use crate::axum;

/// Boots the HTTP server with the application's `routes`.
pub async fn run(routes: Option<axum::Router>, env: Option<String>, port: Option<u16>) {
    match routes {
        Some(router) => {
            if let Some(env) = env.as_deref() {
                std::env::set_var("DOIDO_ENV", env);
            }

            // Ensure all default initializers run
            if let Err(e) = doido_core::initializers::Initializers::with_defaults().run_all() {
                doido_core::tracing::error!("initializers failed: {e}");
            }

            let router = doido_storage::serving::merge_routes(router);

            if let Err(e) = crate::start_server_with(router, None, port).await {
                doido_core::tracing::error!("server error: {e}");
            }
        }
        None => {
            doido_core::tracing::warn!("no routes configured; server not started");
        }
    }
}
