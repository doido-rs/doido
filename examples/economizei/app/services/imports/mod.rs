mod c6;
mod nubank;
pub mod parser;

use crate::models::bank_account::Entity as BankAccountEntity;
use crate::models::bank_statement_import::{
    ActiveModel as ImportActiveModel, Column as ImportColumn,
    Entity as ImportEntity, Model as ImportModel,
};
use crate::models::category::{
    ActiveModel as CategoryActiveModel, Column as CategoryColumn,
    Entity as CategoryEntity,
};
use crate::models::counterparty::{
    ActiveModel as CounterpartyActiveModel, Column as CounterpartyColumn,
    Entity as CounterpartyEntity,
};
use crate::models::enums::{
    ImportSource, ImportStatementType, ImportStatus, MovementType,
};
use crate::models::transaction::ActiveModel as TransactionActiveModel;
use crate::services::i18n;
use chrono::Utc;
use doido::model::sea_orm::{
    entity::prelude::*, ActiveValue::Set, TransactionTrait,
};
use flate2::write::GzEncoder;
use flate2::Compression;
use parser::{parse_statement, ParseError, ParsedRow};
use sha2::{Digest, Sha256};
use std::io::Write;

pub struct ImportRequest {
    pub user_id: i64,
    pub company_id: i64,
    pub bank_account_id: i64,
    pub source: ImportSource,
    pub statement_type: ImportStatementType,
    pub original_filename: String,
    pub file_bytes: Vec<u8>,
}

pub struct ImportResult {
    pub import: ImportModel,
    pub transactions_imported: i32,
}

pub enum ImportFailure {
    DuplicateFile,
    BankAccountNotFound,
    BankAccountForbidden,
    Parse(ParseError),
    Database(String),
}

impl ImportFailure {
    pub fn message(&self) -> String {
        match self {
            Self::DuplicateFile => i18n::t("imports.errors.duplicate_file"),
            Self::BankAccountNotFound => {
                i18n::t("imports.errors.bank_account_not_found")
            }
            Self::BankAccountForbidden => {
                i18n::t("imports.errors.bank_account_forbidden")
            }
            Self::Parse(err) => i18n::t("imports.errors.parse")
                .replace("{line}", &err.line.to_string())
                .replace("{message}", &err.message),
            Self::Database(message) => message.clone(),
        }
    }
}

pub fn compress_file(bytes: &[u8]) -> doido::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(bytes)
        .map_err(|e| doido::core::anyhow::anyhow!("compression failed: {e}"))?;
    encoder
        .finish()
        .map_err(|e| doido::core::anyhow::anyhow!("compression failed: {e}"))
}

pub fn decompress_file(bytes: &[u8]) -> doido::Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(bytes);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).map_err(|e| {
        doido::core::anyhow::anyhow!("decompression failed: {e}")
    })?;
    Ok(out)
}

