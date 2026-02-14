//! Dead letter queue operations.

use anyhow::Result;
use aspen_constants::coordination::MAX_QUEUE_BATCH_SIZE;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::DLQItem;
use super::DLQReason;
use super::QueueItem;
use super::QueueManager;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Get items from the dead letter queue.
    pub async fn get_dlq(&self, name: &str, max_items: u32) -> Result<Vec<DLQItem>> {
        let max_items = max_items.min(MAX_QUEUE_BATCH_SIZE);
        let dlq_pref = verified::dlq_prefix(name);

        let keys = self.scan_keys(&dlq_pref, max_items).await?;
        let mut items = Vec::new();

        for key in keys {
            if let Some(item) = self.read_json::<DLQItem>(&key).await? {
                items.push(item);
            }
        }

        Ok(items)
    }

    /// Move a DLQ item back to the main queue.
    ///
    /// Idempotent: returns Ok(()) if the item doesn't exist (already redriven or never in DLQ).
    pub async fn redrive_dlq(&self, name: &str, item_id: u64) -> Result<()> {
        let d_key = verified::dlq_key(name, item_id);

        let dlq_item: DLQItem = match self.read_json(&d_key).await? {
            Some(item) => item,
            None => {
                // Idempotent: item already gone or never existed
                debug!(name, item_id, "DLQ item not found - treating as already redriven");
                return Ok(());
            }
        };

        // Create new queue item with reset delivery attempts
        let item = QueueItem {
            item_id: dlq_item.item_id,
            payload: dlq_item.payload,
            enqueued_at_ms: now_unix_ms(), // Reset enqueue time
            expires_at_ms: 0,
            delivery_attempts: 0, // Reset attempts
            message_group_id: None,
            deduplication_id: None,
        };

        let i_key = verified::item_key(name, dlq_item.item_id);
        let item_json = serde_json::to_string(&item)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: i_key,
                    value: item_json,
                },
            })
            .await?;

        // Delete from DLQ
        self.delete_key(&d_key).await?;

        debug!(name, item_id, "item redriven from DLQ");
        Ok(())
    }

    /// Move an item to the DLQ.
    pub(super) async fn move_to_dlq(
        &self,
        name: &str,
        item: &QueueItem,
        reason: DLQReason,
        error: Option<String>,
    ) -> Result<()> {
        let dlq_item = DLQItem {
            item_id: item.item_id,
            payload: item.payload.clone(),
            enqueued_at_ms: item.enqueued_at_ms,
            delivery_attempts: item.delivery_attempts,
            reason,
            moved_at_ms: now_unix_ms(),
            last_error: error,
        };

        let d_key = verified::dlq_key(name, item.item_id);
        let dlq_json = serde_json::to_string(&dlq_item)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: d_key,
                    value: dlq_json,
                },
            })
            .await?;

        self.increment_dlq_count(name).await?;
        Ok(())
    }
}
