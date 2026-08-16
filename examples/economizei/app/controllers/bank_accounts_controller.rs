use crate::models::bank::Entity as BankEntity;
use crate::models::bank_account::{ActiveModel, Column, Entity, Model};
use crate::models::enums::AccountType;
use crate::services::{auth, associations, csv, i18n, listing, pagination, tenant};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct BankAccountForm {
    pub bank_id: i64,
    pub agency: String,
    pub account_number: String,
    pub cpf_cnpj: String,
    pub account_type: Option<AccountType>,
}

pub struct BankAccountsController;

#[controller]
impl BankAccountsController {
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
            associations::bank_account_index_rows(ctx.db(), response.data).await?;
        let paginated = pagination::PaginatedResponse {
            data: rows,
            pagination: response.pagination,
        };
        Ok(listing::respond_index(
            &ctx,
            "bank_accounts/index",
            &i18n::t("resources.bank_accounts"),
            "/bank_accounts",
            "/bank_accounts/export",
            paginated,
            current_company_id,
        ))
    }

    pub async fn export(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        let accounts = Entity::find()
            .filter(Column::UserId.eq(user.id))
            .all(ctx.db())
            .await?;
        Ok(csv::attachment(
            &ctx,
            "bank_accounts.csv",
            bank_accounts_to_csv(accounts),
        ))
    }

    pub async fn new(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(listing::redirect_unauthenticated(&ctx)),
        };
        let _ = user;
        let banks = BankEntity::find().all(ctx.db()).await?;
        let current_company_id =
            tenant::resolve_current_company_id(ctx).await?;
        Ok(ctx.render(
            "bank_accounts/form",
            form_context(
                &i18n::t("forms.new_bank_account"),
                None,
                banks,
                current_company_id,
                "/bank_accounts",
            ),
        ))
    }

    pub async fn edit(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(listing::redirect_unauthenticated(&ctx)),
        };
        let banks = BankEntity::find().all(ctx.db()).await?;
        let current_company_id =
            tenant::resolve_current_company_id(ctx).await?;
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(account) if account.user_id == user.id => Ok(ctx.render(
                "bank_accounts/form",
                form_context(
                    &i18n::t("forms.edit_bank_account"),
                    Some(account),
                    banks,
                    current_company_id,
                    "/bank_accounts",
                ),
            )),
            Some(_) => Ok(listing::forbidden(&ctx)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn show(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(account) if account.user_id == user.id => {
                let current_company_id =
                    tenant::resolve_current_company_id(ctx).await.ok().flatten();
                let labels = listing::table_labels();
                let label_map = labels.as_object().cloned().unwrap_or_default();
                let fields = associations::bank_account_show_fields(
                    ctx.db(),
                    &account,
                    &label_map,
                )
                .await?;
                Ok(listing::respond_show(
                    &ctx,
                    "shared/show",
                    &i18n::t("resources.bank_accounts"),
                    "/bank_accounts",
                    account.id,
                    &account,
                    serde_json::to_value(fields).unwrap_or(json!([])),
                    current_company_id,
                    true,
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
        let form: BankAccountForm = listing::parse_json_or_form(ctx).await?;
        let record = ActiveModel {
            user_id: Set(user.id),
            bank_id: Set(form.bank_id),
            agency: Set(form.agency),
            account_number: Set(form.account_number),
            cpf_cnpj: Set(form.cpf_cnpj),
            account_type: Set(form
                .account_type
                .unwrap_or(AccountType::Corrente)),
            ..Default::default()
        };
        let created = record.insert(ctx.db()).await?;
        Ok(listing::respond_created(&ctx, "/bank_accounts", created))
    }

    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        let form: BankAccountForm = listing::parse_json_or_form(ctx).await?;
        let id = parse_id(&ctx);
        match Entity::find_by_id(id).one(ctx.db()).await? {
            Some(existing) if existing.user_id == user.id => {
                let mut record: ActiveModel = existing.into();
                record.bank_id = Set(form.bank_id);
                record.agency = Set(form.agency);
                record.account_number = Set(form.account_number);
                record.cpf_cnpj = Set(form.cpf_cnpj);
                if let Some(account_type) = form.account_type {
                    record.account_type = Set(account_type);
                }
                let updated = record.update(ctx.db()).await?;
                Ok(listing::respond_updated(&ctx, "/bank_accounts", updated))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn destroy(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(account) if account.user_id == user.id => {
                Entity::delete_by_id(account.id).exec(ctx.db()).await?;
                Ok(listing::respond_destroyed(&ctx, "/bank_accounts"))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }
}

fn form_context(
    title: &str,
    account: Option<Model>,
    banks: Vec<crate::models::bank::Model>,
    current_company_id: Option<i64>,
    cancel_path: &str,
) -> serde_json::Value {
    let mut ctx = listing::page_context(title, current_company_id);
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert(
            "account".to_string(),
            account
                .map(|a| serde_json::to_value(a).unwrap_or(json!({})))
                .unwrap_or_else(|| {
                    json!({
                        "id": null,
                        "bank_id": null,
                        "agency": "",
                        "account_number": "",
                        "cpf_cnpj": "",
                        "account_type": "corrente"
                    })
                }),
        );
        obj.insert(
            "banks".to_string(),
            serde_json::to_value(banks).unwrap_or_else(|_| json!([])),
        );
        obj.insert("cancel_path".to_string(), json!(cancel_path));
        obj.insert("submit_label".to_string(), json!(i18n::t("forms.save")));
        obj.insert("cancel_label".to_string(), json!(i18n::t("forms.cancel")));
        obj.insert("labels".to_string(), listing::table_labels());
    }
    ctx
}

fn bank_accounts_to_csv(accounts: Vec<Model>) -> String {
    csv::build_csv(
        &[
            "id",
            "user_id",
            "bank_id",
            "agency",
            "account_number",
            "cpf_cnpj",
            "account_type",
        ],
        accounts
            .into_iter()
            .map(|account| {
                vec![
                    account.id.to_string(),
                    account.user_id.to_string(),
                    account.bank_id.to_string(),
                    account.agency,
                    account.account_number,
                    account.cpf_cnpj,
                    format!("{:?}", account.account_type),
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
