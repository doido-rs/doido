use crate::models::bank_account::{
    Column as BankAccountColumn, Entity as BankAccountEntity,
};
use crate::models::bank_statement_import::{Column, Entity, Model};
use crate::models::enums::{ImportSource, ImportStatementType};
use crate::services::{auth, associations, csv, i18n, imports, listing, pagination, tenant};
use base64::Engine;
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::entity::prelude::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct ImportForm {
    pub bank_account_id: i64,
    pub source: ImportSource,
    pub statement_type: ImportStatementType,
    pub original_filename: Option<String>,
    pub csv_content: Option<String>,
    pub content_base64: Option<String>,
}

pub struct BankStatementImportsController;

#[controller]
impl BankStatementImportsController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(listing::redirect_unauthenticated(&ctx)),
        };
        let current_company_id =
            tenant::resolve_current_company_id(ctx).await?;
        let page = pagination::from_context(&ctx);
        let query = Entity::find().filter(Column::UserId.eq(user.id));
        let response = pagination::fetch(ctx.db(), query, page).await?;
        let rows =
            associations::bank_statement_import_index_rows(ctx.db(), response.data)
                .await?;
        let paginated = pagination::PaginatedResponse {
            data: rows,
            pagination: response.pagination,
        };
        Ok(listing::respond_index(
            &ctx,
            "bank_statement_imports/index",
            &i18n::t("resources.bank_statement_imports"),
            "/bank_statement_imports",
            "/bank_statement_imports/export",
            paginated,
            current_company_id,
        ))
    }

    pub async fn export(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        let rows = Entity::find()
            .filter(Column::UserId.eq(user.id))
            .all(ctx.db())
            .await?;
        Ok(csv::attachment(
            &ctx,
            "bank_statement_imports.csv",
            imports_to_csv(rows),
        ))
    }

    pub async fn new(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(listing::redirect_unauthenticated(&ctx)),
        };
        let current_company_id =
            tenant::resolve_current_company_id(ctx).await?;
        let bank_accounts = BankAccountEntity::find()
            .filter(BankAccountColumn::UserId.eq(user.id))
            .all(ctx.db())
            .await?;
        Ok(ctx.render(
            "bank_statement_imports/form",
            form_context(
                &i18n::t("forms.new_bank_statement_import"),
                bank_accounts,
                current_company_id,
            ),
        ))
    }

    pub async fn show(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(record) if record.user_id == user.id => {
                let current_company_id =
                    tenant::resolve_current_company_id(ctx).await.ok().flatten();
                let labels = listing::table_labels();
                let label_map = labels.as_object().cloned().unwrap_or_default();
                let fields = associations::bank_statement_import_show_fields(
                    ctx.db(),
                    &record,
                    &label_map,
                )
                .await?;
                Ok(listing::respond_show(
                    &ctx,
                    "shared/show",
                    &i18n::t("resources.bank_statement_imports"),
                    "/bank_statement_imports",
                    record.id,
                    &record,
                    serde_json::to_value(fields).unwrap_or(json!([])),
                    current_company_id,
                    false,
                ))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        let (company_id, _) = tenant::require_current_company(ctx).await?;
        let form: ImportForm = listing::parse_json_or_form(ctx).await?;
        let file_bytes = decode_file_bytes(&form)?;
        let original_filename = form
            .original_filename
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "import.csv".to_string());

        let request = imports::ImportRequest {
            user_id: user.id,
            company_id,
            bank_account_id: form.bank_account_id,
            source: form.source,
            statement_type: form.statement_type,
            original_filename,
            file_bytes,
        };

        match imports::run(ctx.db(), request).await {
            Ok(result) => Ok(listing::respond_created(
                &ctx,
                "/bank_statement_imports",
                json!({
                    "import": result.import,
                    "transactions_imported": result.transactions_imported,
                }),
            )),
            Err(failure) => {
                let error_message = failure.message();
                if listing::wants_json(&ctx) {
                    Ok(ctx.json(json!({ "error": error_message })))
                } else {
                    let form_data = form_context_with_error(
                        &i18n::t("forms.new_bank_statement_import"),
                        error_message,
                        form.bank_account_id,
                        ctx,
                    )
                    .await?;
                    Ok(ctx.render("bank_statement_imports/form", form_data))
                }
            }
        }
    }
}

