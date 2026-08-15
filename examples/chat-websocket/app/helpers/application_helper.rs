use doido::controller::helper;

/// Application-wide controller helpers. Import in controllers with
/// `use crate::helpers::ApplicationHelper;`.
#[helper]
pub struct ApplicationHelper;

impl ApplicationHelper {
    /// Example helper — replace with your own utilities.
    pub fn greet(name: &str) -> String {
        format!("Hello, {name}!")
    }
}
