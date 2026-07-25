use doido_model::scope::Scoped;
use doido_model::sea_orm::{ColumnTrait, DbBackend, EntityTrait, QueryFilter, QueryTrait, Select};

mod widget {
    use doido_model::sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "widgets")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub published: bool,
        pub priority: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// A named scope is just a query transformation.
fn published(q: Select<widget::Entity>) -> Select<widget::Entity> {
    q.filter(widget::Column::Published.eq(true))
}

#[test]
fn named_scopes_compose_into_one_query() {
    let sql = widget::Entity::find()
        .scope(published)
        .scope(|q| q.filter(widget::Column::Priority.gt(5)))
        .build(DbBackend::Sqlite)
        .to_string();
    // Both scopes contribute a WHERE condition, joined with AND.
    assert!(sql.contains("WHERE"), "a scope added a filter: {sql}");
    assert!(sql.contains(" AND "), "both scopes applied: {sql}");
    assert!(sql.contains("> 5"), "priority scope applied: {sql}");
}

#[test]
fn scope_if_applies_conditionally() {
    let with = widget::Entity::find()
        .scope_if(true, published)
        .build(DbBackend::Sqlite)
        .to_string();
    let without = widget::Entity::find()
        .scope_if(false, published)
        .build(DbBackend::Sqlite)
        .to_string();
    assert!(with.contains("WHERE"), "scope_if(true) filtered: {with}");
    assert!(
        !without.contains("WHERE"),
        "scope_if(false) added no filter: {without}"
    );
}
