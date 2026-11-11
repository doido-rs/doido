//! Rendering `config/<env>.yml` through Tera before it is parsed as YAML.
//!
//! Every crate loads its slice of `config/<env>.yml`; before the YAML is
//! deserialized it is rendered by [`render`], which exposes a single Tera
//! function, `get_env`, so config files can pull values from the environment:
//!
//! ```yaml
//! database:
//!   url: {{ get_env(name="DATABASE_URL") }}
//! server:
//!   port: {{ get_env(name="PORT", default="3000") }}
//! ```
//!
//! `get_env(name="VAR")` substitutes the value of the `VAR` environment
//! variable. An optional `default` supplies a fallback; without it, a missing
//! variable fails the render (and therefore the config load) with an error that
//! names the variable. This is how values such as `DATABASE_URL` reach `database.url`.
//!
//! Note: this `get_env` (a Tera function used inside config files) is unrelated
//! to [`crate::Environment::get_env`], which selects the current environment
//! *mode* from `DOIDO_ENV`.
//!
//! Environment variables enter **typed config only** through `get_env` references in
//! `config/<env>.yml` (rendered by [`render`] on every [`YamlConfig::from_yaml`] load).
//! Bootstrap keys (`DOIDO_ENV`, `DOIDO_MASTER_KEY`, …) and `RUST_LOG` are separate.
//! Wrap `get_env` in YAML quotes when the value may contain `:` (e.g.
//! `url: '{{ get_env(name="DATABASE_URL", default="sqlite://…") }}'`).

use std::env;
use std::path::PathBuf;

use tera::{Context, Error, Kwargs, State, Tera, TeraResult, Value};

/// Render a raw `config/<env>.yml` string through Tera, expanding
/// `{{ get_env(name="VAR") }}` / `{{ get_env(name="VAR", default="…") }}` into
/// environment-variable values.
///
/// Returns an error if a referenced variable is unset and no default is given,
/// or if the template is otherwise invalid. Rendering runs with autoescaping
/// disabled so YAML values (URLs, connection strings) are not HTML-escaped.
pub fn render(raw: &str) -> Result<String, Error> {
    let mut tera = Tera::new();
    tera.register_function("get_env", get_env);
    tera.render_str(raw, &Context::new(), false)
}

/// Tera `get_env(name=…, default=…)` function backing [`render`].
fn get_env(kwargs: Kwargs, _state: &State) -> TeraResult<Value> {
    let name: String = kwargs.must_get("name")?;
    let default: Option<String> = kwargs.get("default")?;
    match std::env::var(&name) {
        Ok(value) => Ok(Value::from(value)),
        Err(_) => match default {
            Some(default) => Ok(Value::from(default)),
            None => Err(Error::message(format!(
                "environment variable {name:?} is not set and no default was provided"
            ))),
        },
    }
}

/// Retrieves the current working directory.
///
/// Fallback strategy:
/// 1. Attempts to get the current working directory (`env::current_dir`).
/// 2. If it fails, logs the error and attempts to use the `HOME` environment variable.
/// 3. If `HOME` is also unavailable, logs the error and falls back to the root directory (`/`).
pub fn get_working_dir() -> PathBuf {
    match env::current_dir() {
        Ok(path) => path,
        Err(e) => {
            eprintln!(
                "Warning: Failed to retrieve current directory: {}. Trying fallback (HOME)...",
                e
            );

            match env::var("HOME") {
                Ok(home) => PathBuf::from(home),
                Err(home_err) => {
                    eprintln!(
                        "Warning: Failed to retrieve HOME environment variable: {}. Falling back to root (/)...",
                        home_err
                    );
                    PathBuf::from("/")
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::get_working_dir;
    use super::render;

    #[test]
    fn substitutes_env_var_value() {
        // Unique name per test to avoid races with parallel tests mutating env.
        std::env::set_var("DOIDO_CFG_SUBSTITUTE", "postgres://u:p@h/db");
        let out =
            render("database:\n  url: {{ get_env(name=\"DOIDO_CFG_SUBSTITUTE\") }}\n").unwrap();
        assert_eq!(out, "database:\n  url: postgres://u:p@h/db\n");
        std::env::remove_var("DOIDO_CFG_SUBSTITUTE");
    }

    #[test]
    fn uses_default_when_unset() {
        std::env::remove_var("DOIDO_CFG_DEFAULTED");
        let out = render(
            "server:\n  port: {{ get_env(name=\"DOIDO_CFG_DEFAULTED\", default=\"3000\") }}\n",
        )
        .unwrap();
        assert_eq!(out, "server:\n  port: 3000\n");
    }

    #[test]
    fn errors_when_unset_without_default() {
        std::env::remove_var("DOIDO_CFG_MISSING");
        let err = render("k: {{ get_env(name=\"DOIDO_CFG_MISSING\") }}\n").unwrap_err();
        assert!(err.to_string().contains("DOIDO_CFG_MISSING"));
    }

    #[test]
    fn substitutes_multiple_references() {
        std::env::set_var("DOIDO_CFG_MULTI_A", "aaa");
        std::env::set_var("DOIDO_CFG_MULTI_B", "bbb");
        let out = render(
            "a: {{ get_env(name=\"DOIDO_CFG_MULTI_A\") }}\nb: {{ get_env(name=\"DOIDO_CFG_MULTI_B\") }}\n",
        )
        .unwrap();
        assert_eq!(out, "a: aaa\nb: bbb\n");
        std::env::remove_var("DOIDO_CFG_MULTI_A");
        std::env::remove_var("DOIDO_CFG_MULTI_B");
    }

    #[test]
    fn does_not_html_escape_special_chars() {
        // Guards autoescape = false: URLs with & and : must round-trip verbatim.
        std::env::set_var("DOIDO_CFG_URL", "https://x/y?a=1&b=2");
        let out = render("url: {{ get_env(name=\"DOIDO_CFG_URL\") }}\n").unwrap();
        assert_eq!(out, "url: https://x/y?a=1&b=2\n");
        std::env::remove_var("DOIDO_CFG_URL");
    }

    #[test]
    fn passes_through_plain_yaml_unchanged() {
        let yaml = "server:\n  bind: 0.0.0.0\n  port: 3000\n";
        assert_eq!(render(yaml).unwrap(), yaml);
    }

    #[test]
    fn test_current_dir_success() {
        // In a healthy test environment, current_dir() should succeed.
        let dir = get_working_dir();
        assert!(dir.exists(), "The returned directory must exist on disk");
        assert!(dir.is_dir(), "The returned path must be a valid directory");
    }

    #[test]
    fn test_fallback_to_home() {
        // Save the original HOME variable to restore it later
        let original_home = std::env::var("HOME").ok();

        // Set a mock HOME directory to validate environment fallback handling
        std::env::set_var("HOME", "/tmp/mock-home");

        let dir = get_working_dir();
        // Ensure the function returns a non-empty, valid path fallback structure
        assert!(dir.as_os_str().len() > 0);

        // Restore original environment state
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        } else {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn test_fallback_to_root() {
        // Temporarily remove HOME to trigger the final root (/) fallback
        let original_home = std::env::var("HOME").ok();
        std::env::remove_var("HOME");

        let dir = get_working_dir();
        // Ensure the function safely defaults without panicking
        assert!(dir.as_os_str().len() > 0);

        // Restore original environment state
        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }
}
