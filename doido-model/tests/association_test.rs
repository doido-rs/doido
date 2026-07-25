use doido_model::association::{join_table, Association, AssociationKind};

#[test]
fn belongs_to_conventions() {
    let a = Association::belongs_to("author");
    assert_eq!(a.kind, AssociationKind::BelongsTo);
    assert_eq!(a.foreign_key, "author_id");
    assert_eq!(a.table, "authors");
}

#[test]
fn has_many_uses_the_owner_foreign_key() {
    let a = Association::has_many("Post", "comments");
    assert_eq!(a.kind, AssociationKind::HasMany);
    assert_eq!(a.foreign_key, "post_id");
    assert_eq!(a.table, "comments");
}

#[test]
fn has_one_uses_the_owner_foreign_key() {
    let a = Association::has_one("User", "profile");
    assert_eq!(a.kind, AssociationKind::HasOne);
    assert_eq!(a.foreign_key, "user_id");
    assert_eq!(a.table, "profiles");
}

#[test]
fn habtm_join_table_is_sorted_and_plural() {
    assert_eq!(join_table("Article", "Tag"), "articles_tags");
    assert_eq!(join_table("Tag", "Article"), "articles_tags");
}
