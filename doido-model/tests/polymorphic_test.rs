use doido_model::association::{sti_type_column, PolymorphicAssociation};

#[test]
fn polymorphic_belongs_to_uses_type_and_id_columns() {
    let p = PolymorphicAssociation::belongs_to("commentable");
    assert_eq!(p.name, "commentable");
    assert_eq!(p.type_column, "commentable_type");
    assert_eq!(p.id_column, "commentable_id");
}

#[test]
fn sti_discriminator_defaults_to_type() {
    assert_eq!(sti_type_column(), "type");
}
