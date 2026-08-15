use doido::controller::{controller, Context, Response};

pub struct HelloController;

#[controller]
impl HelloController {
    pub async fn index(ctx: Context) -> Response {
        ctx.redirect_to("/chat")
    }
}
