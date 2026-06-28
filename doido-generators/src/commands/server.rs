use doido_controller::axum;

/// Boots the HTTP server with the application's `routes`.
///
/// When `routes` is `None` — as it is for the standalone `doido-generators`
/// binary, which carries no application routes — the server is not started.
pub async fn run(routes: Option<axum::Router>) {
    match routes {
        Some(router) => {
            // The global tracing subscriber is installed by `cli::run` before
            // dispatch, so the steps below already log through the centralized
            // logger.

            // Install the global DB pool before serving so controllers can reach
            // it via `Context::db()`. A failure here is fatal — handlers would
            // otherwise panic on first use.
            if let Err(e) = doido_model::pool::init().await {
                doido_core::tracing::error!("failed to connect to the database: {e}");
                return;
            }
            // Install the Tera view engine over `app/views` so `Context::render`
            // works. Non-fatal: an app may serve JSON only.
            if let Err(e) = doido_view::init("app/views") {
                doido_core::tracing::warn!("failed to load views from app/views: {e}");
            }
            if let Err(e) = doido_controller::start_server(router).await {
                doido_core::tracing::error!("server error: {e}");
            }
        }
        None => {
            doido_core::tracing::warn!("no routes configured; server not started");
        }
    }
}
