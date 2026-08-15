use crate::models::counterparty::{ActiveModel, Column, Entity, Model};
use crate::services::{associations, csv, i18n, listing, pagination, tenant};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct CounterpartyForm {
    pub name: String,
}

pub struct CounterpartiesController;

#[controller]
impl CounterpartiesController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let page = pagination::from_context(&ctx);
        let query = Entity::find().filter(Column::CompanyId.eq(company_id));
        let response = pagination::fetch(ctx.db(), query, page).await?;
        let base_path = listing::collection_path("counterparties");
        Ok(listing::respond_index(
            &ctx,
            "counterparties/index",
            &i18n::t("resources.counterparties"),
            &base_path,
            &format!("{base_path}/export"),
            response,
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
            "counterparties.csv",
            counterparties_to_csv(rows),
        ))
    }

    pub async fn new(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let base_path = listing::collection_path("counterparties");
        Ok(ctx.render(
            "counterparties/form",
            form_context(
                &i18n::t("forms.new_counterparty"),
                None,
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
        let base_path = listing::collection_path("counterparties");
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(row) if row.company_id == company_id => Ok(ctx.render(
                "counterparties/form",
                form_context(
                    &i18n::t("forms.edit_counterparty"),
                    Some(row),
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
                let fields =
                    associations::counterparty_show_fields(&row, &label_map);
                let base_path = listing::collection_path("counterparties");
                Ok(listing::respond_show(
                    &ctx,
                    "shared/show",
                    &i18n::t("resources.counterparties"),
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
        let form: CounterpartyForm = listing::parse_json_or_form(ctx).await?;
        let record = ActiveModel {
            company_id: Set(company_id),
            name: Set(form.name),
            ..Default::default()
        };
        let created = record.insert(ctx.db()).await?;
        let redirect = listing::collection_path("counterparties");
        Ok(listing::respond_created(&ctx, &redirect, created))
    }

    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        let form: CounterpartyForm = listing::parse_json_or_form(ctx).await?;
        let base_path = listing::collection_path("counterparties");
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(existing) if existing.company_id == company_id => {
                let mut record: ActiveModel = existing.into();
                record.name = Set(form.name);
                let updated = record.update(ctx.db()).await?;
                Ok(listing::respond_updated(&ctx, &base_path, updated))
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
        let redirect = listing::collection_path("counterparties");
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(row) if row.company_id == company_id => {
                Entity::delete_by_id(row.id).exec(ctx.db()).await?;
                Ok(listing::respond_destroyed(&ctx, &redirect))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }
}

fn form_context(
    title: &str,
    counterparty: Option<Model>,
    current_company_id: Option<i64>,
    cancel_path: &str,
) -> serde_json::Value {
    let mut ctx = listing::page_context(title, current_company_id);
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert(
            "counterparty".to_string(),
            counterparty
                .map(|row| serde_json::to_value(row).unwrap_or(json!({})))
                .unwrap_or_else(|| json!({ "id": null, "name": "" })),
        );
        obj.insert("cancel_path".to_string(), json!(cancel_path));
        obj.insert("submit_label".to_string(), json!(i18n::t("forms.save")));
        obj.insert("cancel_label".to_string(), json!(i18n::t("forms.cancel")));
        obj.insert("labels".to_string(), listing::table_labels());
    }
    ctx
}

fn counterparties_to_csv(rows: Vec<Model>) -> String {
    csv::build_csv(
        &["id", "company_id", "name"],
        rows.into_iter()
            .map(|row| {
                vec![row.id.to_string(), row.company_id.to_string(), row.name]
            })
            .collect(),
    )
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id")
        .and_then(|v| v.parse().ok())
        .unwrap_or_default()
}
