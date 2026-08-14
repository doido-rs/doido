//! Built-in, overridable auth views (the Devise "views live in the gem" analogue).
//!
//! `doido new --auth` and `auth:install` no longer copy auth controllers/views
//! into the app. The built-in controllers under [`crate::controllers`] render
//! templates like `auth/sign_in`; [`register_views`] makes those templates
//! resolvable out of the box by registering them as *framework templates* in
//! `doido-view`. An app that writes its own `app/views/auth/*.html.tera` (e.g.
//! after `doido generate auth:controllers`) overrides them by name.
//!
//! Call [`register_views`] once at boot, **before** `doido_view::init`.

/// Registers the built-in auth view templates with `doido-view` so the framework
/// controllers can render HTML without the app copying any view files. Idempotent.
pub fn register_views() {
    for (name, content) in VIEWS {
        doido_view::register_framework_template(name, content);
    }
}

/// `(tera template name, source)` pairs for the built-in HTML auth views. Names
/// mirror what the built-in controllers pass to `Context::render` (with the
/// `.html.tera` suffix `doido-view` appends).
const VIEWS: &[(&str, &str)] = &[
    (
        "auth/sign_in.html.tera",
        include_str!("../templates/auth/views/sign_in.html.tera"),
    ),
    (
        "auth/sign_up.html.tera",
        include_str!("../templates/auth/views/sign_up.html.tera"),
    ),
    (
        "auth/password_new.html.tera",
        include_str!("../templates/auth/views/password_new.html.tera"),
    ),
    (
        "auth/password_edit.html.tera",
        include_str!("../templates/auth/views/password_edit.html.tera"),
    ),
    #[cfg(feature = "auth-2fa")]
    (
        "auth/two_factor.html.tera",
        include_str!("../templates/auth/views/two_factor.html.tera"),
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_core_auth_views() {
        register_views();
        register_views(); // idempotent
        let snapshot = doido_view::global::framework_template_snapshot();
        for name in [
            "auth/sign_in.html.tera",
            "auth/sign_up.html.tera",
            "auth/password_new.html.tera",
            "auth/password_edit.html.tera",
        ] {
            assert!(
                snapshot.iter().any(|(n, _)| n == name),
                "expected framework view {name} to be registered"
            );
        }
    }
}
