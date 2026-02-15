//! Queue dequeue operations.

use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::DEFAULT_QUEUE_POLL_INTERVAL_MS;
use aspen_constants::coordination::MAX_QUEUE_BATCH_SIZE;
use aspen_constants::coordination::MAX_QUEUE_CLEANUP_BATCH;
use aspen_constants::coordination::MAX_QUEUE_POLL_INTERVAL_MS;
use aspen_constants::coordination::MAX_QUEUE_VISIBILITY_TIMEOUT_MS;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;
use tracing::info;

use super::DLQReason;
use super::DequeuedItem;
use super::PendingItem;
use super::QueueItem;
use super::QueueManager;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Dequeue items with visibility timeout (non-blocking).
    ///
    /// Returns up to `max_items` items. Each item is locked for `visibility_timeout_ms`.
    /// Returns empty vec if queue is empty.
    pub async fn dequeue(
        &self,
        name: &str,
        consumer_id: &str,
        max_items: u32,
        visibility_timeout_ms: u64,
    ) -> Result<Vec<DequeuedItem>> {
        let max_items = max_items.min(MAX_QUEUE_BATCH_SIZE);
        let visibility_timeout_ms = visibility_timeout_ms.min(MAX_QUEUE_VISIBILITY_TIMEOUT_MS);

        // Cleanup expired pending items first
        self.cleanup_expired_pending(name).await?;

        let queue_key = verified::queue_metadata_key(name);
        let queue_state = match self.read_queue_state(&queue_key).await? {
            Some(state) => state,
            None => {
                info!(name, queue_key, "queue does not exist, returning empty");
                return Ok(vec![]);
            }
        };

        let items_pref = verified::items_prefix(name);

        // Get list of message groups currently pending (for FIFO ordering)
        let pending_groups = self.get_pending_message_groups(name).await?;

        // Scan for available items
        let item_keys = self.scan_keys(&items_pref, max_items * 2).await?;

        info!(
            name,
            consumer_id,
            items_prefix = items_pref,
            items_found = item_keys.len(),
            pending_groups = ?pending_groups,
            "dequeue scanning for items"
        );

        let mut dequeued = Vec::new();
        let now = now_unix_ms();

        for item_key in item_keys {
            if dequeued.len() >= max_items as usize {
                break;
            }

            // Read item
            let item: QueueItem = match self.read_json(&item_key).await? {
                Some(item) => item,
                None => continue, // Item was deleted
            };

            // Skip expired items
            if item.is_expired() {
                let _ = self.delete_key(&item_key).await;
                continue;
            }

            // Check message group ordering - skip if group is currently pending
            if let Some(ref group) = item.message_group_id
                && pending_groups.contains(group)
            {
                debug!(name, item_id = item.item_id, group, "skipping item - message group is pending");
                continue;
            }

            // Check max delivery attempts
            if queue_state.max_delivery_attempts > 0 && item.delivery_attempts >= queue_state.max_delivery_attempts {
                // Move to DLQ
                self.move_to_dlq(name, &item, DLQReason::MaxDeliveryAttemptsExceeded, None).await?;
                let _ = self.delete_key(&item_key).await;
                continue;
            }

            // Generate receipt handle
            let receipt_handle = verified::generate_receipt_handle(item.item_id, now, rand::random::<u64>());

            // Create pending item
            let pending = PendingItem {
                item_id: item.item_id,
                payload: item.payload.clone(),
                consumer_id: consumer_id.to_string(),
                receipt_handle: receipt_handle.clone(),
                dequeued_at_ms: now,
                visibility_deadline_ms: verified::compute_visibility_deadline(now, visibility_timeout_ms),
                delivery_attempts: item.delivery_attempts + 1,
                enqueued_at_ms: item.enqueued_at_ms,
                message_group_id: item.message_group_id.clone(),
            };

            // Try to claim item with CAS
            let pend_key = verified::pending_key(name, item.item_id);
            let pending_json = serde_json::to_string(&pending)?;

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: pend_key.clone(),
                        expected: None,
                        new_value: pending_json,
                    },
                })
                .await
            {
                Ok(_) => {
                    // Successfully claimed - delete from items
                    let _ = self.delete_key(&item_key).await;

                    dequeued.push(DequeuedItem {
                        item_id: item.item_id,
                        payload: item.payload,
                        receipt_handle,
                        delivery_attempts: pending.delivery_attempts,
                        enqueued_at_ms: item.enqueued_at_ms,
                        visibility_deadline_ms: pending.visibility_deadline_ms,
                    });
                }
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    // Another consumer got it, try next
                    continue;
                }
                Err(e) => bail!("dequeue CAS failed: {}", e),
            }
        }

        debug_assert!(dequeued.len() <= max_items as usize, "QUEUE: dequeued count must not exceed max_items");

        debug!(name, consumer_id, count = dequeued.len(), "items dequeued");
        Ok(dequeued)
    }

    /// Dequeue items with blocking wait.
    ///
    /// Polls until items are available or timeout expires.
    pub async fn dequeue_wait(
        &self,
        name: &str,
        consumer_id: &str,
        max_items: u32,
        visibility_timeout_ms: u64,
        wait_timeout_ms: u64,
    ) -> Result<Vec<DequeuedItem>> {
        let deadline = std::time::Instant::now() + Duration::from_millis(wait_timeout_ms);
        let mut poll_interval = DEFAULT_QUEUE_POLL_INTERVAL_MS;

        loop {
            let items = self.dequeue(name, consumer_id, max_items, visibility_timeout_ms).await?;

            if !items.is_empty() {
                return Ok(items);
            }

            if std::time::Instant::now() >= deadline {
                return Ok(vec![]);
            }

            // Wait with exponential backoff
            tokio::time::sleep(Duration::from_millis(poll_interval)).await;
            poll_interval = (poll_interval * 2).min(MAX_QUEUE_POLL_INTERVAL_MS);
        }
    }

    /// Cleanup expired pending items.
    pub(super) async fn cleanup_expired_pending(&self, name: &str) -> Result<u32> {
        let pending_pref = verified::pending_prefix(name);
        let keys = self.scan_keys(&pending_pref, MAX_QUEUE_CLEANUP_BATCH).await?;

        let mut cleaned = 0u32;

        for key in keys {
            if let Some(pending) = self.read_json::<PendingItem>(&key).await?
                && pending.is_visibility_expired()
            {
                // Return to queue using pure function
                let item = verified::create_queue_item_from_pending(&pending, false);

                let i_key = verified::item_key(name, pending.item_id);
                let item_json = serde_json::to_string(&item)?;

                // Try to return item - may fail if already processed
                let _ = self
                    .store
                    .write(WriteRequest {
                        command: WriteCommand::Set {
                            key: i_key,
                            value: item_json,
                        },
                    })
                    .await;

                // Delete pending entry
                let _ = self.delete_key(&key).await;
                cleaned += 1;
            }
        }

        if cleaned > 0 {
            debug!(name, cleaned, "expired pending items cleaned up");
        }

        Ok(cleaned)
    }

    /// Get message groups that currently have pending items.
    pub(super) async fn get_pending_message_groups(&self, name: &str) -> Result<Vec<String>> {
        let pending_pref = verified::pending_prefix(name);
        let keys = self.scan_keys(&pending_pref, MAX_QUEUE_CLEANUP_BATCH).await?;

        let mut groups = Vec::new();
        for key in keys {
            if let Some(pending) = self.read_json::<PendingItem>(&key).await?
                && let Some(group) = pending.message_group_id
                && !groups.contains(&group)
            {
                groups.push(group);
            }
        }

        Ok(groups)
    }
}
