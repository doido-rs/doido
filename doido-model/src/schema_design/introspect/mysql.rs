//! MySQL schema introspection via `sea-schema`.

use std::collections::HashSet;

use doido_core::Result;
use sea_schema::mysql::def::{
    ColumnDefault, ColumnInfo, ColumnKey, ForeignKeyInfo, IndexInfo, TableDef,
};
use sea_schema::mysql::discovery::SchemaDiscovery;
use sqlx::MySql;

use crate::schema_design::column_type::map_from_raw_type_name;
use crate::schema_design::model::{
    ColumnDesign, ConstraintDesign, ConstraintKind, ForeignKeyDesign, IndexDesign,
    PrimaryKeyDesign, SchemaDesign, TableDesign,
};

pub async fn introspect(url: &str, ignore_tables: &[String]) -> Result<SchemaDesign> {
    let database_name = database_name_from_url(url)?;
    let pool = sqlx::pool::PoolOptions::<MySql>::new()
        .max_connections(1)
        .connect(url)
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("mysql connect failed: {e}"))?;

    let schema = SchemaDiscovery::new(pool, &database_name)
        .discover()
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("mysql schema discovery failed: {e}"))?;

    let ignore: HashSet<&str> = ignore_tables.iter().map(String::as_str).collect();
    let tables = schema
        .tables
        .into_iter()
        .filter(|t| !ignore.contains(t.info.name.as_str()))
        .map(|t| map_table(t, &schema.schema))
        .collect();

    Ok(SchemaDesign { tables })
}

fn database_name_from_url(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url)
        .map_err(|e| doido_core::anyhow::anyhow!("invalid database url: {e}"))?;
    let name = parsed
        .path_segments()
        .and_then(|mut s| s.next())
        .filter(|n| !n.is_empty())
        .ok_or_else(|| doido_core::anyhow::anyhow!("no database name in url path"))?;
    Ok(name.to_string())
}

fn map_table(table: TableDef, schema: &str) -> TableDesign {
    let pk_columns: HashSet<String> = table
        .columns
        .iter()
        .filter(|c| c.key == ColumnKey::Primary)
        .map(|c| c.name.clone())
        .collect();

    let unique_columns: HashSet<String> = table
        .columns
        .iter()
        .filter(|c| c.key == ColumnKey::Unique)
        .map(|c| c.name.clone())
        .collect();

    let fk_columns: HashSet<String> = table
        .foreign_keys
        .iter()
        .flat_map(|fk| fk.columns.iter().cloned())
        .collect();

    let columns: Vec<ColumnDesign> = table
        .columns
        .iter()
        .filter(|c| !c.extra.generated)
        .map(|c| map_column(c, &pk_columns, &unique_columns, &fk_columns))
        .collect();

    let pk_col_names: Vec<String> = table
        .columns
        .iter()
        .filter(|c| c.key == ColumnKey::Primary)
        .map(|c| c.name.clone())
        .collect();

    let indexes: Vec<IndexDesign> = table.indexes.iter().map(map_index).collect();

    let constraints: Vec<ConstraintDesign> = table
        .indexes
        .iter()
        .filter(|idx| idx.unique || idx.name.eq_ignore_ascii_case("PRIMARY"))
        .map(|idx| ConstraintDesign {
            kind: if idx.name.eq_ignore_ascii_case("PRIMARY") {
                ConstraintKind::PrimaryKey
            } else {
                ConstraintKind::Unique
            },
            name: Some(idx.name.clone()),
            columns: index_columns(idx),
            definition: None,
        })
        .collect();

    let foreign_keys: Vec<ForeignKeyDesign> = table
        .foreign_keys
        .iter()
        .map(map_foreign_key)
        .collect();

    TableDesign {
        name: table.info.name,
        schema: Some(schema.to_string()),
        columns,
        primary_key: PrimaryKeyDesign {
            columns: pk_col_names,
            autoincrement: table.info.auto_increment.is_some(),
        },
        indexes,
        foreign_keys,
        constraints,
    }
}

fn map_column(
    col: &ColumnInfo,
    pk_columns: &HashSet<String>,
    unique_columns: &HashSet<String>,
    fk_columns: &HashSet<String>,
) -> ColumnDesign {
    let raw_type = format!("{:?}", col.col_type);
    let (abstract_type, raw_type) = map_from_raw_type_name(&raw_type);
    ColumnDesign {
        name: col.name.clone(),
        abstract_type,
        raw_type,
        nullable: col.null,
        default: map_default(col.default.as_ref()),
        primary_key: pk_columns.contains(&col.name),
        unique: unique_columns.contains(&col.name),
        foreign_key: fk_columns.contains(&col.name),
    }
}

fn map_default(default: Option<&ColumnDefault>) -> Option<String> {
    match default? {
        ColumnDefault::Null => None,
        ColumnDefault::Int(i) => Some(i.to_string()),
        ColumnDefault::Real(f) => Some(f.to_string()),
        ColumnDefault::String(s) => Some(format!("'{s}'")),
        ColumnDefault::CustomExpr(s) => Some(s.clone()),
        ColumnDefault::CurrentTimestamp => Some("CURRENT_TIMESTAMP".to_string()),
    }
}

fn index_columns(idx: &IndexInfo) -> Vec<String> {
    idx.parts.iter().map(|p| p.column.clone()).collect()
}

fn map_index(idx: &IndexInfo) -> IndexDesign {
    IndexDesign {
        name: idx.name.clone(),
        columns: index_columns(idx),
        unique: idx.unique,
        primary: idx.name.eq_ignore_ascii_case("PRIMARY"),
    }
}

fn map_foreign_key(fk: &ForeignKeyInfo) -> ForeignKeyDesign {
    ForeignKeyDesign {
        name: Some(fk.name.clone()),
        columns: fk.columns.clone(),
        referenced_table: fk.referenced_table.clone(),
        referenced_schema: None,
        referenced_columns: fk.referenced_columns.clone(),
        on_delete: Some(format!("{:?}", fk.on_delete)),
        on_update: Some(format!("{:?}", fk.on_update)),
    }
}
