use crate::services::{i18n, pagination::PaginatedResponse, tenant};
use doido::controller::respond::Format;
use doido::controller::{Context, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;

pub fn pagination_labels() -> serde_json::Value {
    json!({
        "showing": i18n::t("pagination.showing"),
        "per_page": i18n::t("pagination.per_page"),
        "per_page_label": i18n::t("pagination.per_page_label"),
        "total": i18n::t("pagination.total"),
        "page": i18n::t("pagination.page"),
        "previous": i18n::t("pagination.previous"),
        "next": i18n::t("pagination.next"),
        "export_csv": i18n::t("pagination.export_csv"),
    })
}

pub fn table_labels() -> serde_json::Value {
    json!({
        "id": i18n::t("tables.id"),
        "name": i18n::t("tables.name"),
        "code": i18n::t("tables.code"),
        "slug": i18n::t("tables.slug"),
        "email": i18n::t("tables.email"),
        "agency": i18n::t("tables.agency"),
        "account_number": i18n::t("tables.account_number"),
        "cpf_cnpj": i18n::t("tables.cpf_cnpj"),
        "account_type": i18n::t("tables.account_type"),
        "bank_id": i18n::t("tables.bank_id"),
        "country_id": i18n::t("tables.country_id"),
        "user_id": i18n::t("tables.user_id"),
        "company_id": i18n::t("tables.company_id"),
        "role": i18n::t("tables.role"),
        "salary": i18n::t("tables.salary"),
        "occurred_at": i18n::t("tables.occurred_at"),
        "amount": i18n::t("tables.amount"),
        "operation": i18n::t("tables.operation"),
        "movement_type": i18n::t("tables.movement_type"),
        "category_id": i18n::t("tables.category_id"),
        "bank_account_id": i18n::t("tables.bank_account_id"),
        "counterparty_id": i18n::t("tables.counterparty_id"),
        "source": i18n::t("imports.labels.source"),
        "statement_type": i18n::t("imports.labels.statement_type"),
        "original_filename": i18n::t("imports.labels.original_filename"),
        "transactions_imported": i18n::t("imports.labels.transactions_imported"),
        "status": i18n::t("imports.labels.status"),
        "created_at": i18n::t("tables.created_at"),
    })
}

pub fn respond_index<T: Serialize>(
    ctx: &Context,
    template: &str,
    title: &str,
    base_path: &str,
    export_path: &str,
    page: PaginatedResponse<T>,
    current_company_id: Option<i64>,
) -> Response {
    let rows = serde_json::to_value(&page.data).unwrap_or_else(|_| json!([]));
    let pagination =
        serde_json::to_value(&page.pagination).unwrap_or_else(|_| json!({}));
    let transactions_path =
        current_company_id.map(|_| "/transactions".to_string());
    let categories_path = current_company_id.map(|_| "/categories".to_string());
    let counterparties_path =
        current_company_id.map(|_| "/counterparties".to_string());

    ctx.respond_to()
        .html(|| {
            ctx.render(
                template,
                json!({
                    "title": title,
                    "app_name": i18n::t("app.name"),
                    "rows": rows,
                    "pagination": pagination,
                    "base_path": base_path,
                    "export_path": export_path,
                    "pagination_labels": pagination_labels(),
                    "labels": table_labels(),
                    "current_company_id": current_company_id,
                    "transactions_path": transactions_path,
                    "categories_path": categories_path,
                    "counterparties_path": counterparties_path,
                    "nav": nav_labels(),
                }),
            )
        })
        .json(|| ctx.json(page))
        .finish()
}

pub fn reports_labels() -> serde_json::Value {
    json!({
        "balance_chart": i18n::t("reports.balance_chart"),
        "expenses_by_category": i18n::t("reports.expenses_by_category"),
        "budget_by_category": i18n::t("reports.budget_by_category"),
        "savings_split": i18n::t("reports.savings_split"),
        "savings_rate": i18n::t("reports.savings_rate"),
        "expense_ratio": i18n::t("reports.expense_ratio"),
        "total_income": i18n::t("reports.total_income"),
        "total_expenses": i18n::t("reports.total_expenses"),
    })
}

pub fn nav_labels() -> serde_json::Value {
    json!({
        "dashboard": i18n::t("nav.dashboard"),
        "accounts": i18n::t("nav.accounts"),
        "transactions": i18n::t("nav.transactions"),
        "categories": i18n::t("nav.categories"),
        "counterparties": i18n::t("nav.counterparties"),
        "new_counterparty": i18n::t("nav.new_counterparty"),
        "company_users": i18n::t("nav.company_users"),
        "new_account": i18n::t("nav.new_account"),
        "new_transaction": i18n::t("nav.new_transaction"),
        "new_category": i18n::t("nav.new_category"),
        "edit": i18n::t("nav.edit"),
        "import_statement": i18n::t("nav.import_statement"),
        "reports": i18n::t("nav.reports"),
        "sign_out": i18n::t("nav.sign_out"),
        "view": i18n::t("nav.view"),
    })
}

pub fn form_labels() -> serde_json::Value {
    let mut labels = table_labels();
    if let Some(obj) = labels.as_object_mut() {
        obj.insert("back".to_string(), json!(i18n::t("forms.back")));
        obj.insert(
            "manage_counterparties".to_string(),
            json!(i18n::t("forms.manage_counterparties")),
        );
        obj.insert(
            "new_counterparty".to_string(),
            json!(i18n::t("forms.add_counterparty")),
        );
    }
    labels
}

pub fn respond_show<T: Serialize>(
    ctx: &Context,
    template: &str,
    title: &str,
    base_path: &str,
    record_id: i64,
    record: &T,
    fields: serde_json::Value,
    current_company_id: Option<i64>,
    editable: bool,
) -> Response {
    ctx.respond_to()
        .html(|| {
            ctx.render(
                template,
                json!({
                    "title": title,
                    "app_name": i18n::t("app.name"),
                    "base_path": base_path,
                    "record_id": record_id,
                    "fields": fields,
                    "labels": table_labels(),
                    "back_label": i18n::t("forms.back"),
                    "nav": nav_labels(),
                    "current_company_id": current_company_id,
                    "editable": editable,
                }),
            )
        })
        .json(|| ctx.json(record))
        .finish()
}

pub fn page_context(
    title: &str,
    current_company_id: Option<i64>,
) -> serde_json::Value {
    let transactions_path =
        current_company_id.map(|_| "/transactions".to_string());
    let categories_path = current_company_id.map(|_| "/categories".to_string());
    let counterparties_path =
        current_company_id.map(|_| "/counterparties".to_string());
    json!({
        "title": title,
        "app_name": i18n::t("app.name"),
        "current_company_id": current_company_id,
        "transactions_path": transactions_path,
        "categories_path": categories_path,
        "counterparties_path": counterparties_path,
        "nav": nav_labels(),
    })
}

pub async fn layout_context(
    ctx: &mut Context,
    title: &str,
) -> doido::Result<serde_json::Value> {
    let current_company_id = tenant::resolve_current_company_id(ctx).await?;
    Ok(page_context(title, current_company_id))
}

pub fn wants_json(ctx: &Context) -> bool {
    matches!(ctx.negotiated_format(), Format::Json)
}

pub fn redirect_unauthenticated(ctx: &Context) -> Response {
    match ctx.negotiated_format() {
        Format::Json => ctx.status(401),
        _ => ctx.redirect_to("/users/sign_in"),
    }
}

pub fn forbidden(ctx: &Context) -> Response {
    match ctx.negotiated_format() {
        Format::Json => ctx.status(403),
        _ => ctx.redirect_to("/"),
    }
}

pub fn collection_path(resource: &str) -> String {
    format!("/{resource}")
}

pub async fn parse_json_or_form<T: DeserializeOwned>(
    ctx: &mut Context,
) -> doido::Result<T> {
    if wants_json(ctx) {
        ctx.body_json().await
    } else {
        ctx.form().await
    }
}

pub fn respond_created<T: Serialize>(
    ctx: &Context,
    redirect_path: &str,
    record: T,
) -> Response {
    if wants_json(ctx) {
        ctx.json(record)
    } else {
        ctx.redirect_to(redirect_path)
    }
}

pub fn respond_updated<T: Serialize>(
    ctx: &Context,
    redirect_path: &str,
    record: T,
) -> Response {
    respond_created(ctx, redirect_path, record)
}

pub fn respond_destroyed(ctx: &Context, redirect_path: &str) -> Response {
    if wants_json(ctx) {
        ctx.status(204)
    } else {
        ctx.redirect_to(redirect_path)
    }
}
