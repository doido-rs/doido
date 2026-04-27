use doido_controller::controller;

#[controller]
pub struct PostsController;

impl PostsController {
    pub async fn index(&self) -> &'static str {
        "ok"
    }
}
