#[tokio::main]
async fn main() {
    // The standalone generator binary carries no application routes, so the
    // `server` command will report that it has nothing to start.
    doido_generators::run(None).await;
}
