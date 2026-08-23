//! PostgreSQL schema introspection via `sea-schema`.

use std::collections::HashSet;

use doido_core::Result;
use sea_schema::postgres::def::{
    ColumnDefault, ColumnInfo, PrimaryKey, References, TableDef, Unique,
};
use sea_schema::postgres::discovery::SchemaDiscovery;
use sqlx::Postgres;

use crate::schema_design::column_type::map_from_raw_type_name;
use crate::schema_design::model::{
    ColumnDesign, ConstraintDesign, ConstraintKind, ForeignKeyDesign, IndexDesign,
    PrimaryKeyDesign, SchemaDesign, TableDesign,
};

pub async fn introspect(
    url: &str,
    database_schema: Option<&str>,
    ignore_tables: &[String],
) -> Result<SchemaDesign> {
    let schema_name = database_schema.unwrap_or("public");
    let pool = connect_postgres(url, Some(schema_name)).await?;

    let schema = SchemaDiscovery::new(pool, schema_name)
        .discover()
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("postgres schema discovery failed: {e}"))?;

    let ignore: HashSet<&str> = ignore_tables.iter().map(String::as_str).collect();
    let tables = schema
        .tables
        .into_iter()
        .filter(|t| !ignore.contains(t.info.name.as_str()))
        .map(|t| map_table(t, &schema.schema))
        .collect();

    Ok(SchemaDesign { tables })
}

async fn connect_postgres(url: &str, schema: Option<&str>) -> Result<sqlx::Pool<Postgres>> {
    let mut pool_options = sqlx::pool::PoolOptions::<Postgres>::new().max_connections(1);
    if let Some(schema) = schema {
        let sql = format!("SET search_path = '{schema}'");
        pool_options = pool_options.after_connect(move |conn, _| {
            let sql = sql.clone();
            Box::pin(async move {
                sqlx::Executor::execute(conn, sqlx::AssertSqlSafe(sql))
                    .await
                    .map(|_| ())
            })
        });
    }
    pool_options
        .connect(url)
        .await
        .map_err(|e| doido_core::anyhow::anyhow!("postgres connect failed: {e}"))
}

fn map_table(table: TableDef, schema: &str) -> TableDesign {
    let pk = primary_key(&table.primary_key_constraints);
    let pk_columns: HashSet<String> = pk.columns.iter().cloned().collect();

    let unique_columns = unique_column_names(&table.unique_constraints);
    let fk_by_column = fk_columns(&table.reference_constraints);

    let columns: Vec<ColumnDesign> = table
        .columns
        .iter()
        .filter(|c| c.generated.is_none())
        .map(|c| map_column(c, &pk_columns, &unique_columns, &fk_by_column))
        .collect();

    let indexes: Vec<IndexDesign> = table
        .unique_constraints
        .iter()
        .map(|u| IndexDesign {
            name: u.name.clone(),
            columns: u.columns.clone(),
            unique: true,
            primary: false,
        })
        .collect();

    let mut constraints: Vec<ConstraintDesign> = table
        .unique_constraints
        .iter()
        .map(|u| ConstraintDesign {
            kind: ConstraintKind::Unique,
            name: Some(u.name.clone()),
            columns: u.columns.clone(),
            definition: None,
        })
        .collect();

    if !pk.columns.is_empty() {
        constraints.push(ConstraintDesign {
            kind: ConstraintKind::PrimaryKey,
            name: Some(pk.name.clone()),
            columns: pk.columns.clone(),
            definition: None,
        });
    }

    constraints.extend(table.check_constraints.iter().map(|c| ConstraintDesign {
        kind: ConstraintKind::Check,
        name: Some(c.name.clone()),
        columns: Vec::new(),
        definition: Some(c.expr.clone()),
    }));

    let foreign_keys: Vec<ForeignKeyDesign> = table
        .reference_constraints
        .iter()
        .map(map_reference)
        .collect();

    TableDesign {
        name: table.info.name,
        schema: Some(schema.to_string()),
        columns,
        primary_key: PrimaryKeyDesign {
            columns: pk.columns,
            autoincrement: table.columns.iter().any(|c| c.is_identity),
        },
        indexes,
        foreign_keys,
        constraints,
    }
}

fn primary_key(pks: &[PrimaryKey]) -> PrimaryKey {
    pks.first().cloned().unwrap_or(PrimaryKey {
        name: String::new(),
        columns: Vec::new(),
    })
}

fn unique_column_names(uniques: &[Unique]) -> HashSet<String> {
    uniques
        .iter()
        .flat_map(|u| u.columns.iter().cloned())
        .collect()
}

fn fk_columns(refs: &[References]) -> HashSet<String> {
    refs.iter()
        .flat_map(|r| r.columns.iter().cloned())
        .collect()
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
        nullable: col.not_null.is_none(),
        default: map_default(col.default.as_ref()),
        primary_key: pk_columns.contains(&col.name),
        unique: unique_columns.contains(&col.name),
        foreign_key: fk_columns.contains(&col.name),
    }
}

fn map_default(default: Option<&ColumnDefault>) -> Option<String> {
    match default? {
        ColumnDefault::Int(i) => Some(i.to_string()),
        ColumnDefault::Real(f) => Some(f.to_string()),
        ColumnDefault::String(s) => Some(format!("'{s}'")),
        ColumnDefault::Bool(b) => Some(b.to_string()),
        ColumnDefault::CurrentTimestamp => Some("CURRENT_TIMESTAMP".to_string()),
        ColumnDefault::AutoIncrement(s) | ColumnDefault::Expression(s) => Some(s.clone()),
    }
}

fn map_reference(r: &References) -> ForeignKeyDesign {
    ForeignKeyDesign {
        name: Some(r.name.clone()),
        columns: r.columns.clone(),
        referenced_table: r.table.clone(),
        referenced_schema: None,
        referenced_columns: r.foreign_columns.clone(),
        on_delete: r.on_delete.as_ref().map(|a| format!("{a:?}")),
        on_update: r.on_update.as_ref().map(|a| format!("{a:?}")),
    }
}
