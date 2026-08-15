use crate::models::enums::MembershipRole;
use crate::models::membership::{ActiveModel, Column, Entity, Model};
use crate::services::{csv, i18n, listing, pagination, tenant};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MembershipForm {
    pub user_id: i64,
    pub role: Option<MembershipRole>,
    pub salary: Option<Decimal>,
}

pub struct MembershipsController;

#[controller]
impl MembershipsController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let page = pagination::from_context(&ctx);
        let query = Entity::find().filter(Column::CompanyId.eq(company_id));
        let response = pagination::fetch(ctx.db(), query, page).await?;
        let base_path = listing::collection_path("memberships");
        Ok(listing::respond_index(
            &ctx,
            "memberships/index",
            &i18n::t("resources.memberships"),
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
        let memberships = Entity::find()
            .filter(Column::CompanyId.eq(company_id))
            .all(ctx.db())
            .await?;
        Ok(csv::attachment(
            &ctx,
            "memberships.csv",
            memberships_to_csv(memberships),
        ))
    }

    pub async fn show(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_company_scope(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(m) if m.company_id == company_id => Ok(ctx.json(m)),
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, membership) =
            match tenant::require_company_scope(ctx).await {
                Ok(v) => v,
                Err(_) => return Ok(ctx.status(403)),
            };
        if membership.role == MembershipRole::Member {
            return Ok(ctx.status(403));
        }
        let form: MembershipForm = ctx.body_json().await?;
        let record = ActiveModel {
            user_id: Set(form.user_id),
            company_id: Set(company_id),
            role: Set(form.role.unwrap_or(MembershipRole::Member)),
            salary: Set(form.salary),
            ..Default::default()
        };
        let created = record.insert(ctx.db()).await?;
        Ok(ctx.json(created))
    }

    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, actor) = match tenant::require_company_scope(ctx).await
        {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        if actor.role == MembershipRole::Member {
            return Ok(ctx.status(403));
        }
        let form: MembershipForm = ctx.body_json().await?;
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(existing) if existing.company_id == company_id => {
                let mut record: ActiveModel = existing.into();
                if let Some(role) = form.role {
                    record.role = Set(role);
                }
                if form.salary.is_some() {
                    record.salary = Set(form.salary);
                }
                let updated = record.update(ctx.db()).await?;
                Ok(ctx.json(updated))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn destroy(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, actor) = match tenant::require_company_scope(ctx).await
        {
            Ok(v) => v,
            Err(_) => return Ok(ctx.status(403)),
        };
        if actor.role == MembershipRole::Member {
            return Ok(ctx.status(403));
        }
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(m) if m.company_id == company_id => {
                Entity::delete_by_id(m.id).exec(ctx.db()).await?;
                Ok(ctx.status(204))
            }
            Some(_) => Ok(ctx.status(403)),
            None => Ok(ctx.status(404)),
        }
    }
}

fn memberships_to_csv(rows: Vec<Model>) -> String {
    csv::build_csv(
        &["id", "user_id", "company_id", "role", "salary"],
        rows.into_iter()
            .map(|row| {
                vec![
                    row.id.to_string(),
                    row.user_id.to_string(),
                    row.company_id.to_string(),
                    format!("{:?}", row.role),
                    row.salary
                        .map(|salary| salary.to_string())
                        .unwrap_or_default(),
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
