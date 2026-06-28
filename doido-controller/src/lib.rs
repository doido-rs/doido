pub mod config;
pub mod context;
pub mod environment;
pub mod response;
pub mod route_table;
pub mod server;
// Tower middleware stacks and sessions (the former `doido-middleware` crate).
pub mod session;
pub mod stack;

// Re-exported so `routes!`-generated code and application crates can reach axum
// through doido-controller (the former `doido-router` crate lived here).
pub use axum;
pub use config::{Config, ServerConfig, YamlConfig};
pub use context::{Context, IntoActionResponse};
pub use doido_controller_macros::{after_action, before_action, controller, routes};
pub use environment::Environment;
pub use response::Response;
pub use route_table::{all_routes, print_routes, register_routes, RouteEntry};
pub use server::start_server;
pub use session::{CookieSessionStore, Session, SessionStore};
pub use stack::MiddlewareStack;
