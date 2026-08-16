use doido_view::helpers::i18n::I18n;
use std::sync::OnceLock;

static I18N: OnceLock<I18n> = OnceLock::new();

pub fn global() -> &'static I18n {
    I18N.get_or_init(|| {
        let mut i18n = I18n::new("en");
        let _ = i18n.load_yaml(include_str!("../../config/locales/en.yml"));
        let _ = i18n.load_yaml(include_str!("../../config/locales/pt-BR.yml"));
        i18n
    })
}

pub fn t(key: &str) -> String {
    global().t(key)
}
