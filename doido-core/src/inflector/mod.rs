pub mod inflections;
pub(crate) mod rules;

pub use inflections::Inflections;

use std::sync::OnceLock;

static INFLECTIONS: OnceLock<Inflections> = OnceLock::new();

/// Call this once at application boot, before any `Inflector::*` call.
/// The closure receives the default English rules; add custom overrides there.
///
/// ```rust
/// doido_core::inflector::init_inflections(|i| {
///     i.irregular("goose", "geese");
///     i.uncountable("bitcoin");
/// });
/// ```
pub fn init_inflections<F: FnOnce(&mut Inflections)>(configure: F) {
    let mut base = Inflections::default();
    configure(&mut base);
    // Silently ignore if already initialised (e.g. called twice in tests).
    let _ = INFLECTIONS.set(base);
}

fn global() -> &'static Inflections {
    INFLECTIONS.get_or_init(Inflections::default)
}

/// Static facade over the application-global `Inflections`.
/// All methods delegate to the global instance initialised by `init_inflections`
/// (or default English rules if `init_inflections` was never called).
pub struct Inflector;

impl Inflector {
    pub fn pluralize(s: &str) -> String {
        global().pluralize(s)
    }
    pub fn singularize(s: &str) -> String {
        global().singularize(s)
    }
    pub fn camelize(s: &str) -> String {
        global().camelize(s)
    }
    pub fn camelize_lower(s: &str) -> String {
        global().camelize_lower(s)
    }
    pub fn underscore(s: &str) -> String {
        global().underscore(s)
    }
    pub fn dasherize(s: &str) -> String {
        global().dasherize(s)
    }
    pub fn humanize(s: &str) -> String {
        global().humanize(s)
    }
    pub fn tableize(s: &str) -> String {
        global().tableize(s)
    }
    pub fn classify(s: &str) -> String {
        global().classify(s)
    }
    pub fn foreign_key(s: &str) -> String {
        global().foreign_key(s)
    }
    pub fn constantize(s: &str) -> String {
        global().constantize(s)
    }
}
