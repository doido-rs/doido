use doido_core::inflector::Inflections;

#[test]
fn test_plural_adds_rule() {
    let mut i = Inflections::new();
    i.plural(r"(?i)^(cat)$", "cats");
    // Rule is applied: pluralize works
    assert_eq!(i.pluralize("cat"), "cats");
}

#[test]
fn test_singular_adds_rule() {
    let mut i = Inflections::new();
    i.singular(r"(?i)^(cats)$", "cat");
    assert_eq!(i.singularize("cats"), "cat");
}

#[test]
fn test_irregular_stores_lowercase() {
    let mut i = Inflections::new();
    i.irregular("Person", "People");
    // pluralize and singularize should use the irregular
    assert_eq!(i.pluralize("person"), "people");
    assert_eq!(i.singularize("people"), "person");
}

#[test]
fn test_uncountable_stores_lowercase() {
    let mut i = Inflections::new();
    i.uncountable("Sheep");
    // uncountables are returned unchanged
    assert_eq!(i.pluralize("sheep"), "sheep");
    assert_eq!(i.singularize("sheep"), "sheep");
}

#[test]
fn test_acronym_stores_uppercase() {
    let mut i = Inflections::new();
    i.acronym("api");
    // camelize should uppercase known acronyms
    assert_eq!(i.camelize("api_client"), "APIClient");
}
