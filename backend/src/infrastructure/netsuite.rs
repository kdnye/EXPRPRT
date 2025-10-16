use serde::{Deserialize, Serialize};
use tracing::info;

use crate::domain::models::{JournalLine, NetSuiteBatch};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetSuiteResponse {
    pub succeeded: bool,
    pub reference: Option<String>,
    pub message: Option<String>,
}

pub async fn export_batch(
    _batch: &NetSuiteBatch,
    _lines: &[JournalLine],
) -> anyhow::Result<NetSuiteResponse> {
    // Stub implementation – integrate with REST/SOAP client once credentials available.
    info!("netsuite export stub invoked");
    Ok(NetSuiteResponse {
        succeeded: true,
        reference: Some("STUB-REF".to_string()),
        message: Some("Simulated export".to_string()),
    })
}
