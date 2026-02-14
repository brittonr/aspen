//! Internal helper methods for queue operations.

use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use serde::Deserialize;

use super::QueueManager;
use super::QueueState;
use super::types::DeduplicationEntry;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Parse item ID from receipt handle.
    pub(super) fn parse_receipt_handle(&self, receipt_handle: &str) -> Result<u64> {
        crate::verified::parse_receipt_handle(receipt_handle)
            .ok_or_else(|| anyhow::anyhow!("invalid receipt handle format"))
    }

    /// Read queue state.
    pub(super) async fn read_queue_state(&self, key: &str) -> Result<Option<QueueState>> {
        self.read_json(key).await
    }

    /// Read deduplication entry.
    pub(super) async fn read_dedup_entry(&self, key: &str) -> Result<Option<DeduplicationEntry>> {
        self.read_json(key).await
    }

    /// Read and deserialize JSON from a key.
    pub(super) async fn read_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        match self.store.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value_str.is_empty() {
                    Ok(None)
                } else {
                    let value: T = serde_json::from_str(&value_str)?;
                    Ok(Some(value))
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => bail!("read failed: {}", e),
        }
    }

    /// Scan keys with a prefix.
    pub(super) async fn scan_keys(&self, prefix: &str, limit: u32) -> Result<Vec<String>> {
        match self
            .store
            .scan(aspen_core::ScanRequest {
                prefix: prefix.to_string(),
                limit: Some(limit),
                continuation_token: None,
            })
            .await
        {
            Ok(result) => Ok(result.entries.iter().map(|e| e.key.clone()).collect()),
            Err(e) => bail!("scan failed: {}", e),
        }
    }

    /// Delete a key.
    pub(super) async fn delete_key(&self, key: &str) -> Result<bool> {
        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::Delete { key: key.to_string() },
            })
            .await
        {
            Ok(_) => Ok(true),
            Err(KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => bail!("delete failed: {}", e),
        }
    }
}
