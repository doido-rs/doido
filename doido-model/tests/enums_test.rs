use doido_model::enums::AttributeEnum;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Status {
    Active,
    Archived,
}

impl AttributeEnum for Status {
    fn all() -> &'static [Self] {
        &[Status::Active, Status::Archived]
    }
    fn label(&self) -> &'static str {
        match self {
            Status::Active => "active",
            Status::Archived => "archived",
        }
    }
}

#[test]
fn label_maps_each_variant() {
    assert_eq!(Status::Active.label(), "active");
    assert_eq!(Status::Archived.label(), "archived");
}

#[test]
fn from_label_round_trips() {
    assert_eq!(Status::from_label("archived"), Some(Status::Archived));
    assert_eq!(Status::from_label("active"), Some(Status::Active));
    assert_eq!(Status::from_label("nope"), None);
}

#[test]
fn index_and_labels_enumerate_variants() {
    assert_eq!(Status::Active.index(), 0);
    assert_eq!(Status::Archived.index(), 1);
    assert_eq!(Status::labels(), vec!["active", "archived"]);
}
