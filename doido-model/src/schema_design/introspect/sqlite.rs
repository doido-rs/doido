//! SQLite schema introspection via `sea-schema`.

use std::collections::HashSet;

use doido_core::Result;
use sea_schema::sqlite::def::{ColumnInfo, ColumnVisibility, DefaultType, ForeignKeyAction, TableDef};
use sea_schema::sqlite::discovery::SchemaDiscovery;
use sqlx::Sqlite;

use crate::schema_design::column_type::map_column_type;
use crate::schema_design::model::{
    ColumnDesign, ConstraintDesign, ConstraintKind, ForeignKeyDesign, IndexDesign,
    PrimaryKeyDesign, SchemaDesign, TableDesign,
};

pub async fn introspect(url: &str, ignore_tables: &[String]) -> Result<SchemaDesign> {
    let pool = sqlx::pool::PoolOptions::<Sqlite>::new()
        .max_connections(1)
        .connect(url)
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("sqlite connect failed: {e}"))?;

    let schema = SchemaDiscovery::new(pool)
        .discover()
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("sqlite schema discovery failed: {e}"))?
        .merge_indexes_into_table();

    let ignore: HashSet<&str> = ignore_tables.iter().map(String::as_str).collect();
    let tables = schema
        .tables
        .into_iter()
        .filter(|t| !ignore.contains(t.name.as_str()))
        .map(map_table)
        .collect();

    Ok(SchemaDesign { tables })
}

fn map_table(table: TableDef) -> TableDesign {
    let fk_columns: HashSet<String> = table
        .foreign_keys
        .iter()
        .flat_map(|fk| fk.from.iter().cloned())
        .collect();

    let pk_columns: Vec<String> = table
        .columns
        .iter()
        .filter(|c| c.primary_key && !sqlite_column_is_generated(c))
        .map(|c| c.name.clone())
        .collect();

    let mut unique_columns: HashSet<String> = table
        .constraints
        .iter()
        .filter(|c| c.unique)
        .flat_map(|c| c.columns.iter().cloned())
        .collect();
    for idx in &table.indexes {
        if idx.unique {
            unique_columns.extend(idx.columns.iter().cloned());
        }
    }

    let columns: Vec<ColumnDesign> = table
        .columns
        .iter()
        .filter(|c| !sqlite_column_is_generated(c))
        .map(|c| {
            let (abstract_type, raw_type) = map_column_type(&c.r#type);
            ColumnDesign {
                name: c.name.clone(),
                abstract_type,
                raw_type,
                nullable: !c.not_null,
                default: map_default(&c.default_value),
                primary_key: c.primary_key,
                unique: unique_columns.contains(&c.name),
                foreign_key: fk_columns.contains(&c.name),
            }
        })
        .collect();

    let mut indexes: Vec<IndexDesign> = table
        .indexes
        .iter()
        .map(|idx| IndexDesign {
            name: idx.index_name.clone(),
            columns: idx.columns.clone(),
            unique: idx.unique,
            primary: false,
        })
        .collect();
    for c in &table.constraints {
        if c.unique && c.origin != "pk" {
            indexes.push(IndexDesign {
                name: c.index_name.clone(),
                columns: c.columns.clone(),
                unique: true,
                primary: false,
            });
        }
    }

    let constraints: Vec<ConstraintDesign> = table
        .constraints
        .iter()
        .map(|c| {
            let kind = if c.origin == "pk" {
                ConstraintKind::PrimaryKey
            } else if c.unique {
                ConstraintKind::Unique
            } else {
                ConstraintKind::Check
            };
            ConstraintDesign {
                kind,
                name: Some(c.index_name.clone()),
                columns: c.columns.clone(),
                definition: None,
            }
        })
        .collect();

    let foreign_keys: Vec<ForeignKeyDesign> = table
        .foreign_keys
        .iter()
        .map(|fk| ForeignKeyDesign {
            name: None,
            columns: fk.from.clone(),
            referenced_table: fk.table.clone(),
            referenced_schema: None,
            referenced_columns: fk.to.clone(),
            on_delete: Some(format_fk_action(&fk.on_delete)),
            on_update: Some(format_fk_action(&fk.on_update)),
        })
        .collect();

    TableDesign {
        name: table.name,
        schema: None,
        columns,
        primary_key: PrimaryKeyDesign {
            columns: pk_columns,
            autoincrement: table.auto_increment,
        },
        indexes,
        foreign_keys,
        constraints,
    }
}

fn sqlite_column_is_generated(col: &ColumnInfo) -> bool {
    matches!(
        col.hidden,
        ColumnVisibility::GeneratedVirtual | ColumnVisibility::GeneratedStored
    )
}

fn map_default(def: &DefaultType) -> Option<String> {
    match def {
        DefaultType::Unspecified | DefaultType::Null => None,
        DefaultType::Integer(i) => Some(i.to_string()),
        DefaultType::Float(f) => Some(f.to_string()),
        DefaultType::String(s) => Some(format!("'{s}'")),
        DefaultType::CurrentTimestamp => Some("CURRENT_TIMESTAMP".to_string()),
    }
}

fn format_fk_action(action: &ForeignKeyAction) -> String {
    match action {
        ForeignKeyAction::NoAction => "NO ACTION".to_string(),
        ForeignKeyAction::Restrict => "RESTRICT".to_string(),
        ForeignKeyAction::SetNull => "SET NULL".to_string(),
        ForeignKeyAction::SetDefault => "SET DEFAULT".to_string(),
        ForeignKeyAction::Cascade => "CASCADE".to_string(),
    }
}
