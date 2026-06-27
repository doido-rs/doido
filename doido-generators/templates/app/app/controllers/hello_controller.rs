use doido_controller::controller;
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(
        ctx: doido_controller::Context,
    ) -> doido_controller::Response {
        ctx.json(json!({ "message": "Hello word!" }))
    }
}
