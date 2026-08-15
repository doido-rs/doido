#[test]
fn en_locale_contains_app_name() {
    let yaml = include_str!("../config/locales/en.yml");
    assert!(yaml.contains("economizei"));
}

#[test]
fn pt_br_locale_contains_app_name() {
    let yaml = include_str!("../config/locales/pt-BR.yml");
    assert!(yaml.contains("economizei"));
}

#[test]
fn en_locale_has_auth_strings() {
    let yaml = include_str!("../config/locales/en.yml");
    assert!(yaml.contains("invalid_credentials"));
}
