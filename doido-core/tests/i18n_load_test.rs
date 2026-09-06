use std::fs;
use std::path::Path;

use doido_core::i18n::{load_locale_dir, register_yaml, reset_for_test, test_guard, translate};
use tempfile::tempdir;

#[test]
fn load_locale_dir_merges_plain_and_scoped_files() {
    let _guard = test_guard();
    reset_for_test();
    let dir = tempdir().unwrap();
    let locales = dir.path().join("locales");
    fs::create_dir(&locales).unwrap();

    fs::write(
        locales.join("en.yml"),
        "en:\n  hello: Hello\n  app:\n    title: App\n",
    )
    .unwrap();
    fs::write(
        locales.join("auth.en.yml"),
        "_version: 1\ninvalid_credentials: Invalid\n",
    )
    .unwrap();

    load_locale_dir(&locales).unwrap();

    assert_eq!(translate("hello", "en"), "Hello");
    assert_eq!(translate("app.title", "en"), "App");
    assert_eq!(translate("auth.invalid_credentials", "en"), "Invalid");
}

#[test]
fn register_yaml_later_entries_override_earlier() {
    let _guard = test_guard();
    reset_for_test();

    register_yaml("en", "invalid_credentials: Framework\n", Some("auth")).unwrap();
    assert_eq!(translate("auth.invalid_credentials", "en"), "Framework");

    register_yaml("en", "invalid_credentials: Override\n", Some("auth")).unwrap();
    assert_eq!(translate("auth.invalid_credentials", "en"), "Override");
}

#[test]
fn missing_locale_dir_is_noop() {
    let _guard = test_guard();
    reset_for_test();
    load_locale_dir(Path::new("definitely/missing/locales")).unwrap();
}
