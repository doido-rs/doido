use crate::models::bank::{ActiveModel, Column, Entity, Model};
use crate::services::{auth, associations, csv, i18n, listing, pagination, tenant};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct BankForm {
    pub name: String,
    pub code: String,
    pub country_id: i64,
}

pub struct BanksController;

#[controller]
impl BanksController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        if auth::optional_user_id(ctx).is_none() {
            return Ok(listing::redirect_unauthenticated(&ctx));
        }
        let page = pagination::from_context(&ctx);
        let response =
            pagination::fetch(ctx.db(), Entity::find(), page).await?;
        let rows = associations::bank_index_rows(ctx.db(), response.data).await?;
        let paginated = pagination::PaginatedResponse {
            data: rows,
            pagination: response.pagination,
        };
        let current_company_id =
            tenant::resolve_current_company_id(ctx).await?;
        Ok(listing::respond_index(
            &ctx,
            "banks/index",
            &i18n::t("resources.banks"),
            "/banks",
            "/banks/export",
            paginated,
            current_company_id,
        ))
    }

    pub async fn export(ctx: Context) -> doido::Result<Response> {
        let banks = Entity::find().all(ctx.db()).await?;
        Ok(csv::attachment(&ctx, "banks.csv", banks_to_csv(banks)))
    }

    pub async fn show(mut ctx: Context) -> doido::Result<Response> {
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(bank) => {
                let labels = listing::table_labels();
                let label_map = labels.as_object().cloned().unwrap_or_default();
                let fields =
                    associations::bank_show_fields(ctx.db(), &bank, &label_map)
                        .await?;
                let current_company_id =
                    tenant::resolve_current_company_id(ctx).await.ok().flatten();
                Ok(listing::respond_show(
                    &ctx,
                    "shared/show",
                    &i18n::t("resources.banks"),
                    "/banks",
                    bank.id,
                    &bank,
                    serde_json::to_value(fields).unwrap_or(json!([])),
                    current_company_id,
                    false,
                ))
            }
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let form: BankForm = ctx.body_json().await?;
        if Entity::find()
            .filter(Column::Code.eq(&form.code))
            .one(ctx.db())
            .await?
            .is_some()
        {
            return Ok(ctx.status(422));
        }
        let record = ActiveModel {
            name: Set(form.name),
            code: Set(form.code),
            country_id: Set(form.country_id),
            ..Default::default()
        };
        let created = record.insert(ctx.db()).await?;
        Ok(ctx.json(created))
    }

    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let form: BankForm = ctx.body_json().await?;
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(existing) => {
                let mut record: ActiveModel = existing.into();
                record.name = Set(form.name);
                record.code = Set(form.code);
                record.country_id = Set(form.country_id);
                let updated = record.update(ctx.db()).await?;
                Ok(ctx.json(updated))
            }
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn destroy(ctx: Context) -> doido::Result<Response> {
        Entity::delete_by_id(parse_id(&ctx)).exec(ctx.db()).await?;
        Ok(ctx.status(204))
    }
}

fn banks_to_csv(banks: Vec<Model>) -> String {
    csv::build_csv(
        &["id", "name", "code", "country_id"],
        banks
            .into_iter()
            .map(|bank| {
                vec![
                    bank.id.to_string(),
                    bank.name,
                    bank.code,
                    bank.country_id.to_string(),
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
