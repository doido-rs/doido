//! Built-in jobs dashboard templates (overridable via `app/views`).

const VIEWS: &[(&str, &str)] = &[(
    "jobs/dashboard.html.tera",
    include_str!("../../templates/webui/dashboard.html.tera"),
)];

/// Register framework templates before [`doido_view::init`]. Idempotent.
pub fn register_views() {
    for (name, content) in VIEWS {
        doido_view::register_framework_template(name, content);
    }
}
