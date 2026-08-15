use crate::models::bank::{
    ActiveModel as BankActiveModel, Column as BankColumn, Entity as BankEntity,
};
use crate::models::country::{
    ActiveModel as CountryActiveModel, Column as CountryColumn,
    Entity as CountryEntity,
};
use doido::model::sea_orm::{
    entity::prelude::*, ActiveModelTrait, DatabaseConnection, Set,
};
use encoding_rs::WINDOWS_1252;
use regex::Regex;
use std::collections::HashSet;

pub const BCB_AUTHORIZED_INSTITUTIONS_URL: &str =
    "https://www.bcb.gov.br/Rex/CCR/instituicoes_autorizadas_ccr.asp?frame=1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BcbInstitution {
    pub code: String,
    pub name: String,
    pub city: String,
    pub country_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportSummary {
    pub institutions_parsed: usize,
    pub countries_created: usize,
    pub banks_created: usize,
    pub banks_updated: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("failed to fetch BCB institutions: {0}")]
    Fetch(String),
    #[error("failed to parse BCB institutions page: {0}")]
    Parse(String),
    #[error("database error: {0}")]
    Database(#[from] doido::model::sea_orm::DbErr),
}

pub type ImportResult<T> = std::result::Result<T, ImportError>;

pub async fn import_authorized_banks(
    db: &DatabaseConnection,
) -> ImportResult<ImportSummary> {
    let html = fetch_bcb_page().await?;
    let institutions = parse_authorized_institutions(&html)?;
    upsert_institutions(db, institutions).await
}

