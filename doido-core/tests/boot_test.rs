use doido_core::i18n::{init, register_entry, reset_for_test, test_guard, DEFAULT_LOCALE};
use std::path::Path;

#[test]
fn i18n_init_is_idempotent() {
    let _g = test_guard();
    reset_for_test();
    register_entry(DEFAULT_LOCALE, "greeting", "Hi").unwrap();
    init(Path::new("/no/such/locales")).unwrap();
    init(Path::new("/no/such/locales")).unwrap();
}
