use crate::services::{auth, i18n, listing};
use doido::controller::{controller, Context, Response};

pub struct DashboardController;

#[controller]
impl DashboardController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        if auth::optional_user_id(ctx).is_none() {
            return Ok(ctx.redirect_to("/users/sign_in"));
        }
        let page =
            listing::layout_context(ctx, &i18n::t("nav.dashboard")).await?;
        Ok(ctx.render("dashboard/index", page))
    }
}
