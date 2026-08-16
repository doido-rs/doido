use crate::models::membership::{
    ActiveModel as MembershipActive, Column as MembershipColumn,
    Entity as MembershipEntity, Model as Membership,
};
use crate::models::user::{
    Column as UserColumn, Entity as UserEntity, Model as User,
};
use crate::services::{auth, i18n, listing, pagination, tenant};
use doido::controller::{controller, Context, Response};
use doido::model::sea_orm::{entity::prelude::*, Set};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct MeResponse {
    user: User,
    memberships: Vec<Membership>,
    current_company_id: Option<i64>,
}

#[derive(Serialize)]
struct CompanyUserRow {
    id: i64,
    user_id: i64,
    email: String,
    role: crate::models::enums::MembershipRole,
    salary: Option<Decimal>,
}

pub struct MeController;

#[controller]
impl MeController {
    pub async fn show(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        let memberships = MembershipEntity::find()
            .filter(MembershipColumn::UserId.eq(user.id))
            .all(ctx.db())
            .await?;
        let current_company_id = tenant::current_company_id(ctx);
        Ok(ctx.json(MeResponse {
            user,
            memberships,
            current_company_id,
        }))
    }

    pub async fn company_users(mut ctx: Context) -> doido::Result<Response> {
        let (company_id, _) = match tenant::require_current_company(ctx).await {
            Ok(v) => v,
            Err(_) => return Ok(listing::forbidden(&ctx)),
        };
        let page = pagination::from_context(&ctx);
        let query = MembershipEntity::find()
            .filter(MembershipColumn::CompanyId.eq(company_id));
        let response = pagination::fetch(ctx.db(), query, page).await?;
        let user_ids: Vec<i64> =
            response.data.iter().map(|m| m.user_id).collect();
        let emails = if user_ids.is_empty() {
            HashMap::new()
        } else {
            UserEntity::find()
                .filter(UserColumn::Id.is_in(user_ids))
                .all(ctx.db())
                .await?
                .into_iter()
                .map(|user| (user.id, user.email))
                .collect()
        };
        let rows: Vec<CompanyUserRow> = response
            .data
            .into_iter()
            .map(|membership| CompanyUserRow {
                id: membership.id,
                user_id: membership.user_id,
                email: emails
                    .get(&membership.user_id)
                    .cloned()
                    .unwrap_or_default(),
                role: membership.role,
                salary: membership.salary,
            })
            .collect();
        let paginated = pagination::PaginatedResponse {
            data: rows,
            pagination: response.pagination,
        };
        Ok(listing::respond_index(
            &ctx,
            "me/company_users",
            &i18n::t("resources.company_users"),
            "/members",
            "/members/export",
            paginated,
            Some(company_id),
        ))
    }

    pub async fn update_salary(mut ctx: Context) -> doido::Result<Response> {
        let user = match auth::require_user(ctx).await {
            Ok(user) => user,
            Err(_) => return Ok(ctx.status(401)),
        };
        let membership_id = parse_id(&ctx);
        let body: serde_json::Value = ctx.body_json().await?;
        let salary = body
            .get("salary")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse::<Decimal>().ok())
            .ok_or_else(|| doido::core::anyhow::anyhow!("invalid salary"))?;

        let membership = MembershipEntity::find_by_id(membership_id)
            .one(ctx.db())
            .await?
            .filter(|m| m.user_id == user.id)
            .ok_or_else(|| {
                doido::core::anyhow::anyhow!("membership not found")
            })?;

        let mut record: MembershipActive = membership.into();
        record.salary = Set(Some(salary));
        let updated = record.update(ctx.db()).await?;
        Ok(ctx.json(updated))
    }
}

fn parse_id(ctx: &Context) -> i64 {
    ctx.param("id")
        .and_then(|v| v.parse().ok())
        .unwrap_or_default()
}
