//! Imports authorized financial institutions from the BCB CCR registry.
#![allow(dead_code)]

use crate::services::banks::bcb_import;
use doido_jobs::{job, JobContext};
use serde::{Deserialize, Serialize};

/// Empty payload — the job always performs a full sync from the BCB source.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportAuthorizedBanksPayload {}

#[job(max_retries = 3, queue = "default")]
async fn import_authorized_banks_job(
    ctx: &JobContext,
    _payload: ImportAuthorizedBanksPayload,
) -> doido_core::Result<()> {
    let summary = bcb_import::import_authorized_banks(ctx.db())
        .await
        .map_err(|error| doido_core::anyhow::anyhow!("{error}"))?;

    doido_core::tracing::info!(
        institutions_parsed = summary.institutions_parsed,
        countries_created = summary.countries_created,
        banks_created = summary.banks_created,
        banks_updated = summary.banks_updated,
        "BCB authorized banks import completed"
    );

    Ok(())
}
