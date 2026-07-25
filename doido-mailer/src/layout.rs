//! Mailer layouts (Rails `layout "mailer"`): wrap a rendered body in a layout.

/// The placeholder a mailer layout uses to mark where the body goes.
pub const YIELD: &str = "{{ yield }}";

/// Wrap `body` inside `layout` at the [`YIELD`] marker (appended if absent).
pub fn apply_layout(layout: &str, body: &str) -> String {
    if layout.contains(YIELD) {
        layout.replace(YIELD, body)
    } else {
        format!("{layout}{body}")
    }
}
