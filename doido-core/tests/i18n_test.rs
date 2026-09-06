use doido_core::i18n::{
    init_from_env, locale_from_env, normalize_locale, parse_locale_filename, register_entry,
    reset_for_test, resolve_locale, test_guard, translate, translate_for, DEFAULT_LOCALE,
    LOCALE_ENV_VAR,
};

#[test]
fn normalize_handles_common_spellings() {
    assert_eq!(normalize_locale("pt_BR"), "pt_BR");
    assert_eq!(normalize_locale("pt-BR"), "pt_BR");
    assert_eq!(normalize_locale("PT_br"), "pt_BR");
    assert_eq!(normalize_locale("pt"), "pt");
    assert_eq!(normalize_locale("en"), "en");
    assert_eq!(normalize_locale("fr"), "en");
    assert_eq!(normalize_locale("pt-PT"), "en");
    assert_eq!(normalize_locale("pt_pt"), "en");
}

#[test]
fn parse_locale_filename_supports_scoped_and_plain_names() {
    assert_eq!(parse_locale_filename("en.yml"), Some((None, "en")));
    assert_eq!(parse_locale_filename("pt.yml"), Some((None, "pt")));
    assert_eq!(parse_locale_filename("pt_BR.yml"), Some((None, "pt_BR")));
    assert_eq!(parse_locale_filename("pt-BR.yml"), Some((None, "pt_BR")));
    assert_eq!(
        parse_locale_filename("auth.en.yml"),
        Some((Some("auth".to_string()), "en"))
    );
    assert_eq!(
        parse_locale_filename("models.users.pt_BR.yml"),
        Some((Some("models.users".to_string()), "pt_BR"))
    );
    assert_eq!(parse_locale_filename("readme.md"), None);
}

#[test]
fn translates_registered_key_with_fallback() {
    let _guard = test_guard();
    reset_for_test();
    register_entry("en", "greeting", "Hello").unwrap();
    register_entry("pt", "greeting", "Olá").unwrap();
    register_entry("pt_BR", "greeting", "Oi").unwrap();

    assert_eq!(translate("greeting", "en"), "Hello");
    assert_eq!(translate("greeting", "pt"), "Olá");
    assert_eq!(translate("greeting", "pt_BR"), "Oi");
    assert_eq!(translate("missing", "en"), "translation missing: missing");
    assert_eq!(translate("missing", "pt"), "translation missing: missing");
    assert_eq!(
        translate("missing", "pt_BR"),
        "translation missing: missing"
    );
}

#[test]
fn locale_resolution_and_env() {
    let _guard = test_guard();
    reset_for_test();
    register_entry("en", "greeting", "Hello").unwrap();
    register_entry("pt", "greeting", "Olá").unwrap();
    register_entry("pt_BR", "greeting", "Oi").unwrap();

    std::env::remove_var(LOCALE_ENV_VAR);
    init_from_env();

    assert_eq!(translate_for("greeting", Some("fr")), "Hello");
    assert_eq!(translate_for("greeting", None), "Hello");
    assert_eq!(resolve_locale(None), DEFAULT_LOCALE);

    std::env::set_var(LOCALE_ENV_VAR, "pt");
    assert_eq!(locale_from_env(), Some("pt"));
    assert_eq!(translate_for("greeting", None), "Olá");
    assert_eq!(resolve_locale(None), "pt");

    std::env::set_var(LOCALE_ENV_VAR, "pt_BR");
    assert_eq!(locale_from_env(), Some("pt_BR"));
    assert_eq!(translate_for("greeting", None), "Oi");
    assert_eq!(resolve_locale(None), "pt_BR");

    init_from_env();
    assert_eq!(translate_for("greeting", None), "Oi");

    std::env::remove_var(LOCALE_ENV_VAR);
    init_from_env();
    assert_eq!(resolve_locale(None), DEFAULT_LOCALE);
}
