use crate::controllers::HelloController;
use doido::controller::{axum, routes};

pub fn router() -> axum::Router {
    routes! {
        get!("/", HelloController::index);
    }
}
