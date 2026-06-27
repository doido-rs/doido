mod controllers;

#[path = "../config/routes.rs"]
mod routes;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("listening on http://0.0.0.0:3000");
    doido::controller::axum::serve(listener, routes::router()).await?;
    Ok(())
}
