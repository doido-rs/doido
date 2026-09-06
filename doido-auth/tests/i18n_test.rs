use doido_auth::register_locales;
use doido_core::i18n::{reset_for_test, test_guard, translate, translate_for, LOCALE_ENV_VAR};

const AUTH_KEYS: &[&str] = &[
    "auth.invalid_credentials",
    "auth.email_taken",
    "auth.password_confirmation_mismatch",
    "auth.email_not_confirmed",
    "auth.account_locked",
    "auth.unauthorized",
    "auth.invalid_token",
    "auth.reset_link_invalid",
    "auth.confirmation_sent",
    "auth.reset_email_sent",
];

#[test]
fn register_locales_loads_all_auth_keys_in_english() {
    let _guard = test_guard();
    reset_for_test();
    register_locales().unwrap();

    for key in AUTH_KEYS {
        let value = translate(key, "en");
        assert!(
            !value.starts_with("translation missing:"),
            "missing english translation for {key}"
        );
    }

    assert_eq!(
        translate("auth.invalid_credentials", "en"),
        "Invalid credentials."
    );
}

#[test]
fn register_locales_loads_portuguese_translations() {
    let _guard = test_guard();
    reset_for_test();
    register_locales().unwrap();

    assert_eq!(
        translate("auth.invalid_credentials", "pt_BR"),
        "Credenciais inválidas."
    );
}

#[test]
fn translate_for_respects_doido_locale_after_register() {
    let _guard = test_guard();
    reset_for_test();
    register_locales().unwrap();

    std::env::set_var(LOCALE_ENV_VAR, "pt_BR");
    assert_eq!(
        translate_for("auth.invalid_credentials", None),
        "Credenciais inválidas."
    );
    std::env::remove_var(LOCALE_ENV_VAR);
}
