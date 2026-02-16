//! Internal helper methods for the service registry.

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use serde::Deserialize;

use super::ServiceRegistry;
use super::types::ServiceInstance;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> ServiceRegistry<S> {
    /// Cleanup expired instances for a service.
    pub(super) async fn cleanup_expired(&self, service_name: &str) -> Result<u32> {
        use aspen_constants::coordination::SERVICE_CLEANUP_BATCH;
        use tracing::debug;

        let prefix = verified::service_instances_prefix(service_name);
        let keys = self.scan_keys(&prefix, SERVICE_CLEANUP_BATCH).await?;

        let mut cleaned = 0u32;

        for key in keys {
            if let Some(instance) = self.read_json::<ServiceInstance>(&key).await?
                && instance.is_expired()
            {
                let _ = self.delete_key(&key).await;
                cleaned += 1;
            }
        }

        if cleaned > 0 {
            debug!(service_name, cleaned, "expired instances cleaned up");
        }

        Ok(cleaned)
    }

    /// Generate instance key.
    pub(super) fn instance_key(service_name: &str, instance_id: &str) -> String {
        verified::instance_key(service_name, instance_id)
    }

    /// Read JSON from key.
    pub(super) async fn read_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        match self.store.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value.is_empty() {
                    Ok(None)
                } else {
                    let parsed = serde_json::from_str(&value)
                        .with_context(|| format!("failed to parse JSON for key '{}'", key))?;
                    Ok(Some(parsed))
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => bail!("failed to read {}: {}", key, e),
        }
    }

    /// Scan keys with prefix.
    pub(super) async fn scan_keys(&self, prefix: &str, limit: u32) -> Result<Vec<String>> {
        use aspen_kv_types::ScanRequest;

        match self
            .store
            .scan(ScanRequest {
                prefix: prefix.to_string(),
                limit: Some(limit),
                continuation_token: None,
            })
            .await
        {
            Ok(result) => Ok(result.entries.iter().map(|e| e.key.clone()).collect()),
            Err(e) => bail!("failed to scan {}: {}", prefix, e),
        }
    }

    /// Delete a key.
    pub(super) async fn delete_key(&self, key: &str) -> Result<()> {
        self.store
            .write(WriteRequest {
                command: WriteCommand::Delete { key: key.to_string() },
            })
            .await
            .map_err(|e| anyhow::anyhow!("failed to delete {}: {}", key, e))?;
        Ok(())
    }
}
