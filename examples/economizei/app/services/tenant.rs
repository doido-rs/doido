use crate::models::membership::{
    Column as MembershipColumn, Entity as MembershipEntity, Model as Membership,
};
use crate::services::auth;
use doido::controller::Context;
use doido::model::sea_orm::entity::prelude::*;

pub const CURRENT_COMPANY_KEY: &str = "current_company_id";

pub fn current_company_id(ctx: &mut Context) -> Option<i64> {
    session_company_id(ctx)
}

pub fn session_company_id(ctx: &mut Context) -> Option<i64> {
    ctx.session()
        .data
        .get(CURRENT_COMPANY_KEY)
        .and_then(|v| v.as_i64())
}

pub async fn resolve_current_company_id(
    ctx: &mut Context,
) -> doido::Result<Option<i64>> {
    if let Some(company_id) = session_company_id(ctx) {
        return Ok(Some(company_id));
    }
    let Some(user_id) = auth::optional_user_id(ctx) else {
        return Ok(None);
    };
    if let Some(membership) = MembershipEntity::find()
        .filter(MembershipColumn::UserId.eq(user_id))
        .one(ctx.db())
        .await?
    {
        set_current_company(ctx, membership.company_id);
        return Ok(Some(membership.company_id));
    }
    Ok(None)
}

pub fn set_current_company(ctx: &mut Context, company_id: i64) {
    ctx.session().set(CURRENT_COMPANY_KEY, company_id);
}

pub async fn require_membership(
    ctx: &mut Context,
    company_id: i64,
) -> doido::Result<Membership> {
    let user = auth::require_user(ctx).await?;
    MembershipEntity::find()
        .filter(MembershipColumn::CompanyId.eq(company_id))
        .filter(MembershipColumn::UserId.eq(user.id))
        .one(ctx.db())
        .await?
        .ok_or_else(|| doido::core::anyhow::anyhow!("membership required"))
}

pub async fn require_current_company(
    ctx: &mut Context,
) -> doido::Result<(i64, Membership)> {
    let company_id =
        resolve_current_company_id(ctx).await?.ok_or_else(|| {
            doido::core::anyhow::anyhow!("current company required")
        })?;
    let membership = require_membership(ctx, company_id).await?;
    Ok((company_id, membership))
}

pub async fn require_company_scope(
    ctx: &mut Context,
) -> doido::Result<(i64, Membership)> {
    require_current_company(ctx).await
}

pub async fn set_default_company(
    ctx: &mut Context,
    user_id: i64,
) -> doido::Result<()> {
    if current_company_id(ctx).is_some() {
        return Ok(());
    }
    if let Some(membership) = MembershipEntity::find()
        .filter(MembershipColumn::UserId.eq(user_id))
        .one(ctx.db())
        .await?
    {
        set_current_company(ctx, membership.company_id);
    }
    Ok(())
}