pub fn file_checksum(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

pub async fn run(
    db: &DatabaseConnection,
    request: ImportRequest,
) -> Result<ImportResult, ImportFailure> {
    let account = BankAccountEntity::find_by_id(request.bank_account_id)
        .one(db)
        .await
        .map_err(|e| ImportFailure::Database(e.to_string()))?
        .ok_or(ImportFailure::BankAccountNotFound)?;

    if account.user_id != request.user_id {
        return Err(ImportFailure::BankAccountForbidden);
    }

    let checksum = file_checksum(&request.file_bytes);
    let duplicate = ImportEntity::find()
        .filter(ImportColumn::BankAccountId.eq(request.bank_account_id))
        .filter(ImportColumn::FileChecksum.eq(&checksum))
        .one(db)
        .await
        .map_err(|e| ImportFailure::Database(e.to_string()))?;

    if duplicate.is_some() {
        return Err(ImportFailure::DuplicateFile);
    }

    let csv_content =
        String::from_utf8(request.file_bytes.clone()).map_err(|_| {
            ImportFailure::Parse(ParseError::new(0, "file is not valid UTF-8"))
        })?;

    let rows = parse_statement(
        request.source.clone(),
        request.statement_type.clone(),
        &csv_content,
    )
    .map_err(ImportFailure::Parse)?;

    let compressed = compress_file(&request.file_bytes)
        .map_err(|e| ImportFailure::Database(e.to_string()))?;
    let now = Utc::now().naive_utc();
    let movement_type = movement_type_for(&request.statement_type);

    let txn = db
        .begin()
        .await
        .map_err(|e| ImportFailure::Database(e.to_string()))?;

    let default_category_id = ensure_default_category(&txn, request.company_id)
        .await
        .map_err(|e| ImportFailure::Database(e.to_string()))?;

    let mut imported = 0_i32;
    for row in rows {
        let category_id = resolve_category(
            &txn,
            request.company_id,
            default_category_id,
            &row,
        )
        .await
        .map_err(|e| ImportFailure::Database(e.to_string()))?;
        let counterparty_id =
            resolve_counterparty(&txn, request.company_id, &row.description)
                .await
                .map_err(|e| ImportFailure::Database(e.to_string()))?;

        let record = TransactionActiveModel {
            company_id: Set(request.company_id),
            bank_account_id: Set(request.bank_account_id),
            category_id: Set(category_id),
            counterparty_id: Set(counterparty_id),
            occurred_at: Set(row.occurred_at),
            amount: Set(row.amount),
            operation: Set(row.operation),
            movement_type: Set(movement_type.clone()),
            ..Default::default()
        };
        record
            .insert(&txn)
            .await
            .map_err(|e| ImportFailure::Database(e.to_string()))?;
        imported += 1;
    }

    let import_record = ImportActiveModel {
        user_id: Set(request.user_id),
        bank_account_id: Set(request.bank_account_id),
        company_id: Set(request.company_id),
        source: Set(request.source),
        statement_type: Set(request.statement_type),
        original_filename: Set(request.original_filename),
        compressed_data: Set(compressed),
        file_checksum: Set(checksum),
        byte_size: Set(request.file_bytes.len() as i64),
        transactions_imported: Set(imported),
        status: Set(ImportStatus::Completed),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(|e| ImportFailure::Database(e.to_string()))?;

    txn.commit()
        .await
        .map_err(|e| ImportFailure::Database(e.to_string()))?;

    Ok(ImportResult {
        import: import_record,
        transactions_imported: imported,
    })
}

fn movement_type_for(statement_type: &ImportStatementType) -> MovementType {
    match statement_type {
        ImportStatementType::CheckingAccount => MovementType::Balance,
        ImportStatementType::CreditCard => MovementType::CreditCard,
    }
}

async fn ensure_default_category(
    db: &impl ConnectionTrait,
    company_id: i64,
) -> doido::Result<i64> {
    let name = i18n::t("imports.default_category");
    if let Some(existing) = CategoryEntity::find()
        .filter(CategoryColumn::CompanyId.eq(company_id))
        .filter(CategoryColumn::Name.eq(&name))
        .one(db)
        .await?
    {
        return Ok(existing.id);
    }

    let created = CategoryActiveModel {
        company_id: Set(company_id),
        name: Set(name),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(created.id)
}

async fn resolve_category(
    db: &impl ConnectionTrait,
    company_id: i64,
    default_category_id: i64,
    row: &ParsedRow,
) -> doido::Result<i64> {
    let Some(name) =
        row.category_name.as_ref().filter(|v| !v.trim().is_empty())
    else {
        return Ok(default_category_id);
    };

    if let Some(existing) = CategoryEntity::find()
        .filter(CategoryColumn::CompanyId.eq(company_id))
        .filter(CategoryColumn::Name.eq(name))
        .one(db)
        .await?
    {
        return Ok(existing.id);
    }

    let created = CategoryActiveModel {
        company_id: Set(company_id),
        name: Set(name.clone()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(created.id)
}

async fn resolve_counterparty(
    db: &impl ConnectionTrait,
    company_id: i64,
    description: &str,
) -> doido::Result<Option<i64>> {
    let name = description.trim();
    if name.is_empty() {
        return Ok(None);
    }

    if let Some(existing) = CounterpartyEntity::find()
        .filter(CounterpartyColumn::CompanyId.eq(company_id))
        .filter(CounterpartyColumn::Name.eq(name))
        .one(db)
        .await?
    {
        return Ok(Some(existing.id));
    }

    let created = CounterpartyActiveModel {
        company_id: Set(company_id),
        name: Set(name.to_string()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(Some(created.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compresses_and_decompresses_roundtrip() {
        let original = b"date,title,amount\n2026-01-01,Test,10.00\n".to_vec();
        let compressed = compress_file(&original).unwrap();
        let restored = decompress_file(&compressed).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn checksum_is_stable() {
        let bytes = b"sample csv";
        assert_eq!(file_checksum(bytes), file_checksum(bytes));
        assert_ne!(file_checksum(b"a"), file_checksum(b"b"));
    }
}
