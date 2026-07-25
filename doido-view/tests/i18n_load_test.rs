use doido_view::helpers::i18n::I18n;

#[test]
fn load_yaml_flattens_nested_keys() {
    let yaml = "en:\n  hello: \"Hi\"\n  user:\n    greeting: \"Welcome\"\n    count: 3\n";
    let mut i = I18n::new("en");
    i.load_yaml(yaml).unwrap();

    assert_eq!(i.t("hello"), "Hi");
    assert_eq!(i.t("user.greeting"), "Welcome");
    assert_eq!(i.t("user.count"), "3");
    assert_eq!(i.t("missing"), "translation missing: missing");
}
