//! Engine-agnostic database schema design and ER diagram export.

pub mod column_type;
pub mod export;
pub mod introspect;
pub mod model;

pub use export::{export_html, write_html};
pub use introspect::{introspect_from_url, resolve_ignore_tables, DEFAULT_IGNORE_TABLES};
pub use model::{
    AbstractDataType, ColumnDesign, ConstraintDesign, ConstraintKind, ForeignKeyDesign,
    IndexDesign, PrimaryKeyDesign, SchemaDesign, TableDesign,
};
