//! Deployment history management.
//!
//! On completion, the deployment record is copied to
//! `_sys:deploy:history:{timestamp:020}` and `_sys:deploy:current` is deleted.
//! Old history entries are pruned to keep at most `MAX_DEPLOY_HISTORY`.

use aspen_constants::api::MAX_DEPLOY_HISTORY;
use aspen_kv_types::DeleteRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;
use tracing::warn;

use crate::DEPLOY_CURRENT_KEY;
use crate::DEPLOY_HISTORY_PREFIX;
use crate::DeploymentRecord;
use crate::error::DeployError;
use crate::error::Result;

/// Archive a completed deployment to history.
///
/// 1. Write to `_sys:deploy:history:{updated_at_ms:020}`
/// 2. Delete `_sys:deploy:current`
/// 3. Prune old history entries beyond MAX_DEPLOY_HISTORY
pub async fn archive_deployment<K: KeyValueStore + ?Sized>(kv: &K, record: &DeploymentRecord) -> Result<()> {
    let value = serde_json::to_string(record).map_err(|e| DeployError::SerdeError { reason: e.to_string() })?;

    // Write history entry (zero-padded timestamp for lexicographic ordering)
    let history_key = format!("{}{:020}", DEPLOY_HISTORY_PREFIX, record.updated_at_ms);
    kv.write(WriteRequest::set(&history_key, value))
        .await
        .map_err(|e| DeployError::KvError { reason: e.to_string() })?;

    // Delete current deployment record
    let _ = kv.delete(DeleteRequest::new(DEPLOY_CURRENT_KEY)).await;

    debug!(
        deploy_id = %record.deploy_id,
        history_key = %history_key,
        "deployment archived to history"
    );

    // Prune old history entries
    prune_history(kv).await?;

    Ok(())
}

/// Read the latest deployment from history.
pub async fn read_latest_history<K: KeyValueStore + ?Sized>(kv: &K) -> Result<DeploymentRecord> {
    let scan_result = kv
        .scan(ScanRequest {
            prefix: DEPLOY_HISTORY_PREFIX.to_string(),
            limit_results: Some(MAX_DEPLOY_HISTORY),
            continuation_token: None,
        })
        .await
        .map_err(|e| DeployError::KvError { reason: e.to_string() })?;

    // History entries are keyed by timestamp — the last one is the most recent
    if let Some(latest) = scan_result.entries.last() {
        serde_json::from_str(&latest.value).map_err(|e| DeployError::SerdeError { reason: e.to_string() })
    } else {
        Err(DeployError::NoDeploymentFound)
    }
}

/// Prune history entries beyond MAX_DEPLOY_HISTORY.
async fn prune_history<K: KeyValueStore + ?Sized>(kv: &K) -> Result<()> {
    let scan_result = kv
        .scan(ScanRequest {
            prefix: DEPLOY_HISTORY_PREFIX.to_string(),
            limit_results: Some(MAX_DEPLOY_HISTORY.saturating_add(10)),
            continuation_token: None,
        })
        .await
        .map_err(|e| DeployError::KvError { reason: e.to_string() })?;

    let count = scan_result.entries.len();
    if count <= MAX_DEPLOY_HISTORY as usize {
        return Ok(());
    }

    // Delete oldest entries (they're sorted by timestamp key, oldest first)
    let to_delete = count - MAX_DEPLOY_HISTORY as usize;
    for entry in scan_result.entries.iter().take(to_delete) {
        if let Err(e) = kv.delete(DeleteRequest::new(&entry.key)).await {
            warn!(key = %entry.key, error = %e, "failed to prune history entry");
        }
    }

    debug!(pruned = to_delete, remaining = MAX_DEPLOY_HISTORY, "history pruned");

    Ok(())
}
