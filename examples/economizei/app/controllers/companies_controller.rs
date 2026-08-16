use crate::models::company::{ActiveModel, Column, Entity, Model};
use crate::models::enums::MembershipRole;
use crate::models::membership::ActiveModel as MembershipActive;
use crate::services::{auth, csv, i18n, listing, pagination, tenant};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CompanyForm {
    pub name: String,
    pub slug: String,
}

pub struct CompaniesController;

#[controller]
impl CompaniesController {
    pub async fn index(mut ctx: Context) -> doido::Result<Response> {
        if auth::require_user(ctx).await.is_err() {
            return Ok(listing::redirect_unauthenticated(&ctx));
        }
        let page = pagination::from_context(&ctx);
        let response =
            pagination::fetch(ctx.db(), Entity::find(), page).await?;
        let current_company_id =
            tenant::resolve_current_company_id(ctx).await?;
        Ok(listing::respond_index(
            &ctx,
            "companies/index",
            &i18n::t("resources.companies"),
            "/companies",
            "/companies/export",
            response,
            current_company_id,
        ))
    }

    pub async fn export(mut ctx: Context) -> doido::Result<Response> {
        if auth::require_user(ctx).await.is_err() {
            return Ok(ctx.status(401));
        }
        let companies = Entity::find().all(ctx.db()).await?;
        Ok(csv::attachment(
            &ctx,
            "companies.csv",
            companies_to_csv(companies),
        ))
    }

    pub async fn show(mut ctx: Context) -> doido::Result<Response> {
        if tenant::require_membership(ctx, parse_id(&ctx))
            .await
            .is_err()
        {
            return Ok(ctx.status(403));
        }
        match Entity::find_by_id(parse_id(&ctx)).one(ctx.db()).await? {
            Some(company) => Ok(ctx.json(company)),
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn create(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        let form: CompanyForm = ctx.body_json().await?;
        if Entity::find()
            .filter(Column::Slug.eq(&form.slug))
            .one(ctx.db())
            .await?
            .is_some()
        {
            return Ok(ctx.status(422));
        }

        let company = ActiveModel {
            name: Set(form.name),
            slug: Set(form.slug),
            ..Default::default()
        }
        .insert(ctx.db())
        .await?;

        MembershipActive {
            user_id: Set(user.id),
            company_id: Set(company.id),
            role: Set(MembershipRole::Owner),
            salary: Set(None),
            ..Default::default()
        }
        .insert(ctx.db())
        .await?;

        tenant::set_current_company(ctx, company.id);
        Ok(ctx.json(company))
    }

    pub async fn update(mut ctx: Context) -> doido::Result<Response> {
        let company_id = parse_id(&ctx);
        if tenant::require_membership(ctx, company_id).await.is_err() {
            return Ok(ctx.status(403));
        }
        let form: CompanyForm = ctx.body_json().await?;
        match Entity::find_by_id(company_id).one(ctx.db()).await? {
            Some(existing) => {
                let mut record: ActiveModel = existing.into();
                record.name = Set(form.name);
                record.slug = Set(form.slug);
                let updated = record.update(ctx.db()).await?;
                Ok(ctx.json(updated))
            }
            None => Ok(ctx.status(404)),
        }
    }

    pub async fn destroy(mut ctx: Context) -> doido::Result<Response> {
        let company_id = parse_id(&ctx);
        let membership = match tenant::require_membership(ctx, company_id).await
        {
            Ok(m) => m,
            Err(_) => return Ok(ctx.status(403)),
        };
        if membership.role != MembershipRole::Owner {
            return Ok(ctx.status(403));
        }
        Entity::delete_by_id(company_id).exec(ctx.db()).await?;
        Ok(ctx.status(204))
    }
}

fn companies_to_csv(companies: Vec<Model>) -> String {
    csv::build_csv(
        &["id", "name", "slug"],
        companies
            .into_iter()
            .map(|company| {
                vec![company.id.to_string(), company.name, company.slug]
            })
            .collect(),
    )
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id")
        .and_then(|v| v.parse().ok())
        .unwrap_or_default()
}
