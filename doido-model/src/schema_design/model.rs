//! Engine-agnostic schema model for ER diagram export.

use serde::{Deserialize, Serialize};

/// Full database schema as an abstract design model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaDesign {
    pub tables: Vec<TableDesign>,
}

/// One table and its metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableDesign {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub columns: Vec<ColumnDesign>,
    pub primary_key: PrimaryKeyDesign,
    pub indexes: Vec<IndexDesign>,
    pub foreign_keys: Vec<ForeignKeyDesign>,
    pub constraints: Vec<ConstraintDesign>,
}

/// Primary key definition for a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimaryKeyDesign {
    pub columns: Vec<String>,
    pub autoincrement: bool,
}

/// One column in a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDesign {
    pub name: String,
    pub abstract_type: AbstractDataType,
    pub raw_type: String,
    pub nullable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    pub primary_key: bool,
    pub unique: bool,
    pub foreign_key: bool,
}

/// Normalized column type independent of the database engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbstractDataType {
    Integer,
    BigInteger,
    Float,
    Double,
    Decimal,
    Boolean,
    Text,
    String,
    Binary,
    Date,
    Time,
    DateTime,
    Timestamp,
    Json,
    Uuid,
    Enum,
    Array,
    Unknown,
}

/// Foreign key relationship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForeignKeyDesign {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub columns: Vec<String>,
    pub referenced_table: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referenced_schema: Option<String>,
    pub referenced_columns: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<String>,
}

/// Index on a table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexDesign {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub primary: bool,
}

/// Table-level constraint (unique, check, primary key).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstraintDesign {
    pub kind: ConstraintKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub columns: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
}

/// Kind of table constraint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintKind {
    Unique,
    Check,
    PrimaryKey,
}

impl SchemaDesign {
    /// Returns tables sorted by name (deterministic layout).
    pub fn sorted_tables(&self) -> Vec<&TableDesign> {
        let mut tables: Vec<_> = self.tables.iter().collect();
        tables.sort_by(|a, b| a.name.cmp(&b.name));
        tables
    }
}
