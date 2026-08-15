use doido::controller::Context;
use doido::model::sea_orm::{
    DatabaseConnection, EntityTrait, PaginatorTrait, Select,
};
use serde::{Deserialize, Serialize};

pub const DEFAULT_PER_PAGE: u64 = 20;
pub const MAX_PER_PAGE: u64 = 100;
pub const ALLOWED_PER_PAGE: [u64; 3] = [20, 50, 100];

#[derive(Debug, Deserialize)]
struct PaginationParams {
    page: Option<u64>,
    per_page: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct Pagination {
    pub page: u64,
    pub per_page: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PaginationMeta {
    pub page: u64,
    pub per_page: u64,
    pub total_count: u64,
    pub total_pages: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: PaginationMeta,
}

pub fn from_context(ctx: &Context) -> Pagination {
    let params: PaginationParams = ctx.params().unwrap_or(PaginationParams {
        page: None,
        per_page: None,
    });
    let page = params.page.unwrap_or(1).max(1);
    let per_page = normalize_per_page(params.per_page);
    Pagination { page, per_page }
}

pub fn normalize_per_page(value: Option<u64>) -> u64 {
    match value {
        Some(size) if ALLOWED_PER_PAGE.contains(&size) => size,
        _ => DEFAULT_PER_PAGE,
    }
}

pub async fn fetch<E>(
    db: &DatabaseConnection,
    query: Select<E>,
    pagination: Pagination,
) -> doido::Result<PaginatedResponse<E::Model>>
where
    E: EntityTrait,
    E::Model: Serialize + Send + Sync + doido::model::sea_orm::FromQueryResult,
{
    let paginator = query.paginate(db, pagination.per_page);
    let total_count = paginator.num_items().await?;
    let total_pages = paginator.num_pages().await?;
    let page_index = pagination.page.saturating_sub(1);
    let data = if total_pages == 0 {
        Vec::new()
    } else {
        let safe_page = page_index.min(total_pages.saturating_sub(1));
        paginator.fetch_page(safe_page).await?
    };

    Ok(PaginatedResponse {
        data,
        pagination: PaginationMeta {
            page: pagination.page,
            per_page: pagination.per_page,
            total_count,
            total_pages,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_per_page_is_20() {
        assert_eq!(normalize_per_page(None), DEFAULT_PER_PAGE);
    }

    #[test]
    fn allowed_per_page_values() {
        for value in ALLOWED_PER_PAGE {
            assert_eq!(normalize_per_page(Some(value)), value);
        }
    }

    #[test]
    fn invalid_per_page_falls_back_to_default() {
        assert_eq!(normalize_per_page(Some(37)), DEFAULT_PER_PAGE);
    }

    #[test]
    fn per_page_cannot_exceed_maximum() {
        assert_eq!(normalize_per_page(Some(250)), DEFAULT_PER_PAGE);
    }
}
