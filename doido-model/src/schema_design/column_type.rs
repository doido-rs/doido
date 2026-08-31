//! Map engine-specific column types to the abstract model.

use crate::sea_orm::sea_query::ColumnType;

use super::model::AbstractDataType;

/// Returns `(abstract_type, raw_type_name)` for a SeaQuery column type.
pub fn map_column_type(col_type: &ColumnType) -> (AbstractDataType, String) {
    let raw = column_type_name(col_type).to_string();
    let abstract_type = match col_type {
        ColumnType::TinyInteger | ColumnType::SmallInteger | ColumnType::Integer => {
            AbstractDataType::Integer
        }
        ColumnType::BigInteger => AbstractDataType::BigInteger,
        ColumnType::TinyUnsigned
        | ColumnType::SmallUnsigned
        | ColumnType::Unsigned
        | ColumnType::BigUnsigned => AbstractDataType::Integer,
        ColumnType::Float => AbstractDataType::Float,
        ColumnType::Double => AbstractDataType::Double,
        ColumnType::Decimal(_) | ColumnType::Money(_) => AbstractDataType::Decimal,
        ColumnType::Boolean => AbstractDataType::Boolean,
        ColumnType::Text => AbstractDataType::Text,
        ColumnType::Char(_) | ColumnType::String(_) => AbstractDataType::String,
        ColumnType::Binary(_) | ColumnType::VarBinary(_) | ColumnType::Blob => {
            AbstractDataType::Binary
        }
        ColumnType::Date => AbstractDataType::Date,
        ColumnType::Time => AbstractDataType::Time,
        ColumnType::DateTime => AbstractDataType::DateTime,
        ColumnType::Timestamp | ColumnType::TimestampWithTimeZone => AbstractDataType::Timestamp,
        ColumnType::Json | ColumnType::JsonBinary => AbstractDataType::Json,
        ColumnType::Uuid => AbstractDataType::Uuid,
        ColumnType::Enum { .. } => AbstractDataType::Enum,
        ColumnType::Array(_) => AbstractDataType::Array,
        ColumnType::Custom(name) => {
            return (AbstractDataType::Unknown, name.to_string());
        }
        _ => AbstractDataType::Unknown,
    };
    (abstract_type, raw)
}

/// Map a database-specific type name string to the abstract model.
pub fn map_from_raw_type_name(raw: &str) -> (AbstractDataType, String) {
    let lower = raw.to_lowercase();
    let abstract_type = if lower.contains("bigint") || lower.contains("bigserial") {
        AbstractDataType::BigInteger
    } else if lower.contains("int") || lower.contains("serial") {
        AbstractDataType::Integer
    } else if lower.contains("double") || lower.contains("float8") {
        AbstractDataType::Double
    } else if lower.contains("float") || lower.contains("real") {
        AbstractDataType::Float
    } else if lower.contains("decimal") || lower.contains("numeric") || lower.contains("money") {
        AbstractDataType::Decimal
    } else if lower.contains("bool") {
        AbstractDataType::Boolean
    } else if lower.contains("json") {
        AbstractDataType::Json
    } else if lower.contains("uuid") {
        AbstractDataType::Uuid
    } else if lower.contains("timestamp") || lower.contains("datetime") {
        AbstractDataType::Timestamp
    } else if lower.contains("date") && !lower.contains("datetime") {
        AbstractDataType::Date
    } else if lower.contains("time") {
        AbstractDataType::Time
    } else if lower.contains("text") || lower.contains("clob") {
        AbstractDataType::Text
    } else if lower.contains("char") || lower.contains("varchar") {
        AbstractDataType::String
    } else if lower.contains("blob") || lower.contains("binary") || lower.contains("bytea") {
        AbstractDataType::Binary
    } else if lower.contains("enum") {
        AbstractDataType::Enum
    } else if lower.contains("array") {
        AbstractDataType::Array
    } else {
        AbstractDataType::Unknown
    };
    (abstract_type, raw.to_string())
}

fn column_type_name(col_type: &ColumnType) -> &'static str {
    #[allow(unreachable_patterns)]
    match col_type {
        ColumnType::Char(_) => "char",
        ColumnType::String(_) => "varchar",
        ColumnType::Text => "text",
        ColumnType::TinyInteger => "tinyint",
        ColumnType::SmallInteger => "smallint",
        ColumnType::Integer => "int",
        ColumnType::BigInteger => "bigint",
        ColumnType::TinyUnsigned => "tinyint_unsigned",
        ColumnType::SmallUnsigned => "smallint_unsigned",
        ColumnType::Unsigned => "int_unsigned",
        ColumnType::BigUnsigned => "bigint_unsigned",
        ColumnType::Float => "float",
        ColumnType::Double => "double",
        ColumnType::Decimal(_) => "decimal",
        ColumnType::Money(_) => "money",
        ColumnType::DateTime => "datetime",
        ColumnType::Timestamp => "timestamp",
        ColumnType::TimestampWithTimeZone => "timestamptz",
        ColumnType::Time => "time",
        ColumnType::Date => "date",
        ColumnType::Year => "year",
        ColumnType::Binary(_) | ColumnType::VarBinary(_) | ColumnType::Blob => "blob",
        ColumnType::Boolean => "bool",
        ColumnType::Json | ColumnType::JsonBinary => "json",
        ColumnType::Uuid => "uuid",
        ColumnType::Enum { .. } => "enum",
        ColumnType::Array(_) => "array",
        ColumnType::Vector(_) => "vector",
        ColumnType::Bit(_) | ColumnType::VarBit(_) => "bit",
        ColumnType::Cidr => "cidr",
        ColumnType::Inet => "inet",
        ColumnType::MacAddr => "macaddr",
        ColumnType::LTree => "ltree",
        ColumnType::Interval(_, _) => "interval",
        ColumnType::Custom(_) => "custom",
        _ => "unknown",
    }
}
