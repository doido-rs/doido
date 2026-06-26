pub mod context;
pub mod response;

// Re-exported so `routes!`-generated code and application crates can reach axum
// through doido-controller (the former `doido-router` crate lived here).
pub use axum;
pub use context::Context;
pub use doido_controller_macros::{after_action, before_action, controller, routes};
pub use response::Response;
