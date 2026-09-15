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
//! names the variable. This is the *only* way environment variables enter the
//! configuration — there is no implicit `SECTION__KEY` override.
//!
//! Note: this `get_env` (a Tera function used inside config files) is unrelated
//! to [`crate::Environment::get_env`], which selects the current environment
//! *mode* from `DOIDO_ENV`.

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

#[cfg(test)]
mod tests {
    use super::render;

    #[test]
    fn substitutes_env_var_value() {
        // Unique name per test to avoid races with parallel tests mutating env.
        std::env::set_var("DOIDO_CFG_SUBSTITUTE", "postgres://u:p@h/db");
        let out = render("database:\n  url: {{ get_env(name=\"DOIDO_CFG_SUBSTITUTE\") }}\n").unwrap();
        assert_eq!(out, "database:\n  url: postgres://u:p@h/db\n");
        std::env::remove_var("DOIDO_CFG_SUBSTITUTE");
    }

    #[test]
    fn uses_default_when_unset() {
        std::env::remove_var("DOIDO_CFG_DEFAULTED");
        let out = render("server:\n  port: {{ get_env(name=\"DOIDO_CFG_DEFAULTED\", default=\"3000\") }}\n")
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
}
