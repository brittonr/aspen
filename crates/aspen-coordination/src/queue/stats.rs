//! Queue statistics and status operations.

use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::CAS_RETRY_INITIAL_BACKOFF_MS;
use aspen_constants::coordination::CAS_RETRY_MAX_BACKOFF_MS;
use aspen_constants::coordination::MAX_CAS_RETRIES;
use aspen_constants::coordination::MAX_QUEUE_BATCH_SIZE;
use aspen_constants::coordination::MAX_QUEUE_CLEANUP_BATCH;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;

use super::QueueItem;
use super::QueueManager;
use super::QueueStats;
use super::QueueStatus;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Get queue status.
    pub async fn status(&self, name: &str) -> Result<QueueStatus> {
        let queue_key = verified::queue_metadata_key(name);

        let queue_state = match self.read_queue_state(&queue_key).await? {
            Some(state) => state,
            None => return Ok(QueueStatus::default()),
        };

        let items_pref = verified::items_prefix(name);
        let pending_pref = verified::pending_prefix(name);
        let dlq_pref = verified::dlq_prefix(name);

        let visible_keys = self.scan_keys(&items_pref, MAX_QUEUE_CLEANUP_BATCH).await?;
        let pending_keys = self.scan_keys(&pending_pref, MAX_QUEUE_CLEANUP_BATCH).await?;
        let dlq_keys = self.scan_keys(&dlq_pref, MAX_QUEUE_CLEANUP_BATCH).await?;

        Ok(QueueStatus {
            exists: true,
            visible_count: visible_keys.len() as u64,
            pending_count: pending_keys.len() as u64,
            dlq_count: dlq_keys.len() as u64,
            total_enqueued: queue_state.stats.total_enqueued,
            total_acked: queue_state.stats.total_acked,
            total_dlq: queue_state.stats.total_dlq,
        })
    }

    /// Peek at items without removing them.
    ///
    /// Returns up to `max_items` items from the front of the queue.
    pub async fn peek(&self, name: &str, max_items: u32) -> Result<Vec<QueueItem>> {
        let max_items = max_items.min(MAX_QUEUE_BATCH_SIZE);
        let items_pref = verified::items_prefix(name);

        let item_keys = self.scan_keys(&items_pref, max_items).await?;
        let mut items = Vec::new();

        for key in item_keys {
            if let Some(item) = self.read_json::<QueueItem>(&key).await?
                && !item.is_expired()
            {
                items.push(item);
            }
        }

        Ok(items)
    }

    /// Increment enqueue count in queue stats.
    pub(super) async fn increment_enqueue_count(&self, name: &str) -> Result<()> {
        self.update_queue_stats(name, |stats| stats.total_enqueued += 1).await
    }

    /// Increment ack count in queue stats.
    pub(super) async fn increment_ack_count(&self, name: &str) -> Result<()> {
        self.update_queue_stats(name, |stats| stats.total_acked += 1).await
    }

    /// Increment DLQ count in queue stats.
    pub(super) async fn increment_dlq_count(&self, name: &str) -> Result<()> {
        self.update_queue_stats(name, |stats| stats.total_dlq += 1).await
    }

    /// Update queue stats atomically.
    async fn update_queue_stats<F>(&self, name: &str, update_fn: F) -> Result<()>
    where F: Fn(&mut QueueStats) {
        let queue_key = verified::queue_metadata_key(name);
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let current = match self.read_queue_state(&queue_key).await? {
                Some(state) => state,
                None => return Ok(()), // Queue doesn't exist
            };

            let mut new_state = current.clone();
            update_fn(&mut new_state.stats);

            let old_json = serde_json::to_string(&current)?;
            let new_json = serde_json::to_string(&new_state)?;

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: queue_key.clone(),
                        expected: Some(old_json),
                        new_value: new_json,
                    },
                })
                .await
            {
                Ok(_) => return Ok(()),
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    attempt += 1;
                    if attempt >= MAX_CAS_RETRIES {
                        bail!("stats update CAS failed after {} attempts (max retries exceeded)", attempt);
                    }
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                }
                Err(e) => bail!("stats update CAS failed: {}", e),
            }
        }
    }
}
