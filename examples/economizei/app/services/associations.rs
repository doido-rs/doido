use crate::models::bank::{Entity as BankEntity, Model as Bank};
use crate::models::bank_account::Model as BankAccount;
use crate::models::bank_statement_import::Model as BankStatementImport;
use crate::models::category::Model as Category;
use crate::models::counterparty::Model as Counterparty;
use crate::models::country::{Entity as CountryEntity, Model as Country};
use crate::models::transaction::Model as Transaction;
use doido::model::sea_orm::entity::prelude::*;
use doido::model::sea_orm::DatabaseConnection;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Serialize)]
pub struct AssociationLink {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl AssociationLink {
    pub fn linked(label: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            url: Some(url.into()),
        }
    }

    pub fn plain(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            url: None,
        }
    }
}

#[derive(Serialize)]
pub struct ShowField {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<AssociationLink>,
}

impl ShowField {
    pub fn text(label: impl Into<String>, value: impl ToString) -> Self {
        Self {
            label: label.into(),
            value: Some(value.to_string()),
            link: None,
        }
    }

    pub fn association(label: impl Into<String>, link: AssociationLink) -> Self {
        Self {
            label: label.into(),
            value: None,
            link: Some(link),
        }
    }
}

pub fn show_path(resource: &str, id: i64) -> String {
    format!("/{resource}/{id}")
}

pub fn bank_account_label(account: &BankAccount) -> String {
    format!("{} / {}", account.agency, account.account_number)
}

pub fn bank_label(bank: &Bank) -> String {
    format!("{} ({})", bank.name, bank.code)
}

async fn load_banks(
    db: &DatabaseConnection,
    ids: &HashSet<i64>,
) -> doido::Result<HashMap<i64, Bank>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(BankEntity::find()
        .filter(crate::models::bank::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|bank| (bank.id, bank))
        .collect())
}

async fn load_countries(
    db: &DatabaseConnection,
    ids: &HashSet<i64>,
) -> doido::Result<HashMap<i64, Country>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(CountryEntity::find()
        .filter(crate::models::country::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|country| (country.id, country))
        .collect())
}

async fn load_bank_accounts(
    db: &DatabaseConnection,
    ids: &HashSet<i64>,
) -> doido::Result<HashMap<i64, BankAccount>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(crate::models::bank_account::Entity::find()
        .filter(crate::models::bank_account::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|account| (account.id, account))
        .collect())
}

async fn load_categories(
    db: &DatabaseConnection,
    ids: &HashSet<i64>,
) -> doido::Result<HashMap<i64, Category>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(crate::models::category::Entity::find()
        .filter(crate::models::category::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|category| (category.id, category))
        .collect())
}

async fn load_counterparties(
    db: &DatabaseConnection,
    ids: &HashSet<i64>,
) -> doido::Result<HashMap<i64, Counterparty>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    Ok(crate::models::counterparty::Entity::find()
        .filter(crate::models::counterparty::Column::Id.is_in(ids.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|counterparty| (counterparty.id, counterparty))
        .collect())
}

fn bank_link(banks: &HashMap<i64, Bank>, bank_id: i64) -> AssociationLink {
    banks.get(&bank_id).map_or_else(
        || AssociationLink::plain(format!("#{bank_id}")),
        |bank| AssociationLink::linked(bank_label(bank), show_path("banks", bank.id)),
    )
}

fn bank_account_link(
    accounts: &HashMap<i64, BankAccount>,
    account_id: i64,
) -> AssociationLink {
    accounts.get(&account_id).map_or_else(
        || AssociationLink::plain(format!("#{account_id}")),
        |account| {
            AssociationLink::linked(
                bank_account_label(account),
                show_path("bank_accounts", account.id),
            )
        },
    )
}

fn category_link(categories: &HashMap<i64, Category>, category_id: i64) -> AssociationLink {
    categories.get(&category_id).map_or_else(
        || AssociationLink::plain(format!("#{category_id}")),
        |category| {
            AssociationLink::linked(category.name.clone(), show_path("categories", category.id))
        },
    )
}

fn counterparty_link(
    counterparties: &HashMap<i64, Counterparty>,
    counterparty_id: Option<i64>,
) -> Option<AssociationLink> {
    counterparty_id.map(|id| {
        counterparties.get(&id).map_or_else(
            || AssociationLink::plain(format!("#{id}")),
            |counterparty| {
                AssociationLink::linked(
                    counterparty.name.clone(),
                    show_path("counterparties", counterparty.id),
                )
            },
        )
    })
}

fn country_link(countries: &HashMap<i64, Country>, country_id: i64) -> AssociationLink {
    countries.get(&country_id).map_or_else(
        || AssociationLink::plain(format!("#{country_id}")),
        |country| AssociationLink::plain(format!("{} ({})", country.name, country.code)),
    )
}

