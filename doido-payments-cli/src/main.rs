//! Full Doido CLI with payment generators always enabled.

#[tokio::main]
async fn main() {
    doido::run(None).await;
}
