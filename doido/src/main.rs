#[tokio::main]
async fn main() {
    // No application routes here, so `server` reports it has nothing to start.
    doido_generators::run(None).await;
}