#[derive(Serialize)]
pub struct BankAccountIndexRow {
    pub id: i64,
    pub bank: AssociationLink,
    pub agency: String,
    pub account_number: String,
    pub cpf_cnpj: String,
    pub account_type: crate::models::enums::AccountType,
}

pub async fn bank_account_index_rows(
    db: &DatabaseConnection,
    accounts: Vec<BankAccount>,
) -> doido::Result<Vec<BankAccountIndexRow>> {
    let bank_ids: HashSet<i64> = accounts.iter().map(|a| a.bank_id).collect();
    let banks = load_banks(db, &bank_ids).await?;
    Ok(accounts
        .into_iter()
        .map(|account| BankAccountIndexRow {
            id: account.id,
            bank: bank_link(&banks, account.bank_id),
            agency: account.agency,
            account_number: account.account_number,
            cpf_cnpj: account.cpf_cnpj,
            account_type: account.account_type,
        })
        .collect())
}

#[derive(Serialize)]
pub struct BankIndexRow {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub country: AssociationLink,
}

pub async fn bank_index_rows(
    db: &DatabaseConnection,
    banks: Vec<Bank>,
) -> doido::Result<Vec<BankIndexRow>> {
    let country_ids: HashSet<i64> = banks.iter().map(|b| b.country_id).collect();
    let countries = load_countries(db, &country_ids).await?;
    Ok(banks
        .into_iter()
        .map(|bank| BankIndexRow {
            id: bank.id,
            name: bank.name,
            code: bank.code,
            country: country_link(&countries, bank.country_id),
        })
        .collect())
}

#[derive(Serialize)]
pub struct TransactionIndexRow {
    pub id: i64,
    pub occurred_at: String,
    pub amount: String,
    pub operation: crate::models::enums::Operation,
    pub movement_type: crate::models::enums::MovementType,
    pub category: AssociationLink,
    pub bank_account: AssociationLink,
    pub counterparty: Option<AssociationLink>,
}

pub async fn transaction_index_rows(
    db: &DatabaseConnection,
    rows: Vec<Transaction>,
) -> doido::Result<Vec<TransactionIndexRow>> {
    let bank_account_ids: HashSet<i64> =
        rows.iter().map(|row| row.bank_account_id).collect();
    let category_ids: HashSet<i64> = rows.iter().map(|row| row.category_id).collect();
    let counterparty_ids: HashSet<i64> = rows
        .iter()
        .filter_map(|row| row.counterparty_id)
        .collect();

    let bank_accounts = load_bank_accounts(db, &bank_account_ids).await?;
    let categories = load_categories(db, &category_ids).await?;
    let counterparties = load_counterparties(db, &counterparty_ids).await?;

    Ok(rows
        .into_iter()
        .map(|row| TransactionIndexRow {
            id: row.id,
            occurred_at: row.occurred_at.to_string(),
            amount: row.amount.to_string(),
            operation: row.operation,
            movement_type: row.movement_type,
            category: category_link(&categories, row.category_id),
            bank_account: bank_account_link(&bank_accounts, row.bank_account_id),
            counterparty: counterparty_link(&counterparties, row.counterparty_id),
        })
        .collect())
}

#[derive(Serialize)]
pub struct BankStatementImportIndexRow {
    pub id: i64,
    pub bank_account: AssociationLink,
    pub source: crate::models::enums::ImportSource,
    pub statement_type: crate::models::enums::ImportStatementType,
    pub original_filename: String,
    pub transactions_imported: i32,
    pub status: crate::models::enums::ImportStatus,
    pub created_at: String,
}

pub async fn bank_statement_import_index_rows(
    db: &DatabaseConnection,
    rows: Vec<BankStatementImport>,
) -> doido::Result<Vec<BankStatementImportIndexRow>> {
    let account_ids: HashSet<i64> = rows.iter().map(|row| row.bank_account_id).collect();
    let bank_accounts = load_bank_accounts(db, &account_ids).await?;

    Ok(rows
        .into_iter()
        .map(|row| BankStatementImportIndexRow {
            id: row.id,
            bank_account: bank_account_link(&bank_accounts, row.bank_account_id),
            source: row.source,
            statement_type: row.statement_type,
            original_filename: row.original_filename,
            transactions_imported: row.transactions_imported,
            status: row.status,
            created_at: row.created_at.to_string(),
        })
        .collect())
}

