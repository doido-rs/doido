pub mod error;
pub mod inflector;
pub mod trace;

// Convenience re-exports so downstream crates depend only on doido-core.
pub use ::anyhow;
pub use ::async_trait::async_trait;
pub use ::serde;
pub use ::thiserror;
pub use ::tracing;

pub use error::{AnyhowContext, Result};
pub use inflector::{
    init_inflections, load_inflections, InflectionConfig, Inflections, Inflector,
};