pub async fn fetch_bcb_page() -> ImportResult<String> {
    let response = reqwest::get(BCB_AUTHORIZED_INSTITUTIONS_URL)
        .await
        .map_err(|e| ImportError::Fetch(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ImportError::Fetch(format!(
            "unexpected status {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| ImportError::Fetch(e.to_string()))?;

    Ok(decode_bcb_response(&bytes))
}

pub fn decode_bcb_response(bytes: &[u8]) -> String {
    let (decoded, _, _) = WINDOWS_1252.decode(bytes);
    decoded.into_owned()
}

pub fn parse_authorized_institutions(
    html: &str,
) -> ImportResult<Vec<BcbInstitution>> {
    let row_pattern = Regex::new(
        r#"(?is)<tr[^>]*class="fundoPadraoAClaro1a"[^>]*>\s*<td[^>]*>([^<]+)</td>\s*<td[^>]*>([^<]+)</td>\s*<td[^>]*>([^<]+)</td>\s*<td[^>]*>([^<]+)</td>\s*</tr>"#,
    )
    .map_err(|e| ImportError::Parse(e.to_string()))?;

    let mut seen_codes = HashSet::new();
    let mut institutions = Vec::new();

    for captures in row_pattern.captures_iter(html) {
        let code = normalize_field(
            captures.get(1).map(|m| m.as_str()).unwrap_or_default(),
        );
        let name = normalize_field(
            captures.get(2).map(|m| m.as_str()).unwrap_or_default(),
        );
        let city = normalize_field(
            captures.get(3).map(|m| m.as_str()).unwrap_or_default(),
        );
        let country_name = normalize_field(
            captures.get(4).map(|m| m.as_str()).unwrap_or_default(),
        );

        if code.is_empty() || name.is_empty() || country_name.is_empty() {
            continue;
        }

        if !seen_codes.insert(code.clone()) {
            continue;
        }

        institutions.push(BcbInstitution {
            code,
            name,
            city,
            country_name,
        });
    }

    if institutions.is_empty() {
        return Err(ImportError::Parse(
            "no authorized institutions found in BCB response".into(),
        ));
    }

    Ok(institutions)
}

async fn upsert_institutions(
    db: &DatabaseConnection,
    institutions: Vec<BcbInstitution>,
) -> ImportResult<ImportSummary> {
    let mut summary = ImportSummary {
        institutions_parsed: institutions.len(),
        ..ImportSummary::default()
    };

    for institution in institutions {
        let (country, created) =
            find_or_create_country(db, &institution.country_name).await?;
        if created {
            summary.countries_created += 1;
        }

        let created = upsert_bank(db, &institution, country.id).await?;
        if created {
            summary.banks_created += 1;
        } else {
            summary.banks_updated += 1;
        }
    }

    Ok(summary)
}

async fn find_or_create_country(
    db: &DatabaseConnection,
    country_name: &str,
) -> ImportResult<(crate::models::country::Model, bool)> {
    let code = country_code_for_name(country_name);

    if let Some(existing) = CountryEntity::find()
        .filter(CountryColumn::Code.eq(&code))
        .one(db)
        .await?
    {
        return Ok((existing, false));
    }

    if let Some(existing) = CountryEntity::find()
        .filter(CountryColumn::Name.eq(country_name))
        .one(db)
        .await?
    {
        return Ok((existing, false));
    }

    let created = CountryActiveModel {
        name: Set(country_name.to_string()),
        code: Set(code),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok((created, true))
}

async fn upsert_bank(
    db: &DatabaseConnection,
    institution: &BcbInstitution,
    country_id: i64,
) -> ImportResult<bool> {
    if let Some(existing) = BankEntity::find()
        .filter(BankColumn::Code.eq(&institution.code))
        .one(db)
        .await?
    {
        let mut record: BankActiveModel = existing.into();
        record.name = Set(institution.name.clone());
        record.country_id = Set(country_id);
        record.update(db).await?;
        return Ok(false);
    }

    BankActiveModel {
        name: Set(institution.name.clone()),
        code: Set(institution.code.clone()),
        country_id: Set(country_id),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(true)
}

fn normalize_field(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn country_code_for_name(country_name: &str) -> String {
    match country_name.trim() {
        "Argentina" => "AR".to_string(),
        "Bolívia" => "BO".to_string(),
        "Brasil" => "BR".to_string(),
        "Chile" => "CL".to_string(),
        "Colômbia" => "CO".to_string(),
        "Equador" => "EC".to_string(),
        "México" => "MX".to_string(),
        "Paraguai" => "PY".to_string(),
        "Peru" => "PE".to_string(),
        "República Dominicana" => "DO".to_string(),
        "Uruguai" => "UY".to_string(),
        "Venezuela" => "VE".to_string(),
        other => {
            let normalized: String = other
                .chars()
                .filter(|ch| ch.is_ascii_alphabetic())
                .take(2)
                .collect();
            if normalized.len() == 2 {
                normalized.to_ascii_uppercase()
            } else {
                "XX".to_string()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_HTML: &str = r#"
<table>
<tbody>
<tr class="fundoPadraoAClaro1a"><td>0916</td><td>BANCO BRADESCO S.A.</td><td>SAO PAULO</td><td>Brasil</td></tr>
<tr class="fundoPadraoAClaro1a"><td>0916</td><td>BANCO BRADESCO S.A.</td><td>RIO DE JANEIRO</td><td>Brasil</td></tr>
<tr class="fundoPadraoAClaro1a"><td>0014</td><td>BANCO BICA S.A</td><td>BUENOS AIRES</td><td>Argentina</td></tr>
</tbody>
</table>
"#;

    #[test]
    fn parse_authorized_institutions_extracts_unique_rows() {
        let rows = parse_authorized_institutions(SAMPLE_HTML).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].code, "0916");
        assert_eq!(rows[0].name, "BANCO BRADESCO S.A.");
        assert_eq!(rows[0].country_name, "Brasil");
        assert_eq!(rows[1].code, "0014");
        assert_eq!(rows[1].country_name, "Argentina");
    }

    #[test]
    fn country_code_for_name_maps_bcb_labels() {
        assert_eq!(country_code_for_name("Brasil"), "BR");
        assert_eq!(country_code_for_name("Bolívia"), "BO");
        assert_eq!(country_code_for_name("República Dominicana"), "DO");
    }

    #[test]
    fn decode_bcb_response_reads_windows_1252() {
        let bytes = b"Pra\xe7a";
        assert_eq!(decode_bcb_response(bytes), "Praça");
    }
}
