use crate::models::enums::{ImportSource, ImportStatementType, Operation};
use chrono::{NaiveDate, NaiveDateTime};
use doido::model::sea_orm::prelude::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRow {
    pub occurred_at: NaiveDateTime,
    pub description: String,
    pub amount: Decimal,
    pub operation: Operation,
    pub category_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl ParseError {
    pub fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

pub fn parse_statement(
    source: ImportSource,
    statement_type: ImportStatementType,
    content: &str,
) -> Result<Vec<ParsedRow>, ParseError> {
    match (source, statement_type) {
        (ImportSource::Nubank, ImportStatementType::CheckingAccount) => {
            super::nubank::parse_checking(content)
        }
        (ImportSource::Nubank, ImportStatementType::CreditCard) => {
            super::nubank::parse_credit_card(content)
        }
        (ImportSource::C6, ImportStatementType::CheckingAccount) => {
            super::c6::parse_checking(content)
        }
        (ImportSource::C6, ImportStatementType::CreditCard) => {
            super::c6::parse_credit_card(content)
        }
    }
}

pub(crate) fn parse_csv_rows(
    content: &str,
) -> Result<(HashMap<String, usize>, Vec<Vec<String>>), ParseError> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(content.as_bytes());

    let headers = reader
        .headers()
        .map_err(|e| ParseError::new(1, format!("invalid CSV header: {e}")))?
        .iter()
        .enumerate()
        .map(|(idx, header)| (normalize_header(header), idx))
        .collect::<HashMap<_, _>>();

    if headers.is_empty() {
        return Err(ParseError::new(1, "CSV header row is empty"));
    }

    let mut rows = Vec::new();
    for (line_idx, record) in reader.records().enumerate() {
        let record = record.map_err(|e| {
            ParseError::new(line_idx + 2, format!("invalid CSV row: {e}"))
        })?;
        rows.push(record.iter().map(|v| v.to_string()).collect());
    }

    Ok((headers, rows))
}

pub(crate) fn column_index(
    headers: &HashMap<String, usize>,
    aliases: &[&str],
) -> Option<usize> {
    aliases
        .iter()
        .map(|alias| normalize_header(alias))
        .find_map(|alias| headers.get(&alias).copied())
}

pub(crate) fn cell(row: &[String], index: usize) -> Option<&str> {
    row.get(index).map(String::as_str)
}

pub(crate) fn parse_date(
    value: &str,
    line: usize,
) -> Result<NaiveDateTime, ParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ParseError::new(line, "date is empty"));
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%d/%m/%Y") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%d-%m-%Y") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap());
    }

    Err(ParseError::new(
        line,
        format!("unsupported date format: {trimmed}"),
    ))
}

pub(crate) fn parse_amount(
    value: &str,
    line: usize,
) -> Result<Decimal, ParseError> {
    let mut trimmed = value.trim().replace('\u{00a0}', "");
    let negative = trimmed.starts_with('-')
        || trimmed.starts_with('(') && trimmed.ends_with(')');
    if trimmed.starts_with('-') {
        trimmed = trimmed.trim_start_matches('-').trim().to_string();
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed = trimmed
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .to_string();
    }

    let cleaned = trimmed
        .replace("R$", "")
        .replace('$', "")
        .trim()
        .to_string();

    let normalized = if cleaned.contains(',') && cleaned.contains('.') {
        cleaned.replace('.', "").replace(',', ".")
    } else if cleaned.contains(',') {
        cleaned.replace(',', ".")
    } else {
        cleaned
    };

    let mut amount = normalized.parse::<Decimal>().map_err(|_| {
        ParseError::new(line, format!("invalid amount: {value}"))
    })?;

    if negative && !amount.is_zero() {
        amount = -amount;
    }

    Ok(amount)
}

pub(crate) fn operation_from_signed_amount(amount: Decimal) -> Operation {
    if amount.is_sign_negative() {
        Operation::Saida
    } else {
        Operation::Entrada
    }
}

pub(crate) fn abs_amount(amount: Decimal) -> Decimal {
    if amount.is_sign_negative() {
        -amount
    } else {
        amount
    }
}

fn normalize_header(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['\u{feff}', '"', '\''], "")
        .replace('ã', "a")
        .replace('á', "a")
        .replace('â', "a")
        .replace('à', "a")
        .replace('é', "e")
        .replace('ê', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ô', "o")
        .replace('ú', "u")
        .replace('ç', "c")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_brazilian_amount_with_currency() {
        let amount = parse_amount("R$ 1.234,56", 1).unwrap();
        assert_eq!(amount.to_string(), "1234.56");
    }

    #[test]
    fn parses_iso_date() {
        let date = parse_date("2026-05-03", 1).unwrap();
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2026-05-03");
    }
}
