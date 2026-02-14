//! Queue visibility timeout operations.

use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
use aspen_constants::coordination::CAS_RETRY_MAX_BACKOFF_MS;
use aspen_constants::coordination::MAX_CAS_RETRIES;
use aspen_constants::coordination::MAX_QUEUE_VISIBILITY_TIMEOUT_MS;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::PendingItem;
use super::QueueManager;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Extend the visibility timeout for a pending item.
    ///
    /// Returns the new visibility deadline.
    pub async fn extend_visibility(&self, name: &str, receipt_handle: &str, additional_timeout_ms: u64) -> Result<u64> {
        let additional_timeout_ms = additional_timeout_ms.min(MAX_QUEUE_VISIBILITY_TIMEOUT_MS);
        let item_id = self.parse_receipt_handle(receipt_handle)?;
        let pend_key = verified::pending_key(name, item_id);
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let pending: PendingItem = match self.read_json(&pend_key).await? {
                Some(p) => p,
                None => bail!("item not found or already processed"),
            };

            if pending.receipt_handle != receipt_handle {
                bail!("receipt handle mismatch - item may have been redelivered");
            }

            let new_deadline = verified::compute_visibility_deadline(now_unix_ms(), additional_timeout_ms);
            let mut new_pending = pending.clone();
            new_pending.visibility_deadline_ms = new_deadline;

            let old_json = serde_json::to_string(&pending)?;
            let new_json = serde_json::to_string(&new_pending)?;

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: pend_key.clone(),
                        expected: Some(old_json),
                        new_value: new_json,
                    },
                })
                .await
            {
                Ok(_) => {
                    debug!(name, item_id, new_deadline, "visibility extended");
                    return Ok(new_deadline);
                }
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    attempt += 1;
                    if attempt >= MAX_CAS_RETRIES {
                        bail!("extend visibility CAS failed after {} attempts (max retries exceeded)", attempt);
                    }
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                }
                Err(e) => bail!("extend visibility CAS failed: {}", e),
            }
        }
    }
}
