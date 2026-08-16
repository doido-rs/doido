//! Application models. Each `doido generate model <Name>` registers its module
//! here, just above the marker below.
//!
//! Entity structs live under `_entities/` (regenerated on migrate). Edit the
//! sibling `app/models/<name>.rs` files for validations and custom behaviour.

pub mod _entities;
pub mod enums;

pub mod bank;
pub mod bank_account;
pub mod bank_statement_import;
pub mod category;
pub mod company;
pub mod counterparty;
pub mod country;
pub mod membership;
pub mod transaction;
pub mod user;
pub mod doido_job;
pub mod storage_blob;
pub mod storage_attachment;
pub mod storage_variant_record;
// @generated-models
