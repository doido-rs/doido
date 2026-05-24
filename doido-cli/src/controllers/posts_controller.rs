use doido_controller::controller;

pub struct PostsController;

#[controller]
impl PostsController {
    pub async fn index(ctx: doido_controller::Context) -> doido_controller::Response {
        ctx.status(200)
    }
}
