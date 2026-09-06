use doido_core::i18n::{
    available_locales, init_from_env, locale_from_env, normalize_locale, parse_locale_filename,
    register_entry, reset_for_test, resolve_locale, test_guard, translate, translate_for,
    DEFAULT_LOCALE, LOCALE_ENV_VAR,
};

#[test]
fn normalize_handles_common_spellings() {
    assert_eq!(normalize_locale("pt_BR"), "pt_BR");
    assert_eq!(normalize_locale("pt-BR"), "pt_BR");
    assert_eq!(normalize_locale("PT_br"), "pt_BR");
    assert_eq!(normalize_locale("pt"), "pt");
    assert_eq!(normalize_locale("pt-PT"), "pt_PT");
    assert_eq!(normalize_locale("pt_pt"), "pt_PT");
    assert_eq!(normalize_locale("en"), "en");
    assert_eq!(normalize_locale("fr"), "fr");
    assert_eq!(normalize_locale("fr-FR"), "fr_FR");
}

#[test]
fn parse_locale_filename_supports_scoped_and_plain_names() {
    assert_eq!(
        parse_locale_filename("en.yml"),
        Some((None, "en".to_string()))
    );
    assert_eq!(
        parse_locale_filename("pt.yml"),
        Some((None, "pt".to_string()))
    );
    assert_eq!(
        parse_locale_filename("pt_BR.yml"),
        Some((None, "pt_BR".to_string()))
    );
    assert_eq!(
        parse_locale_filename("pt-BR.yml"),
        Some((None, "pt_BR".to_string()))
    );
    assert_eq!(
        parse_locale_filename("auth.en.yml"),
        Some((Some("auth".to_string()), "en".to_string()))
    );
    assert_eq!(
        parse_locale_filename("models.users.pt_BR.yml"),
        Some((Some("models.users".to_string()), "pt_BR".to_string()))
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
    assert!(translate("greeting", "fr").contains("locale not available"));
}

#[test]
fn resolve_locale_requires_catalog_match() {
    let _guard = test_guard();
    reset_for_test();
    register_entry("en", "greeting", "Hello").unwrap();
    register_entry("pt_BR", "greeting", "Oi").unwrap();

    assert_eq!(resolve_locale(None).unwrap(), "en");
    assert_eq!(resolve_locale(Some("pt-BR")).unwrap(), "pt_BR");
    assert!(resolve_locale(Some("fr")).is_err());
    assert!(resolve_locale(Some("pt")).is_err());

    let err = resolve_locale(Some("fr")).unwrap_err().to_string();
    assert!(err.contains("locale not available"));
    assert!(err.contains("available: en, pt_BR"));
}

#[test]
fn translate_for_errors_on_unavailable_locale() {
    let _guard = test_guard();
    reset_for_test();
    register_entry("en", "greeting", "Hello").unwrap();

    let message = translate_for("greeting", Some("fr"));
    assert!(message.contains("locale not available"));
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

    assert_eq!(translate_for("greeting", None), "Hello");
    assert_eq!(resolve_locale(None).unwrap(), DEFAULT_LOCALE);

    std::env::set_var(LOCALE_ENV_VAR, "pt");
    assert_eq!(locale_from_env(), Some("pt".to_string()));
    assert_eq!(translate_for("greeting", None), "Olá");
    assert_eq!(resolve_locale(None).unwrap(), "pt");

    std::env::set_var(LOCALE_ENV_VAR, "pt_BR");
    assert_eq!(locale_from_env(), Some("pt_BR".to_string()));
    assert_eq!(translate_for("greeting", None), "Oi");
    assert_eq!(resolve_locale(None).unwrap(), "pt_BR");

    init_from_env();
    assert_eq!(translate_for("greeting", None), "Oi");

    std::env::remove_var(LOCALE_ENV_VAR);
    init_from_env();
    assert_eq!(resolve_locale(None).unwrap(), DEFAULT_LOCALE);
}

#[test]
fn available_locales_lists_loaded_buckets() {
    let _guard = test_guard();
    reset_for_test();
    register_entry("en", "hello", "Hi").unwrap();
    register_entry("pt_BR", "hello", "Oi").unwrap();

    assert_eq!(
        available_locales(),
        vec!["en".to_string(), "pt_BR".to_string()]
    );
}
