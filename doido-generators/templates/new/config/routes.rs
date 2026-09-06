use crate::controllers::HelloController;
use doido::controller::{axum, routes};

// Storage serving routes (`/doido/storage/...`) are mounted automatically at
// boot when storage initialises successfully.

pub fn router() -> axum::Router {
    routes! {
        get!("/", HelloController::index);
    }
}