fn decode_file_bytes(form: &ImportForm) -> doido::Result<Vec<u8>> {
    if let Some(base64_content) = form
        .content_base64
        .as_ref()
        .filter(|v| !v.trim().is_empty())
    {
        return base64::engine::general_purpose::STANDARD
            .decode(base64_content.trim())
            .map_err(|e| {
                doido::core::anyhow::anyhow!("invalid base64 content: {e}")
            });
    }

    form.csv_content
        .as_ref()
        .filter(|content| !content.trim().is_empty())
        .map(|content| content.as_bytes().to_vec())
        .ok_or_else(|| doido::core::anyhow::anyhow!("csv content is required"))
}

async fn form_context_with_error(
    title: &str,
    error: String,
    bank_account_id: i64,
    ctx: &mut Context,
) -> doido::Result<serde_json::Value> {
    let user = auth::require_user(ctx).await?;
    let current_company_id = tenant::resolve_current_company_id(ctx).await?;
    let bank_accounts = BankAccountEntity::find()
        .filter(BankAccountColumn::UserId.eq(user.id))
        .all(ctx.db())
        .await?;
    let mut value = form_context(title, bank_accounts, current_company_id);
    if let Some(obj) = value.as_object_mut() {
        obj.insert("error".to_string(), json!(error));
        obj.insert("bank_account_id".to_string(), json!(bank_account_id));
    }
    Ok(value)
}

fn form_context(
    title: &str,
    bank_accounts: Vec<crate::models::bank_account::Model>,
    current_company_id: Option<i64>,
) -> serde_json::Value {
    let mut ctx = listing::page_context(title, current_company_id);
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert(
            "bank_accounts".to_string(),
            serde_json::to_value(bank_account_select_options(&bank_accounts))
                .unwrap_or_else(|_| json!([])),
        );
        obj.insert(
            "sources".to_string(),
            json!([
                { "value": "nubank", "label": i18n::t("imports.sources.nubank") },
                { "value": "c6", "label": i18n::t("imports.sources.c6") },
            ]),
        );
        obj.insert(
            "statement_types".to_string(),
            json!([
                { "value": "checking_account", "label": i18n::t("imports.statement_types.checking_account") },
                { "value": "credit_card", "label": i18n::t("imports.statement_types.credit_card") },
            ]),
        );
        obj.insert("cancel_path".to_string(), json!("/bank_statement_imports"));
        obj.insert(
            "submit_label".to_string(),
            json!(i18n::t("imports.submit")),
        );
        obj.insert("cancel_label".to_string(), json!(i18n::t("forms.cancel")));
        obj.insert("labels".to_string(), import_labels());
        obj.insert("error".to_string(), json!(null));
        obj.insert("bank_account_id".to_string(), json!(null));
    }
    ctx
}

fn bank_account_select_options(
    accounts: &[crate::models::bank_account::Model],
) -> Vec<serde_json::Value> {
    accounts
        .iter()
        .map(|account| {
            json!({
                "id": account.id,
                "label": associations::bank_account_label(account),
            })
        })
        .collect()
}

fn import_labels() -> serde_json::Value {
    json!({
        "bank_account_id": i18n::t("tables.bank_account_id"),
        "source": i18n::t("imports.labels.source"),
        "statement_type": i18n::t("imports.labels.statement_type"),
        "original_filename": i18n::t("imports.labels.original_filename"),
        "csv_content": i18n::t("imports.labels.csv_content"),
        "file_checksum": i18n::t("imports.labels.file_checksum"),
        "byte_size": i18n::t("imports.labels.byte_size"),
        "transactions_imported": i18n::t("imports.labels.transactions_imported"),
        "status": i18n::t("imports.labels.status"),
        "created_at": i18n::t("tables.occurred_at"),
    })
}

fn imports_to_csv(rows: Vec<Model>) -> String {
    csv::build_csv(
        &[
            "id",
            "bank_account_id",
            "company_id",
            "source",
            "statement_type",
            "original_filename",
            "file_checksum",
            "byte_size",
            "transactions_imported",
            "status",
            "created_at",
        ],
        rows.into_iter()
            .map(|row| {
                vec![
                    row.id.to_string(),
                    row.bank_account_id.to_string(),
                    row.company_id.to_string(),
                    format!("{:?}", row.source),
                    format!("{:?}", row.statement_type),
                    row.original_filename,
                    row.file_checksum,
                    row.byte_size.to_string(),
                    row.transactions_imported.to_string(),
                    format!("{:?}", row.status),
                    row.created_at.to_string(),
                ]
            })
            .collect(),
    )
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id")
        .and_then(|v| v.parse().ok())
        .unwrap_or_default()
}
