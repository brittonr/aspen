//! Queue acknowledgment and rejection operations.

use anyhow::Result;
use anyhow::bail;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;
use tracing::error;
use tracing::info;

use super::DLQItem;
use super::DLQReason;
use super::PendingItem;
use super::QueueItem;
use super::QueueManager;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Acknowledge successful processing of an item.
    ///
    /// Deletes the item from the pending queue.
    pub async fn ack(&self, name: &str, receipt_handle: &str) -> Result<()> {
        let item_id = self.parse_receipt_handle(receipt_handle)?;
        let pend_key = verified::pending_key(name, item_id);

        // Verify receipt handle matches
        if let Some(pending) = self.read_json::<PendingItem>(&pend_key).await? {
            if pending.receipt_handle != receipt_handle {
                bail!("receipt handle mismatch - item may have been redelivered");
            }

            // Delete pending item
            self.delete_key(&pend_key).await?;

            // Update stats
            self.increment_ack_count(name).await?;

            debug!(name, item_id, "item acked");
            Ok(())
        } else {
            bail!("item not found or already processed");
        }
    }

    /// Negative acknowledge - return item to queue or move to DLQ.
    pub async fn nack(
        &self,
        name: &str,
        receipt_handle: &str,
        move_to_dlq: bool,
        error_message: Option<String>,
    ) -> Result<()> {
        let item_id = self.parse_receipt_handle(receipt_handle)?;
        let pend_key = verified::pending_key(name, item_id);

        let pending: PendingItem = match self.read_json(&pend_key).await? {
            Some(p) => p,
            None => bail!("item not found or already processed"),
        };

        if pending.receipt_handle != receipt_handle {
            bail!("receipt handle mismatch - item may have been redelivered");
        }

        // Get queue config for max delivery attempts
        let queue_key = verified::queue_metadata_key(name);
        let queue_state = self.read_queue_state(&queue_key).await?.unwrap_or_default();

        let should_dlq = move_to_dlq
            || (queue_state.max_delivery_attempts > 0
                && pending.delivery_attempts >= queue_state.max_delivery_attempts);

        if should_dlq {
            self.nack_move_to_dlq(name, item_id, &pending, move_to_dlq, error_message).await?;
        } else {
            self.nack_return_to_queue(name, item_id, &pending).await?;
        }

        // Delete pending item
        self.delete_key(&pend_key).await?;

        Ok(())
    }

    /// Move a nacked item to the dead letter queue.
    async fn nack_move_to_dlq(
        &self,
        name: &str,
        item_id: u64,
        pending: &PendingItem,
        explicit_reject: bool,
        error_message: Option<String>,
    ) -> Result<()> {
        let reason = if explicit_reject {
            DLQReason::ExplicitlyRejected
        } else {
            DLQReason::MaxDeliveryAttemptsExceeded
        };

        let dlq_item = DLQItem {
            item_id: pending.item_id,
            payload: pending.payload.clone(),
            enqueued_at_ms: pending.enqueued_at_ms,
            delivery_attempts: pending.delivery_attempts,
            reason,
            moved_at_ms: now_unix_ms(),
            last_error: error_message,
        };

        let d_key = verified::dlq_key(name, pending.item_id);
        let dlq_json = serde_json::to_string(&dlq_item)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: d_key,
                    value: dlq_json,
                },
            })
            .await?;

        // Update DLQ stats
        self.increment_dlq_count(name).await?;

        debug!(name, item_id, "item moved to DLQ");
        Ok(())
    }

    /// Return a nacked item to the queue for redelivery.
    async fn nack_return_to_queue(&self, name: &str, item_id: u64, pending: &PendingItem) -> Result<()> {
        let item = verified::create_queue_item_from_pending(pending, false);

        let i_key = verified::item_key(name, pending.item_id);
        let item_json = serde_json::to_string(&item)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: i_key,
                    value: item_json,
                },
            })
            .await?;

        debug!(name, item_id, "item returned to queue");
        Ok(())
    }

    /// Release a pending item back to the queue without counting against delivery attempts.
    ///
    /// Use this when a worker dequeued a message but cannot handle it (e.g., job type mismatch).
    /// This decrements delivery_attempts by 1 to cancel out the increment that happened during
    /// dequeue, so the item can be picked up by another worker without exhausting retries.
    ///
    /// This is different from nack() which counts the attempt and may move to DLQ if max
    /// delivery attempts is reached.
    pub async fn release_unchanged(&self, name: &str, receipt_handle: &str) -> Result<()> {
        let item_id = self.parse_receipt_handle(receipt_handle)?;
        let pend_key = verified::pending_key(name, item_id);

        let pending: PendingItem = match self.read_json(&pend_key).await? {
            Some(p) => p,
            None => bail!("item not found or already processed"),
        };

        if pending.receipt_handle != receipt_handle {
            bail!("receipt handle mismatch - item may have been redelivered");
        }

        // Return to queue with delivery_attempts decremented by 1 to cancel the dequeue increment.
        // Use saturating_sub to avoid underflow if delivery_attempts is somehow 0.
        let item = verified::create_queue_item_from_pending(&pending, true);

        let i_key = verified::item_key(name, pending.item_id);
        let item_json = serde_json::to_string(&item)?;

        info!(
            name,
            item_id,
            item_key = i_key,
            delivery_attempts = item.delivery_attempts,
            message_group_id = ?item.message_group_id,
            "release_unchanged: writing item back to queue"
        );

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: i_key.clone(),
                    value: item_json,
                },
            })
            .await?;

        // Verify the item was written
        let verification: Option<QueueItem> = self.read_json(&i_key).await?;
        if verification.is_none() {
            error!(
                name,
                item_id,
                item_key = i_key,
                "release_unchanged: VERIFICATION FAILED - item not found after write!"
            );
        } else {
            info!(name, item_id, item_key = i_key, "release_unchanged: verified item exists after write");
        }

        // Delete pending item
        self.delete_key(&pend_key).await?;

        info!(name, item_id, "item released unchanged back to queue (pending deleted)");

        Ok(())
    }
}
