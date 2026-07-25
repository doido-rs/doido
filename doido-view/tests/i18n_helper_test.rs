use doido_view::helpers::i18n::I18n;

#[test]
fn t_looks_up_and_reports_missing() {
    let mut i = I18n::new("en");
    i.add("greeting", "Hi").add("hello", "Hello, %{name}!");

    assert_eq!(i.locale(), "en");
    assert_eq!(i.t("greeting"), "Hi");
    assert_eq!(i.t("missing"), "translation missing: missing");
}

#[test]
fn t_with_interpolates_variables() {
    let mut i = I18n::new("en");
    i.add("hello", "Hello, %{name} from %{city}!");
    assert_eq!(
        i.t_with("hello", &[("name", "Ada"), ("city", "London")]),
        "Hello, Ada from London!"
    );
}
