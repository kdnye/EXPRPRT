use serde::{Deserialize, Serialize};
use tracing::info;

use crate::domain::models::{JournalLine, NetSuiteBatch};

#[cfg(test)]
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(test)]
type ExportBatchOverride =
    dyn Fn(&NetSuiteBatch, &[JournalLine]) -> anyhow::Result<NetSuiteResponse> + Send + Sync;

#[cfg(test)]
static EXPORT_BATCH_OVERRIDE: OnceLock<Mutex<Option<Arc<ExportBatchOverride>>>> = OnceLock::new();

#[cfg(test)]
pub struct ExportBatchOverrideGuard;

#[cfg(test)]
impl Drop for ExportBatchOverrideGuard {
    fn drop(&mut self) {
        if let Some(cell) = EXPORT_BATCH_OVERRIDE.get() {
            if let Ok(mut guard) = cell.lock() {
                *guard = None;
            }
        }
    }
}

#[cfg(test)]
pub fn install_export_batch_override<F>(override_fn: F) -> ExportBatchOverrideGuard
where
    F: Fn(&NetSuiteBatch, &[JournalLine]) -> anyhow::Result<NetSuiteResponse>
        + Send
        + Sync
        + 'static,
{
    let cell = EXPORT_BATCH_OVERRIDE.get_or_init(|| Mutex::new(None));
    let mut guard = cell.lock().expect("export batch override mutex poisoned");
    *guard = Some(Arc::new(override_fn));
    ExportBatchOverrideGuard
}

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
    #[cfg(test)]
    {
        if let Some(override_fn) = EXPORT_BATCH_OVERRIDE
            .get()
            .and_then(|cell| cell.lock().ok().and_then(|guard| guard.as_ref().cloned()))
        {
            return override_fn(_batch, _lines);
        }
    }

    // Stub implementation – integrate with REST/SOAP client once credentials available.
    info!("netsuite export stub invoked");
    Ok(NetSuiteResponse {
        succeeded: true,
        reference: Some("STUB-REF".to_string()),
        message: Some("Simulated export".to_string()),
    })
}
