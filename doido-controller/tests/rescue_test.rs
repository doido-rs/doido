use axum::body::Body;
use axum::response::Response;
use doido_controller::rescue::RescueHandlers;
use http::StatusCode;

#[derive(Debug)]
struct RecordNotFound;
impl std::fmt::Display for RecordNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "record not found")
    }
}
impl std::error::Error for RecordNotFound {}

fn handlers() -> RescueHandlers {
    RescueHandlers::new().on::<RecordNotFound>(|_e| {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap()
    })
}

#[test]
fn registered_error_is_rescued_to_its_response() {
    let err = doido_core::anyhow::Error::new(RecordNotFound);
    let resp = handlers()
        .rescue(&err)
        .expect("registered error is handled");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[test]
fn unregistered_error_is_not_rescued() {
    let err = doido_core::anyhow::anyhow!("some other failure");
    assert!(
        handlers().rescue(&err).is_none(),
        "an unregistered error falls through to the default 500"
    );
}
