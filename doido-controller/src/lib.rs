pub mod config;
pub mod constraints;
pub mod context;
pub mod cookies;
pub mod csrf;
pub mod env_override;
pub mod flash;
pub mod health;
pub mod initializers;
pub mod logging;
pub mod params;
pub mod rate_limit;
pub mod rescue;
pub mod respond;
pub mod response;
pub mod route_table;
pub mod secret;
pub mod server;
pub mod signing;
// Tower middleware stacks and sessions (the former `doido-middleware` crate).
pub mod session;
pub mod stack;
pub mod testing;

// Re-exported so `routes!`-generated code and application crates reach axum through
// `doido_controller::axum` (generated apps: `doido::controller::axum`). Do not depend
// on the `axum` crate directly outside this module.
pub use axum;
pub use config::{Config, LoggerConfig, ServerConfig, YamlConfig};
pub use context::{Context, IntoActionResponse};
pub use cookies::CookieJar;
// `before_action`/`after_action` are helper attributes consumed by `#[controller]`,
// not standalone macros, so only `controller` and `routes` are re-exported.
pub use doido_controller_macros::{controller, routes};
pub use flash::Flash;
pub use params::Params;
pub use rescue::RescueHandlers;
pub use response::Response;
pub use route_table::{all_routes, print_routes, register_routes, RouteEntry};
pub use server::{start_server, start_server_with};
pub use session::{
    CacheSessionStore, CookieSessionStore, EncryptedCookieSessionStore, Session, SessionStore,
};
pub use stack::MiddlewareStack;
