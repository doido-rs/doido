use crate::models::bank_account::{
    Column as BankAccountColumn, Entity as BankAccountEntity,
};
use crate::models::category::{
    Column as CategoryColumn, Entity as CategoryEntity,
};
use crate::models::counterparty::{
    Column as CounterpartyColumn, Entity as CounterpartyEntity,
};
use crate::models::enums::{MovementType, Operation};
use crate::models::transaction::{ActiveModel, Column, Entity, Model};
use crate::services::{auth, csv, i18n, associations, listing, pagination, tenant};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct TransactionForm {
    pub bank_account_id: i64,
    pub category_id: i64,
    pub counterparty_id: Option<i64>,
    pub occurred_at: String,
    pub amount: String,
    pub operation: Operation,
    pub movement_type: Option<MovementType>,
}

pub struct TransactionsController;

#[controller]
impl TransactionsController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let page = pagination::from_context(&ctx);
        let query = Entity::find().filter(Column::CompanyId.eq(company_id));
        let response = pagination::fetch(ctx.db(), query, page).await?;
        let rows = associations::transaction_index_rows(ctx.db(), response.data).await?;
        let paginated = pagination::PaginatedResponse {
            data: rows,
            pagination: response.pagination,
        };
        let base_path = listing::collection_path("transactions");
        Ok(listing::respond_index(
            &ctx,
            "transactions/index",
            &i18n::t("resources.transactions"),
            &base_path,
            &format!("{base_path}/export"),
            paginated,
            Some(company_id),
        ))
    }

    pub async fn export(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        let rows = Entity::find()
            .filter(Column::CompanyId.eq(company_id))
            .all(ctx.db())
            .await?;
        Ok(csv::attachment(
            &ctx,
            "transactions.csv",
            transactions_to_csv(rows),
        ))
    }

    pub async fn new(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let user = auth::require_user(ctx).await?;
        let form_data = form_options(ctx.db(), user.id, company_id).await?;
        let base_path = listing::collection_path("transactions");
        Ok(ctx.render(
            "transactions/form",
            form_context(
                &i18n::t("forms.new_transaction"),
                None,
                form_data,
                Some(company_id),
                &base_path,
            ),
        ))
    }

    pub async fn edit(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let user = auth::require_user(ctx).await?;
        let form_data = form_options(ctx.db(), user.id, company_id).await?;
        let base_path = listing::collection_path("transactions");
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(row) if row.company_id == company_id => Ok(ctx.render(
                "transactions/form",
                form_context(
                    &i18n::t("forms.edit_transaction"),
                    Some(row),
                    form_data,
                    Some(company_id),
                    &base_path,
                ),
            )),
            Some(_) => Ok(listing::forbidden(&ctx)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn show(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(row) if row.company_id == company_id => {
                let labels = listing::table_labels();
                let label_map = labels.as_object().cloned().unwrap_or_default();
                let fields = associations::transaction_show_fields(
                    ctx.db(),
                    &row,
                    &label_map,
                )
                .await?;
                let base_path = listing::collection_path("transactions");
                Ok(listing::respond_show(
                    &ctx,
                    "shared/show",
                    &i18n::t("resources.transactions"),
                    &base_path,
                    row.id,
                    &row,
                    serde_json::to_value(fields).unwrap_or(json!([])),
                    Some(company_id),
                    true,
                ))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        let user = auth::require_user(ctx).await?;
        let form: TransactionForm = listing::parse_json_or_form(ctx).await?;
        if !owns_bank_account(ctx.db(), user.id, form.bank_account_id).await? {
            return Ok(ctx.status(403));
        }
        let occurred_at = parse_datetime(&form.occurred_at)?;
        let amount = form
            .amount
            .parse::<Decimal>()
            .map_err(|_| doido::core::anyhow::anyhow!("invalid amount"))?;

        let record = ActiveModel {
            company_id: Set(company_id),
            bank_account_id: Set(form.bank_account_id),
            category_id: Set(form.category_id),
            counterparty_id: Set(form.counterparty_id),
            occurred_at: Set(occurred_at),
            amount: Set(amount),
            operation: Set(form.operation),
            movement_type: Set(form
                .movement_type
                .unwrap_or(MovementType::Balance)),
            ..Default::default()
        };
        let created = record.insert(ctx.db()).await?;
        let redirect = listing::collection_path("transactions");
        Ok(listing::respond_created(&ctx, &redirect, created))
    }

    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        let user = auth::require_user(ctx).await?;
        let form: TransactionForm = listing::parse_json_or_form(ctx).await?;
        if !owns_bank_account(ctx.db(), user.id, form.bank_account_id).await? {
            return Ok(ctx.status(403));
        }
        let occurred_at = parse_datetime(&form.occurred_at)?;
        let amount = form
            .amount
            .parse::<Decimal>()
            .map_err(|_| doido::core::anyhow::anyhow!("invalid amount"))?;

        let id = parse_id(&ctx);
        match Entity::find_by_id(id).one(ctx.db()).await? {
            Some(existing) if existing.company_id == company_id => {
                let mut record: ActiveModel = existing.into();
                record.bank_account_id = Set(form.bank_account_id);
                record.category_id = Set(form.category_id);
                record.counterparty_id = Set(form.counterparty_id);
                record.occurred_at = Set(occurred_at);
                record.amount = Set(amount);
                record.operation = Set(form.operation);
                if let Some(movement_type) = form.movement_type {
                    record.movement_type = Set(movement_type);
                }
                let updated = record.update(ctx.db()).await?;
                let redirect = listing::collection_path("transactions");
                Ok(listing::respond_updated(&ctx, &redirect, updated))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn destroy(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(row) if row.company_id == company_id => {
                Entity::delete_by_id(row.id).exec(ctx.db()).await?;
                let redirect = listing::collection_path("transactions");
                Ok(listing::respond_destroyed(&ctx, &redirect))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }
}

struct FormOptions {
    bank_accounts: Vec<crate::models::bank_account::Model>,
    categories: Vec<crate::models::category::Model>,
    counterparties: Vec<crate::models::counterparty::Model>,
}

async fn form_options(
    db: &DatabaseConnection,
    user_id: i64,
    company_id: i64,
) -> doido::Result<FormOptions> {
    Ok(FormOptions {
        bank_accounts: BankAccountEntity::find()
            .filter(BankAccountColumn::UserId.eq(user_id))
            .all(db)
            .await?,
        categories: CategoryEntity::find()
            .filter(CategoryColumn::CompanyId.eq(company_id))
            .all(db)
            .await?,
        counterparties: CounterpartyEntity::find()
            .filter(CounterpartyColumn::CompanyId.eq(company_id))
            .all(db)
            .await?,
    })
}

fn form_context(
    title: &str,
    transaction: Option<Model>,
    options: FormOptions,
    current_company_id: Option<i64>,
    cancel_path: &str,
) -> serde_json::Value {
    let mut ctx = listing::page_context(title, current_company_id);
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert(
            "transaction".to_string(),
            transaction
                .map(|row| {
                    let mut value =
                        serde_json::to_value(row).unwrap_or(json!({}));
                    if let Some(obj) = value.as_object_mut() {
                        if let Some(occurred) =
                            obj.get("occurred_at").and_then(|v| v.as_str())
                        {
                            obj.insert(
                                "occurred_at".to_string(),
                                json!(format_datetime_local(occurred)),
                            );
                        }
                    }
                    value
                })
                .unwrap_or_else(|| {
                    json!({
                        "id": null,
                        "bank_account_id": null,
                        "category_id": null,
                        "counterparty_id": null,
                        "occurred_at": "",
                        "amount": "",
                        "operation": "SAIDA",
                        "movement_type": "balance"
                    })
                }),
        );
        obj.insert(
            "bank_accounts".to_string(),
            serde_json::to_value(bank_account_select_options(&options.bank_accounts))
                .unwrap_or_else(|_| json!([])),
        );
        obj.insert(
            "categories".to_string(),
            serde_json::to_value(options.categories)
                .unwrap_or_else(|_| json!([])),
        );
        obj.insert(
            "counterparties".to_string(),
            serde_json::to_value(options.counterparties)
                .unwrap_or_else(|_| json!([])),
        );
        obj.insert("cancel_path".to_string(), json!(cancel_path));
        obj.insert("submit_label".to_string(), json!(i18n::t("forms.save")));
        obj.insert("cancel_label".to_string(), json!(i18n::t("forms.cancel")));
        obj.insert("labels".to_string(), listing::form_labels());
    }
    ctx
}

#[derive(Serialize)]
struct BankAccountSelectOption {
    id: i64,
    label: String,
}

fn bank_account_select_options(
    accounts: &[crate::models::bank_account::Model],
) -> Vec<BankAccountSelectOption> {
    accounts
        .iter()
        .map(|account| BankAccountSelectOption {
            id: account.id,
            label: associations::bank_account_label(account),
        })
        .collect()
}

fn parse_datetime(value: &str) -> doido::Result<DateTime> {
    if let Ok(parsed) = DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M") {
        return Ok(parsed);
    }
    if let Ok(parsed) = DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
        return Ok(parsed);
    }
    if let Ok(parsed) = value.parse::<DateTime>() {
        return Ok(parsed);
    }
    Err(doido::core::anyhow::anyhow!("invalid occurred_at"))
}

fn format_datetime_local(value: &str) -> String {
    if value.len() >= 16 && value.as_bytes().get(10) == Some(&b'T') {
        return value[..16].to_string();
    }
    value.to_string()
}

fn transactions_to_csv(rows: Vec<Model>) -> String {
    csv::build_csv(
        &[
            "id",
            "company_id",
            "bank_account_id",
            "category_id",
            "counterparty_id",
            "occurred_at",
            "amount",
            "operation",
            "movement_type",
        ],
        rows.into_iter()
            .map(|row| {
                vec![
                    row.id.to_string(),
                    row.company_id.to_string(),
                    row.bank_account_id.to_string(),
                    row.category_id.to_string(),
                    row.counterparty_id
                        .map(|id| id.to_string())
                        .unwrap_or_default(),
                    row.occurred_at.to_string(),
                    row.amount.to_string(),
                    format!("{:?}", row.operation),
                    format!("{:?}", row.movement_type),
                ]
            })
            .collect(),
    )
}

async fn owns_bank_account(
    db: &DatabaseConnection,
    user_id: i64,
    bank_account_id: i64,
) -> doido::Result<bool> {
    Ok(BankAccountEntity::find_by_id(bank_account_id)
        .one(db)
        .await?
        .map(|account| account.user_id == user_id)
        .unwrap_or(false))
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id")
        .and_then(|v| v.parse().ok())
        .unwrap_or_default()
}