pub async fn bank_account_show_fields(
    db: &DatabaseConnection,
    account: &BankAccount,
    labels: &serde_json::Map<String, serde_json::Value>,
) -> doido::Result<Vec<ShowField>> {
    let banks = load_banks(db, &HashSet::from([account.bank_id])).await?;
    Ok(vec![
        ShowField::text(label(labels, "id"), account.id),
        ShowField::association(label(labels, "bank_id"), bank_link(&banks, account.bank_id)),
        ShowField::text(label(labels, "agency"), &account.agency),
        ShowField::text(label(labels, "account_number"), &account.account_number),
        ShowField::text(label(labels, "cpf_cnpj"), &account.cpf_cnpj),
        ShowField::text(
            label(labels, "account_type"),
            serde_json::to_string(&account.account_type).unwrap_or_default(),
        ),
    ])
}

pub async fn bank_show_fields(
    db: &DatabaseConnection,
    bank: &Bank,
    labels: &serde_json::Map<String, serde_json::Value>,
) -> doido::Result<Vec<ShowField>> {
    let countries = load_countries(db, &HashSet::from([bank.country_id])).await?;
    Ok(vec![
        ShowField::text(label(labels, "id"), bank.id),
        ShowField::text(label(labels, "name"), &bank.name),
        ShowField::text(label(labels, "code"), &bank.code),
        ShowField::association(
            label(labels, "country_id"),
            country_link(&countries, bank.country_id),
        ),
    ])
}

pub async fn transaction_show_fields(
    db: &DatabaseConnection,
    row: &Transaction,
    labels: &serde_json::Map<String, serde_json::Value>,
) -> doido::Result<Vec<ShowField>> {
    let bank_accounts =
        load_bank_accounts(db, &HashSet::from([row.bank_account_id])).await?;
    let categories = load_categories(db, &HashSet::from([row.category_id])).await?;
    let counterparty_ids = row.counterparty_id.into_iter().collect();
    let counterparties = load_counterparties(db, &counterparty_ids).await?;

    let mut fields = vec![
        ShowField::text(label(labels, "id"), row.id),
        ShowField::text(label(labels, "occurred_at"), row.occurred_at.to_string()),
        ShowField::text(label(labels, "amount"), row.amount.to_string()),
        ShowField::text(
            label(labels, "operation"),
            serde_json::to_string(&row.operation).unwrap_or_default(),
        ),
        ShowField::text(
            label(labels, "movement_type"),
            serde_json::to_string(&row.movement_type).unwrap_or_default(),
        ),
        ShowField::association(
            label(labels, "bank_account_id"),
            bank_account_link(&bank_accounts, row.bank_account_id),
        ),
        ShowField::association(
            label(labels, "category_id"),
            category_link(&categories, row.category_id),
        ),
    ];

    if let Some(link) = counterparty_link(&counterparties, row.counterparty_id) {
        fields.push(ShowField::association(label(labels, "counterparty_id"), link));
    }

    Ok(fields)
}

pub fn category_show_fields(
    category: &Category,
    labels: &serde_json::Map<String, serde_json::Value>,
) -> Vec<ShowField> {
    vec![
        ShowField::text(label(labels, "id"), category.id),
        ShowField::text(label(labels, "name"), &category.name),
    ]
}

pub fn counterparty_show_fields(
    counterparty: &Counterparty,
    labels: &serde_json::Map<String, serde_json::Value>,
) -> Vec<ShowField> {
    vec![
        ShowField::text(label(labels, "id"), counterparty.id),
        ShowField::text(label(labels, "name"), &counterparty.name),
    ]
}

pub async fn bank_statement_import_show_fields(
    db: &DatabaseConnection,
    row: &BankStatementImport,
    labels: &serde_json::Map<String, serde_json::Value>,
) -> doido::Result<Vec<ShowField>> {
    let bank_accounts =
        load_bank_accounts(db, &HashSet::from([row.bank_account_id])).await?;
    Ok(vec![
        ShowField::text(label(labels, "id"), row.id),
        ShowField::association(
            label(labels, "bank_account_id"),
            bank_account_link(&bank_accounts, row.bank_account_id),
        ),
        ShowField::text(
            label(labels, "source"),
            serde_json::to_string(&row.source).unwrap_or_default(),
        ),
        ShowField::text(
            label(labels, "statement_type"),
            serde_json::to_string(&row.statement_type).unwrap_or_default(),
        ),
        ShowField::text(label(labels, "original_filename"), &row.original_filename),
        ShowField::text(label(labels, "transactions_imported"), row.transactions_imported),
        ShowField::text(
            label(labels, "status"),
            serde_json::to_string(&row.status).unwrap_or_default(),
        ),
        ShowField::text(label(labels, "created_at"), row.created_at.to_string()),
    ])
}

fn label(labels: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    labels
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or(key)
        .to_string()
}
