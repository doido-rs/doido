use doido_controller::axum;

/// Boots the HTTP server with the application's `routes`.
///
/// When `routes` is `None` — as it is for the standalone `doido-generators`
/// binary, which carries no application routes — the server is not started.
pub async fn run(routes: Option<axum::Router>) {
    match routes {
        Some(router) => {
            if let Err(e) = doido_controller::start_server(router).await {
                eprintln!("server error: {e}");
            }
        }
        None => {
            println!("No routes configured; server not started.");
        }
    }
}
