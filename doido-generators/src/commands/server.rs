use doido_controller::axum;

/// Boots the HTTP server with the application's `routes`.
///
/// When `routes` is `None` — as it is for the standalone `doido-generators`
/// binary, which carries no application routes — the server is not started.
pub async fn run(routes: Option<axum::Router>) {
    match routes {
        Some(router) => {
            // Install the global tracing subscriber first so connection and
            // request logs from the steps below are captured.
            doido_core::logger::init();

            // Install the global DB pool before serving so controllers can reach
            // it via `Context::db()`. A failure here is fatal — handlers would
            // otherwise panic on first use.
            if let Err(e) = doido_model::pool::init().await {
                eprintln!("failed to connect to the database: {e}");
                return;
            }
            // Install the Tera view engine over `app/views` so `Context::render`
            // works. Non-fatal: an app may serve JSON only.
            if let Err(e) = doido_view::init("app/views") {
                eprintln!("warning: failed to load views from app/views: {e}");
            }
            if let Err(e) = doido_controller::start_server(router).await {
                eprintln!("server error: {e}");
            }
        }
        None => {
            println!("No routes configured; server not started.");
        }
    }
}
