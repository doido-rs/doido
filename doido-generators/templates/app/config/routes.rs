use crate::controllers::HelloController;
use doido::router::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        get!("/", HelloController::index);
    }
}
