use crate::controllers::HelloController;
use crate::models::user::Model as User;
use doido::controller::axum;

pub fn router() -> axum::Router {
    doido::auth::routes! {
        get!("/", HelloController::index);
        auth_routes!(User);
    }
}
