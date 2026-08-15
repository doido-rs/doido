use crate::services::{i18n, listing, reports, tenant};
use doido::controller::{controller, Context, Response};
use serde_json::json;

pub struct ReportsController;

#[controller]
impl ReportsController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let report = reports::reports_index(ctx.db(), company_id).await?;
        let report_json =
            serde_json::to_value(&report).unwrap_or_else(|_| json!({}));
        let report_script = serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string());
        let mut page =
            listing::layout_context(ctx, &i18n::t("nav.reports")).await?;
        if let Some(obj) = page.as_object_mut() {
            obj.insert("report".to_string(), report_json);
            obj.insert("report_script".to_string(), json!(report_script));
            obj.insert("labels".to_string(), listing::reports_labels());
        }

        Ok(ctx.respond_to()
            .html(|| ctx.render("reports/index", page))
            .json(|| ctx.json(report))
            .finish())
    }
}
