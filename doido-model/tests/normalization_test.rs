use doido_model::normalization::Normalizer;

#[test]
fn strip_and_downcase_compose() {
    let n = Normalizer::new().strip().downcase();
    assert_eq!(n.apply("  Foo@Bar.COM  "), "foo@bar.com");
}

#[test]
fn squish_collapses_internal_whitespace() {
    let n = Normalizer::new().squish();
    assert_eq!(n.apply("  hello   world  "), "hello world");
}

#[test]
fn upcase_and_custom_steps() {
    let n = Normalizer::new().upcase().custom(|s| s.replace(' ', "_"));
    assert_eq!(n.apply("ab cd"), "AB_CD");
}

#[test]
fn empty_normalizer_is_identity() {
    let n = Normalizer::new();
    assert_eq!(n.apply("Unchanged "), "Unchanged ");
}
