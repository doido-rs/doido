use doido_core::i18n::{
    init_from_env, locale_from_env, normalize_locale, resolve_locale, translate, translate_for,
    DEFAULT_LOCALE, LOCALE_ENV_VAR,
};

#[test]
fn normalize_handles_common_spellings() {
    assert_eq!(normalize_locale("pt_BR"), "pt_BR");
    assert_eq!(normalize_locale("pt-BR"), "pt_BR");
    assert_eq!(normalize_locale("PT_br"), "pt_BR");
    assert_eq!(normalize_locale("pt"), "pt_BR");
    assert_eq!(normalize_locale("en"), "en");
    assert_eq!(normalize_locale("fr"), "en");
}

#[test]
fn translates_known_key_per_locale() {
    assert_eq!(
        translate("auth.invalid_credentials", "en"),
        "Invalid credentials."
    );
    assert_eq!(
        translate("auth.invalid_credentials", "pt_BR"),
        "Credenciais inválidas."
    );
}

#[test]
fn locale_resolution_and_env() {
    std::env::remove_var(LOCALE_ENV_VAR);
    init_from_env();

    assert_eq!(
        translate_for("auth.invalid_credentials", Some("fr")),
        "Invalid credentials."
    );
    assert_eq!(
        translate_for("auth.invalid_credentials", None),
        "Invalid credentials."
    );
    assert_eq!(resolve_locale(None), DEFAULT_LOCALE);

    std::env::set_var(LOCALE_ENV_VAR, "pt_BR");
    assert_eq!(locale_from_env(), Some("pt_BR"));
    assert_eq!(
        translate_for("auth.invalid_credentials", None),
        "Credenciais inválidas."
    );
    assert_eq!(resolve_locale(None), "pt_BR");

    init_from_env();
    assert_eq!(
        translate_for("auth.invalid_credentials", None),
        "Credenciais inválidas."
    );

    std::env::remove_var(LOCALE_ENV_VAR);
    init_from_env();
    assert_eq!(resolve_locale(None), DEFAULT_LOCALE);
}
