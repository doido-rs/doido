use crate::helpers::ApplicationHelper;
use doido::controller::controller;
use serde_json::json;

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(
        ctx: doido::controller::Context,
    ) -> doido::controller::Response {
        ctx.json(json!({
            "message": ApplicationHelper::greet("world")
        }))
    }
}
