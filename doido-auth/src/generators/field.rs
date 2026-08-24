//! Parsing of `name:type[:modifier...]` field specs for auth scaffolds.

use super::names::to_snake;
use doido_core::anyhow::{anyhow, bail};
use doido_core::Result;

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

#[derive(Debug, Clone)]
pub struct Field {
    raw_name: String,
    ty: ColumnType,
    not_null: bool,
    unique: bool,
    index: bool,
}

impl Field {
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

    pub fn parse_all(specs: &[&str]) -> Result<Vec<Field>> {
        specs.iter().map(|s| Field::parse(s)).collect()
    }

    pub fn column_name(&self) -> String {
        match self.ty {
            ColumnType::References => format!("{}_id", self.raw_name),
            _ => self.raw_name.clone(),
        }
    }

    pub fn is_required(&self) -> bool {
        self.not_null || self.ty == ColumnType::References
    }

    pub fn wants_index(&self) -> bool {
        self.index
    }

    pub fn is_user_reference(&self) -> bool {
        self.ty == ColumnType::References && self.raw_name == "user"
    }

    fn rust_type(&self) -> String {
        let ty = self.ty.rust_type();
        if self.is_required() {
            ty.to_string()
        } else {
            format!("Option<{ty}>")
        }
    }

    pub fn params_struct_field(&self) -> String {
        let col = self.column_name();
        let ty = self.rust_type();
        if self.ty == ColumnType::Boolean && self.is_required() {
            format!("#[serde(default)]\n    pub {col}: {ty},")
        } else {
            format!("pub {col}: {ty},")
        }
    }

    pub fn active_model_set(&self) -> String {
        let col = self.column_name();
        format!("{col}: Set(form.{col}),")
    }

    pub fn active_model_assign(&self) -> String {
        let col = self.column_name();
        format!("record.{col} = Set(form.{col});")
    }

    pub fn html_form_control(&self, singular: &str) -> String {
        let col = self.column_name();
        match self.html_input_type() {
            "textarea" => format!(
                "  <label for=\"{col}\">{col}<br><textarea id=\"{col}\" name=\"{col}\">{{% if {singular} is defined %}}{{{{ {singular}.{col} | default(value=\"\") }}}}{{% endif %}}</textarea></label>\n"
            ),
            "checkbox" => format!(
                "  <label for=\"{col}\">{col} <input id=\"{col}\" type=\"checkbox\" name=\"{col}\" value=\"true\"{{% if {singular} is defined and {singular}.{col} %}} checked{{% endif %}}></label>\n"
            ),
            input => format!(
                "  <label for=\"{col}\">{col}<br><input id=\"{col}\" type=\"{input}\" name=\"{col}\" value=\"{{% if {singular} is defined %}}{{{{ {singular}.{col} | default(value=\"\") }}}}{{% endif %}}\"></label>\n"
            ),
        }
    }

    pub fn html_input_type(&self) -> &'static str {
        match self.ty {
            ColumnType::Text => "textarea",
            ColumnType::Boolean => "checkbox",
            ColumnType::Integer
            | ColumnType::BigInteger
            | ColumnType::Float
            | ColumnType::Double
            | ColumnType::Decimal
            | ColumnType::References => "number",
            ColumnType::Date => "date",
            ColumnType::Timestamp => "datetime-local",
            _ => "text",
        }
    }

    pub fn migration_line(&self) -> String {
        let arg = &self.raw_name;
        let mut line = format!("t.{}(\"{arg}\")", self.ty.builder_method());
        if self.not_null && self.ty != ColumnType::References {
            line.push_str(".not_null()");
        }
        if self.unique {
            line.push_str(".unique_key()");
        }
        line.push(';');
        line
    }

    pub fn model_field(&self) -> String {
        format!("pub {}: {},", self.column_name(), self.rust_type())
    }
}
