use super::parser::{
    abs_amount, cell, column_index, operation_from_signed_amount, parse_amount,
    parse_csv_rows, parse_date, ParseError, ParsedRow,
};
use crate::models::enums::Operation;

const DATE_ALIASES: &[&str] = &["date", "data", "dia"];
const DESCRIPTION_ALIASES: &[&str] =
    &["title", "descricao", "description", "historico"];
const ESTABLISHMENT_ALIASES: &[&str] =
    &["estabelecimento", "merchant", "title"];
const CATEGORY_ALIASES: &[&str] = &["category", "categoria"];
const AMOUNT_ALIASES: &[&str] = &["amount", "valor", "value"];
const CREDIT_ALIASES: &[&str] = &["entrada", "credito", "credit"];
const DEBIT_ALIASES: &[&str] = &["saida", "debito", "debit"];

pub fn parse_checking(content: &str) -> Result<Vec<ParsedRow>, ParseError> {
    let (headers, rows) = parse_csv_rows(content)?;
    let date_idx = column_index(&headers, DATE_ALIASES)
        .ok_or_else(|| ParseError::new(1, "missing date column"))?;
    let description_idx = column_index(&headers, DESCRIPTION_ALIASES)
        .ok_or_else(|| ParseError::new(1, "missing description column"))?;

    let amount_idx = column_index(&headers, AMOUNT_ALIASES);
    let credit_idx = column_index(&headers, CREDIT_ALIASES);
    let debit_idx = column_index(&headers, DEBIT_ALIASES);

    if amount_idx.is_none() && (credit_idx.is_none() || debit_idx.is_none()) {
        return Err(ParseError::new(
            1,
            "missing amount column (Valor or Entrada/Saída)",
        ));
    }

    let mut parsed = Vec::new();
    for (offset, row) in rows.iter().enumerate() {
        let line = offset + 2;
        let occurred_at =
            parse_date(cell(row, date_idx).unwrap_or_default(), line)?;
        let description =
            cell(row, description_idx).unwrap_or("").trim().to_string();
        if description.is_empty() {
            continue;
        }

        let (amount, operation) = if let Some(idx) = amount_idx {
            let signed =
                parse_amount(cell(row, idx).unwrap_or_default(), line)?;
            let operation = operation_from_signed_amount(signed);
            (abs_amount(signed), operation)
        } else {
            let credit = credit_idx
                .and_then(|idx| cell(row, idx))
                .filter(|v| !v.trim().is_empty())
                .map(|v| parse_amount(v, line))
                .transpose()?
                .unwrap_or_else(|| Decimal::ZERO);
            let debit = debit_idx
                .and_then(|idx| cell(row, idx))
                .filter(|v| !v.trim().is_empty())
                .map(|v| parse_amount(v, line))
                .transpose()?
                .unwrap_or_else(|| Decimal::ZERO);

            if credit.is_zero() && debit.is_zero() {
                continue;
            }

            if !credit.is_zero() && !debit.is_zero() {
                return Err(ParseError::new(
                    line,
                    "row has both credit and debit values",
                ));
            }

            if !credit.is_zero() {
                (credit, Operation::Entrada)
            } else {
                (debit, Operation::Saida)
            }
        };

        if amount.is_zero() {
            continue;
        }

        parsed.push(ParsedRow {
            occurred_at,
            description,
            amount,
            operation,
            category_name: None,
        });
    }

    Ok(parsed)
}

pub fn parse_credit_card(content: &str) -> Result<Vec<ParsedRow>, ParseError> {
    let (headers, rows) = parse_csv_rows(content)?;
    let date_idx = column_index(&headers, DATE_ALIASES)
        .ok_or_else(|| ParseError::new(1, "missing date column"))?;
    let description_idx = column_index(&headers, DESCRIPTION_ALIASES)
        .or_else(|| column_index(&headers, ESTABLISHMENT_ALIASES))
        .ok_or_else(|| {
            ParseError::new(1, "missing description/establishment column")
        })?;
    let amount_idx = column_index(&headers, AMOUNT_ALIASES)
        .ok_or_else(|| ParseError::new(1, "missing amount column"))?;
    let category_idx = column_index(&headers, CATEGORY_ALIASES);

    let mut parsed = Vec::new();
    for (offset, row) in rows.iter().enumerate() {
        let line = offset + 2;
        let occurred_at =
            parse_date(cell(row, date_idx).unwrap_or_default(), line)?;
        let description =
            cell(row, description_idx).unwrap_or("").trim().to_string();
        if description.is_empty() {
            continue;
        }

        let signed =
            parse_amount(cell(row, amount_idx).unwrap_or_default(), line)?;
        let amount = abs_amount(signed);
        if amount.is_zero() {
            continue;
        }

        let operation = if signed.is_sign_negative() {
            Operation::Entrada
        } else {
            Operation::Saida
        };

        let category_name = category_idx
            .and_then(|idx| cell(row, idx))
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string);

        parsed.push(ParsedRow {
            occurred_at,
            description,
            amount,
            operation,
            category_name,
        });
    }

    Ok(parsed)
}

use doido::model::sea_orm::prelude::Decimal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nubank_checking_csv() {
        let csv = "\
date,title,amount
2026-05-03,Transferencia recebida,1500.00
2026-05-04,Pagamento boleto,-250.00
";
        let rows = parse_checking(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].operation, Operation::Entrada);
        assert_eq!(rows[1].operation, Operation::Saida);
    }

    #[test]
    fn parses_nubank_credit_card_csv() {
        let csv = "\
date,title,amount,category
2026-05-03,Mercado Pao de Acucar,184.50,supermercado
2026-05-12,Estorno Spotify,-34.90,assinatura
";
        let rows = parse_credit_card(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].operation, Operation::Saida);
        assert_eq!(rows[1].operation, Operation::Entrada);
        assert_eq!(rows[0].category_name.as_deref(), Some("supermercado"));
    }
}
