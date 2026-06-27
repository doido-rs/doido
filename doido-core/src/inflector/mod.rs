pub mod config;
pub mod inflections;
pub(crate) mod rules;

pub use config::InflectionConfig;
pub use inflections::Inflections;

use std::path::Path;
use std::sync::OnceLock;

static INFLECTIONS: OnceLock<Inflections> = OnceLock::new();

/// Default location, relative to the project root, of the custom inflection file.
pub const DEFAULT_CONFIG_PATH: &str = "config/inflection.yaml";

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

/// Errors raised while loading custom inflections from disk.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to read inflection file `{path}`: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse inflection file `{path}`: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_norway::Error,
    },
}

/// Load custom inflections from a YAML file (default: `config/inflection.yaml`)
/// layered on top of the default English rules, and install them globally.
///
/// A **missing** file is not an error — the default rules are installed and
/// `Ok(false)` is returned. `Ok(true)` means custom rules were found and
/// applied. Returns `Err` only when the file exists but cannot be read/parsed.
///
/// ```no_run
/// // At application boot, from the project root:
/// doido_core::inflector::load_inflections(doido_core::inflector::DEFAULT_CONFIG_PATH).unwrap();
/// ```
pub fn load_inflections(path: impl AsRef<Path>) -> Result<bool, LoadError> {
    let path = path.as_ref();
    let mut base = Inflections::default();

    let found = match std::fs::read_to_string(path) {
        Ok(contents) => {
            let config = InflectionConfig::from_yaml(&contents).map_err(|source| {
                LoadError::Parse {
                    path: path.display().to_string(),
                    source,
                }
            })?;
            config.apply(&mut base);
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(source) => {
            return Err(LoadError::Read {
                path: path.display().to_string(),
                source,
            })
        }
    };

    // Silently ignore if already initialised (e.g. called twice in tests).
    let _ = INFLECTIONS.set(base);
    Ok(found)
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
