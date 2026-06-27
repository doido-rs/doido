//! Parsing of `name:type[:modifier...]` field specs passed to the model and
//! scaffold generators (Rails-style), e.g.
//!
//! ```text
//! doido generate model Post title:string body:text published:boolean \
//!     author:references views:integer:index slug:string:unique
//! ```
//!
//! Each parsed [`Field`] knows how to render both a migration column line
//! (`t.string("title").not_null();`) and a SeaORM model field
//! (`pub title: String,`).

use crate::generators::to_snake;
use doido_core::anyhow::{anyhow, bail};
use doido_core::Result;

/// A column type supported by both the migration builder and the SeaORM model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    String,
    Text,
    Integer,
    BigInteger,
    Float,
    Double,
    Decimal,
    Boolean,
    Timestamp,
    Date,
    Json,
    Uuid,
    Binary,
    References,
}

impl ColumnType {
    /// Parse a type token; accepts common aliases.
    fn parse(token: &str) -> Result<Self> {
        Ok(match token.to_lowercase().as_str() {
            "string" => Self::String,
            "text" => Self::Text,
            "integer" | "int" => Self::Integer,
            "bigint" | "biginteger" | "big_integer" | "long" => Self::BigInteger,
            "float" => Self::Float,
            "double" => Self::Double,
            "decimal" | "numeric" => Self::Decimal,
            "boolean" | "bool" => Self::Boolean,
            "timestamp" | "datetime" => Self::Timestamp,
            "date" => Self::Date,
            "json" | "jsonb" => Self::Json,
            "uuid" => Self::Uuid,
            "binary" | "blob" | "bytes" => Self::Binary,
            "references" | "reference" | "belongs_to" => Self::References,
            other => bail!("unknown column type `{other}`"),
        })
    }

    /// The [`TableBuilder`](doido_model::migration) method that adds this column.
    fn builder_method(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Text => "text",
            Self::Integer => "integer",
            Self::BigInteger => "big_integer",
            Self::Float => "float",
            Self::Double => "double",
            Self::Decimal => "decimal",
            Self::Boolean => "boolean",
            Self::Timestamp => "timestamp",
            Self::Date => "date",
            Self::Json => "json",
            Self::Uuid => "uuid",
            Self::Binary => "binary",
            Self::References => "references",
        }
    }

    /// The Rust type used for this column in the SeaORM model struct.
    fn rust_type(self) -> &'static str {
        match self {
            Self::String | Self::Text => "String",
            Self::Integer => "i32",
            Self::BigInteger | Self::References => "i64",
            Self::Float => "f32",
            Self::Double => "f64",
            Self::Decimal => "Decimal",
            Self::Boolean => "bool",
            Self::Timestamp => "DateTime",
            Self::Date => "Date",
            Self::Json => "Json",
            Self::Uuid => "Uuid",
            Self::Binary => "Vec<u8>",
        }
    }
}

/// A single parsed column definition.
#[derive(Debug, Clone)]
pub struct Field {
    /// Name as written by the user (snake-cased), e.g. `author` for a reference.
    raw_name: String,
    ty: ColumnType,
    not_null: bool,
    unique: bool,
    index: bool,
}

impl Field {
    /// Parse one `name:type[:modifier...]` spec. Type defaults to `string`.
    pub fn parse(spec: &str) -> Result<Self> {
        let mut parts = spec.split(':');
        let name = parts
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("empty field spec"))?;

        let ty = match parts.next() {
            Some(t) if !t.is_empty() => ColumnType::parse(t)?,
            _ => ColumnType::String,
        };

        let mut field = Field {
            raw_name: to_snake(name),
            ty,
            not_null: false,
            unique: false,
            index: false,
        };

        for modifier in parts {
            match modifier.to_lowercase().as_str() {
                "" => {}
                "not_null" | "notnull" | "required" => field.not_null = true,
                "unique" | "uniq" => field.unique = true,
                "index" => field.index = true,
                other => bail!("unknown modifier `{other}` in field `{spec}`"),
            }
        }

        Ok(field)
    }

    /// Parse every spec in `specs`, short-circuiting on the first error.
    pub fn parse_all(specs: &[&str]) -> Result<Vec<Field>> {
        specs.iter().map(|s| Field::parse(s)).collect()
    }

    /// The database column name. References get an `_id` suffix.
    pub fn column_name(&self) -> String {
        match self.ty {
            ColumnType::References => format!("{}_id", self.raw_name),
            _ => self.raw_name.clone(),
        }
    }

    /// Whether the column (and therefore the model field) is NOT NULL.
    /// References are always non-null (the builder enforces it).
    fn is_required(&self) -> bool {
        self.not_null || self.ty == ColumnType::References
    }

    /// Whether this field requested its own index.
    pub fn wants_index(&self) -> bool {
        self.index
    }

    /// Render the migration builder line, e.g. `t.string("title").not_null();`.
    /// `references` passes the bare name (it appends `_id` itself).
    pub fn migration_line(&self) -> String {
        let arg = &self.raw_name;
        let mut line = format!("t.{}(\"{arg}\")", self.ty.builder_method());
        // `references` is already NOT NULL; only add it for other types.
        if self.not_null && self.ty != ColumnType::References {
            line.push_str(".not_null()");
        }
        if self.unique {
            line.push_str(".unique_key()");
        }
        line.push(';');
        line
    }

    /// Render the SeaORM model struct field, e.g. `pub title: String,` or
    /// `pub age: Option<i32>,` for a nullable column.
    pub fn model_field(&self) -> String {
        let ty = self.ty.rust_type();
        let rust_ty = if self.is_required() {
            ty.to_string()
        } else {
            format!("Option<{ty}>")
        };
        format!("pub {}: {rust_ty},", self.column_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_string_when_type_omitted() {
        let f = Field::parse("title").unwrap();
        assert_eq!(f.column_name(), "title");
        assert_eq!(f.migration_line(), "t.string(\"title\");");
        assert_eq!(f.model_field(), "pub title: Option<String>,");
    }

    #[test]
    fn maps_types_and_nullability() {
        let f = Field::parse("age:integer:not_null").unwrap();
        assert_eq!(f.migration_line(), "t.integer(\"age\").not_null();");
        assert_eq!(f.model_field(), "pub age: i32,");
    }

    #[test]
    fn unique_modifier_renders_unique_key() {
        let f = Field::parse("email:string:unique").unwrap();
        assert_eq!(f.migration_line(), "t.string(\"email\").unique_key();");
    }

    #[test]
    fn references_get_id_suffix_and_are_non_null() {
        let f = Field::parse("author:references").unwrap();
        assert_eq!(f.column_name(), "author_id");
        assert_eq!(f.migration_line(), "t.references(\"author\");");
        assert_eq!(f.model_field(), "pub author_id: i64,");
    }

    #[test]
    fn index_modifier_is_tracked() {
        let f = Field::parse("slug:string:index").unwrap();
        assert!(f.wants_index());
    }

    #[test]
    fn rejects_unknown_type_and_modifier() {
        assert!(Field::parse("x:notatype").is_err());
        assert!(Field::parse("x:string:notamod").is_err());
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!(
            Field::parse("count:int").unwrap().model_field(),
            "pub count: Option<i32>,"
        );
        assert_eq!(
            Field::parse("active:bool:not_null").unwrap().model_field(),
            "pub active: bool,"
        );
        assert_eq!(
            Field::parse("meta:jsonb").unwrap().model_field(),
            "pub meta: Option<Json>,"
        );
    }
}
