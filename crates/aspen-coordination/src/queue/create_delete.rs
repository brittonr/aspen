//! Queue creation and deletion operations.

use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
use aspen_constants::coordination::CAS_RETRY_MAX_BACKOFF_MS;
use aspen_constants::coordination::DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS;
use aspen_constants::coordination::MAX_CAS_RETRIES;
use aspen_constants::coordination::MAX_QUEUE_CLEANUP_BATCH;
use aspen_constants::coordination::MAX_QUEUE_ITEM_TTL_MS;
use aspen_constants::coordination::MAX_QUEUE_VISIBILITY_TIMEOUT_MS;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::QueueConfig;
use super::QueueManager;
use super::QueueState;
use super::QueueStats;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Create or configure a queue.
    ///
    /// Returns (created, queue_state) where created is false if queue already existed.
    pub async fn create(&self, name: &str, config: QueueConfig) -> Result<(bool, QueueState)> {
        let key = verified::queue_metadata_key(name);
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let current = self.read_queue_state(&key).await?;

            match current {
                None => {
                    // Create new queue
                    let visibility_timeout = config
                        .default_visibility_timeout_ms
                        .unwrap_or(DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS)
                        .min(MAX_QUEUE_VISIBILITY_TIMEOUT_MS);
                    let ttl = config.default_ttl_ms.unwrap_or(0).min(MAX_QUEUE_ITEM_TTL_MS);

                    assert!(
                        visibility_timeout <= MAX_QUEUE_VISIBILITY_TIMEOUT_MS,
                        "QUEUE: visibility timeout must be <= max: {visibility_timeout} > {MAX_QUEUE_VISIBILITY_TIMEOUT_MS}"
                    );
                    assert!(ttl <= MAX_QUEUE_ITEM_TTL_MS, "QUEUE: TTL must be <= max: {ttl} > {MAX_QUEUE_ITEM_TTL_MS}");

                    let state = QueueState {
                        name: name.to_string(),
                        max_delivery_attempts: config.max_delivery_attempts.unwrap_or(3),
                        default_visibility_timeout_ms: visibility_timeout,
                        default_ttl_ms: ttl,
                        created_at_ms: now_unix_ms(),
                        stats: QueueStats::default(),
                    };
                    let new_json = serde_json::to_string(&state)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: None,
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(name, "queue created");
                            return Ok((true, state));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                            attempt += 1;
                            if attempt >= MAX_CAS_RETRIES {
                                bail!("queue create CAS failed after {} attempts (max retries exceeded)", attempt);
                            }
                            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                            backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                        }
                        Err(e) => bail!("queue create CAS failed: {}", e),
                    }
                }
                Some(state) => {
                    // Queue already exists
                    return Ok((false, state));
                }
            }
        }
    }

    /// Delete a queue and all its items.
    ///
    /// Returns the number of items deleted.
    pub async fn delete(&self, name: &str) -> Result<u64> {
        let queue_key = verified::queue_metadata_key(name);
        let items_prefix = verified::items_prefix(name);
        let pending_prefix = verified::pending_prefix(name);
        let dlq_prefix = verified::dlq_prefix(name);
        let dedup_prefix = verified::dedup_prefix(name);
        let seq_key = verified::sequence_key(name);

        let mut deleted = 0u64;

        // Delete all items
        for prefix in [&items_prefix, &pending_prefix, &dlq_prefix, &dedup_prefix] {
            let items = self.scan_keys(prefix, MAX_QUEUE_CLEANUP_BATCH).await?;
            for key in items {
                if self.delete_key(&key).await? {
                    deleted += 1;
                }
            }
        }

        // Delete sequence
        let _ = self.delete_key(&seq_key).await;

        // Delete queue metadata
        let _ = self.delete_key(&queue_key).await;

        debug!(name, deleted, "queue deleted");
        Ok(deleted)
    }
}
