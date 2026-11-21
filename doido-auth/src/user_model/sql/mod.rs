//! Dialect-specific SQL for composable user fetches.

mod actions;
mod backend;
pub mod mysql;
mod patch;
pub mod postgres;
pub mod sqlite;

pub use actions::apply as sql_apply_action;
pub use backend::detect_backend;
pub use backend::fetch_by_email as sql_fetch_by_email;
pub use backend::fetch_by_field as sql_fetch_by_field;
pub use backend::fetch_by_id as sql_fetch_by_id;
