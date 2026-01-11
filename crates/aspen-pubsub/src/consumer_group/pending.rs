//! Pending entries management for consumer groups.
//!
//! Tracks in-flight messages awaiting acknowledgment. Uses dual indexing for
//! efficient lookup by cursor and by deadline (for expiry scanning).

use std::sync::Arc;

use aspen_core::KeyValueStore;
use async_trait::async_trait;

use crate::consumer_group::constants::MAX_BATCH_ACK_SIZE;
use crate::consumer_group::constants::MAX_PENDING_PER_CONSUMER;
use crate::consumer_group::error::ConsumerGroupError;
use crate::consumer_group::error::Result;
use crate::consumer_group::fencing::validate_fencing;
use crate::consumer_group::receipt::ReceiptHandleComponents;
use crate::consumer_group::receipt::generate_receipt_handle;
use crate::consumer_group::receipt::parse_receipt_handle;
use crate::consumer_group::storage;
use crate::consumer_group::types::AckResult;
use crate::consumer_group::types::BatchAckRequest;
use crate::consumer_group::types::BatchAckResult;
use crate::consumer_group::types::ConsumerGroupId;
use crate::consumer_group::types::ConsumerId;
use crate::consumer_group::types::NackResult;
use crate::consumer_group::types::PartitionId;
use crate::consumer_group::types::PendingEntry;

/// Parameters for marking a message as delivered.
#[derive(Debug, Clone)]
pub struct DeliveryParams<'a> {
    /// Consumer group ID.
    pub group_id: &'a ConsumerGroupId,
    /// Consumer receiving the message.
    pub consumer_id: &'a ConsumerId,
    /// Message cursor (Raft log index).
    pub cursor: u64,
    /// Partition the message belongs to.
    pub partition_id: PartitionId,
    /// Visibility timeout in milliseconds.
    pub visibility_timeout_ms: u64,
    /// Consumer's fencing token.
    pub fencing_token: u64,
    /// Current timestamp in milliseconds.
    pub now_ms: u64,
}

/// Parameters for redelivering an expired message.
#[derive(Debug, Clone)]
pub struct RedeliveryParams<'a> {
    /// Consumer group ID.
    pub group_id: &'a ConsumerGroupId,
    /// Message cursor (Raft log index).
    pub cursor: u64,
    /// New consumer receiving the message.
    pub new_consumer_id: &'a ConsumerId,
    /// Visibility timeout in milliseconds.
    pub visibility_timeout_ms: u64,
    /// New consumer's fencing token.
    pub fencing_token: u64,
    /// Current timestamp in milliseconds.
    pub now_ms: u64,
}

/// Manages pending (in-flight) message entries for consumer groups.
///
/// All operations go through Raft consensus via the KV store's write operations.
/// Uses dual indexing (by cursor and by deadline) for efficient operations.
#[async_trait]
pub trait PendingEntriesManager: Send + Sync {
    /// Mark a message as delivered to a consumer.
    ///
    /// Creates a pending entry with a receipt handle for acknowledgment.
    /// The entry is stored in both the primary pending index (by cursor)
    /// and the deadline index (for expiry scanning).
    ///
    /// # Returns
    ///
    /// The generated receipt handle for acknowledgment.
    ///
    /// # Errors
    ///
    /// - `TooManyPending` if consumer exceeds MAX_PENDING_PER_CONSUMER
    async fn mark_delivered(&self, params: DeliveryParams<'_>) -> Result<String>;

    /// Acknowledge a successfully processed message.
    ///
    /// Removes the pending entry from both indexes atomically.
    async fn ack(&self, group_id: &ConsumerGroupId, receipt_handle: &str) -> Result<AckResult>;

    /// Negative acknowledge a message for redelivery.
    ///
    /// Either requeues the message (removes from pending) or moves it to DLQ
    /// if max attempts exceeded.
    async fn nack(
        &self,
        group_id: &ConsumerGroupId,
        receipt_handle: &str,
        max_delivery_attempts: u32,
    ) -> Result<NackResult>;

    /// Batch acknowledge multiple messages.
    ///
    /// Processes each acknowledgment individually. Partial success is possible.
    async fn batch_ack(&self, group_id: &ConsumerGroupId, request: BatchAckRequest) -> Result<BatchAckResult>;

    /// Find expired pending entries (past visibility deadline).
    ///
    /// Uses the pending_by_deadline index for efficient O(n) scanning.
    async fn find_expired(
        &self,
        group_id: &ConsumerGroupId,
        before_deadline_ms: u64,
        limit: u32,
    ) -> Result<Vec<PendingEntry>>;

    /// Redeliver an expired message.
    ///
    /// Updates the pending entry with new consumer, incremented delivery
    /// attempt, and new visibility deadline.
    ///
    /// # Returns
    ///
    /// New receipt handle for the redelivered message.
    async fn redeliver(&self, params: RedeliveryParams<'_>) -> Result<String>;

    /// Move a message to the dead letter queue.
    ///
    /// Atomically removes from pending indexes and adds to DLQ.
    async fn dead_letter(&self, group_id: &ConsumerGroupId, cursor: u64, reason: &str, now_ms: u64) -> Result<()>;

    /// Get the current pending count for a consumer.
    async fn pending_count(&self, group_id: &ConsumerGroupId, consumer_id: &ConsumerId) -> Result<u32>;

    /// Get a specific pending entry by cursor.
    async fn get_pending(&self, group_id: &ConsumerGroupId, cursor: u64) -> Result<Option<PendingEntry>>;
}

/// Raft-backed implementation of PendingEntriesManager.
pub struct RaftPendingEntriesList<K: KeyValueStore + ?Sized> {
    /// KV store for persistence through Raft.
    store: Arc<K>,
    /// Secret for receipt handle generation.
    receipt_secret: [u8; 32],
}

impl<K: KeyValueStore + ?Sized> RaftPendingEntriesList<K> {
    /// Create a new pending entries list.
    ///
    /// # Arguments
    ///
    /// * `store` - KV store for persistence
    /// * `receipt_secret` - 32-byte secret for receipt handle signing
    pub fn new(store: Arc<K>, receipt_secret: [u8; 32]) -> Self {
        Self { store, receipt_secret }
    }
}

#[async_trait]
impl<K: KeyValueStore + ?Sized + 'static> PendingEntriesManager for RaftPendingEntriesList<K> {
    async fn mark_delivered(&self, params: DeliveryParams<'_>) -> Result<String> {
        let DeliveryParams {
            group_id,
            consumer_id,
            cursor,
            partition_id,
            visibility_timeout_ms,
            fencing_token,
            now_ms,
        } = params;

        // Load consumer state and validate fencing
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;
        validate_fencing(consumer_id, fencing_token, &consumer_state)?;

        // Check pending count limit
        if consumer_state.pending_count >= MAX_PENDING_PER_CONSUMER {
            return Err(ConsumerGroupError::TooManyPending {
                consumer_id: consumer_id.as_str().to_string(),
                count: consumer_state.pending_count,
            });
        }

        // Generate receipt handle
        let visibility_deadline_ms = now_ms + visibility_timeout_ms;
        let components = ReceiptHandleComponents {
            cursor,
            consumer_id: consumer_id.as_str().to_string(),
            fencing_token,
            delivery_attempt: 1,
            delivered_at_ms: now_ms,
        };
        let receipt_handle = generate_receipt_handle(&self.receipt_secret, &components);

        // Create pending entry
        let entry = PendingEntry {
            cursor,
            consumer_id: consumer_id.clone(),
            delivery_attempt: 1,
            delivered_at_ms: now_ms,
            visibility_deadline_ms,
            partition_id,
            receipt_handle: receipt_handle.clone(),
        };

        // Save pending entry to both indexes
        storage::save_pending_entry(&*self.store, group_id, &entry).await?;

        // Update consumer pending count
        let mut updated_state = consumer_state;
        updated_state.pending_count += 1;
        storage::save_consumer_state(&*self.store, group_id, &updated_state).await?;

        Ok(receipt_handle)
    }

    async fn ack(&self, group_id: &ConsumerGroupId, receipt_handle: &str) -> Result<AckResult> {
        // Parse and validate receipt handle
        let components = match parse_receipt_handle(&self.receipt_secret, receipt_handle) {
            Some(c) => c,
            None => return Ok(AckResult::InvalidHandle),
        };

        // Load pending entry
        let entry = match storage::load_pending_entry(&*self.store, group_id, components.cursor).await? {
            Some(e) => e,
            None => return Ok(AckResult::NotFound),
        };

        // Verify receipt handle matches stored entry
        if entry.receipt_handle != receipt_handle {
            return Ok(AckResult::InvalidHandle);
        }

        // Validate fencing - load consumer state
        let consumer_id = ConsumerId::new_unchecked(&components.consumer_id);
        match storage::try_load_consumer_state(&*self.store, group_id, &consumer_id).await? {
            Some(state) => {
                if state.fencing_token != components.fencing_token {
                    return Ok(AckResult::Fenced);
                }

                // Delete pending entry
                storage::delete_pending_entry(&*self.store, group_id, &entry).await?;

                // Update consumer pending count
                let mut updated_state = state;
                updated_state.pending_count = updated_state.pending_count.saturating_sub(1);
                storage::save_consumer_state(&*self.store, group_id, &updated_state).await?;
            }
            None => {
                // Consumer no longer exists, but still delete the pending entry
                storage::delete_pending_entry(&*self.store, group_id, &entry).await?;
            }
        }

        Ok(AckResult::Success)
    }

    async fn nack(
        &self,
        group_id: &ConsumerGroupId,
        receipt_handle: &str,
        max_delivery_attempts: u32,
    ) -> Result<NackResult> {
        // Parse and validate receipt handle
        let components = match parse_receipt_handle(&self.receipt_secret, receipt_handle) {
            Some(c) => c,
            None => return Ok(NackResult::InvalidHandle),
        };

        // Load pending entry
        let entry = match storage::load_pending_entry(&*self.store, group_id, components.cursor).await? {
            Some(e) => e,
            None => return Ok(NackResult::NotFound),
        };

        // Verify receipt handle matches stored entry
        if entry.receipt_handle != receipt_handle {
            return Ok(NackResult::InvalidHandle);
        }

        let now_ms = storage::now_unix_ms();

        // Check if max attempts exceeded
        if max_delivery_attempts > 0 && entry.delivery_attempt >= max_delivery_attempts {
            // Move to DLQ
            storage::move_to_dlq(&*self.store, group_id, &entry, "max_attempts_exceeded", now_ms).await?;

            // Update consumer pending count
            let consumer_id = ConsumerId::new_unchecked(&components.consumer_id);
            if let Some(mut state) = storage::try_load_consumer_state(&*self.store, group_id, &consumer_id).await? {
                state.pending_count = state.pending_count.saturating_sub(1);
                storage::save_consumer_state(&*self.store, group_id, &state).await?;
            }

            return Ok(NackResult::DeadLettered);
        }

        // Just delete the pending entry - it will be picked up for redelivery
        // by the visibility timeout processor when it's not found
        storage::delete_pending_entry(&*self.store, group_id, &entry).await?;

        // Update consumer pending count
        let consumer_id = ConsumerId::new_unchecked(&components.consumer_id);
        if let Some(mut state) = storage::try_load_consumer_state(&*self.store, group_id, &consumer_id).await? {
            state.pending_count = state.pending_count.saturating_sub(1);
            storage::save_consumer_state(&*self.store, group_id, &state).await?;
        }

        Ok(NackResult::Requeued)
    }

    async fn batch_ack(&self, group_id: &ConsumerGroupId, request: BatchAckRequest) -> Result<BatchAckResult> {
        if request.receipt_handles.len() > MAX_BATCH_ACK_SIZE {
            return Err(ConsumerGroupError::AckBatchTooLarge {
                size: request.receipt_handles.len(),
            });
        }

        let mut success_count: u32 = 0;
        let mut failure_count: u32 = 0;
        let mut failures = std::collections::HashMap::new();

        for handle in &request.receipt_handles {
            match self.ack(group_id, handle).await {
                Ok(result) => {
                    if result.is_success() {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                        failures.insert(handle.clone(), result);
                    }
                }
                Err(e) => {
                    failure_count += 1;
                    // Map error to AckResult
                    let result = match e {
                        ConsumerGroupError::FencingTokenMismatch { .. } => AckResult::Fenced,
                        _ => AckResult::InvalidHandle,
                    };
                    failures.insert(handle.clone(), result);
                }
            }
        }

        Ok(BatchAckResult {
            success_count,
            failure_count,
            failures,
        })
    }

    async fn find_expired(
        &self,
        group_id: &ConsumerGroupId,
        before_deadline_ms: u64,
        limit: u32,
    ) -> Result<Vec<PendingEntry>> {
        storage::scan_expired_pending(&*self.store, group_id, before_deadline_ms, limit).await
    }

    async fn redeliver(&self, params: RedeliveryParams<'_>) -> Result<String> {
        let RedeliveryParams {
            group_id,
            cursor,
            new_consumer_id,
            visibility_timeout_ms,
            fencing_token,
            now_ms,
        } = params;

        // Load existing pending entry
        let old_entry = storage::load_pending_entry(&*self.store, group_id, cursor).await?.ok_or_else(|| {
            ConsumerGroupError::Internal {
                message: format!("pending entry not found for cursor {}", cursor),
            }
        })?;

        // Check new consumer pending count
        let consumer_state = storage::load_consumer_state(&*self.store, group_id, new_consumer_id).await?;
        if consumer_state.pending_count >= MAX_PENDING_PER_CONSUMER {
            return Err(ConsumerGroupError::TooManyPending {
                consumer_id: new_consumer_id.as_str().to_string(),
                count: consumer_state.pending_count,
            });
        }

        // Delete old entry
        storage::delete_pending_entry(&*self.store, group_id, &old_entry).await?;

        // Generate new receipt handle
        let new_delivery_attempt = old_entry.delivery_attempt + 1;
        let visibility_deadline_ms = now_ms + visibility_timeout_ms;
        let components = ReceiptHandleComponents {
            cursor,
            consumer_id: new_consumer_id.as_str().to_string(),
            fencing_token,
            delivery_attempt: new_delivery_attempt,
            delivered_at_ms: now_ms,
        };
        let receipt_handle = generate_receipt_handle(&self.receipt_secret, &components);

        // Create new pending entry
        let new_entry = PendingEntry {
            cursor,
            consumer_id: new_consumer_id.clone(),
            delivery_attempt: new_delivery_attempt,
            delivered_at_ms: now_ms,
            visibility_deadline_ms,
            partition_id: old_entry.partition_id,
            receipt_handle: receipt_handle.clone(),
        };

        // Save new pending entry
        storage::save_pending_entry(&*self.store, group_id, &new_entry).await?;

        // Update old consumer pending count (if still exists)
        if let Some(mut state) =
            storage::try_load_consumer_state(&*self.store, group_id, &old_entry.consumer_id).await?
        {
            state.pending_count = state.pending_count.saturating_sub(1);
            storage::save_consumer_state(&*self.store, group_id, &state).await?;
        }

        // Update new consumer pending count
        let mut updated_state = consumer_state;
        updated_state.pending_count += 1;
        storage::save_consumer_state(&*self.store, group_id, &updated_state).await?;

        Ok(receipt_handle)
    }

    async fn dead_letter(&self, group_id: &ConsumerGroupId, cursor: u64, reason: &str, now_ms: u64) -> Result<()> {
        // Load existing pending entry
        let entry = storage::load_pending_entry(&*self.store, group_id, cursor).await?.ok_or_else(|| {
            ConsumerGroupError::Internal {
                message: format!("pending entry not found for cursor {}", cursor),
            }
        })?;

        // Move to DLQ
        storage::move_to_dlq(&*self.store, group_id, &entry, reason, now_ms).await?;

        // Update consumer pending count
        if let Some(mut state) = storage::try_load_consumer_state(&*self.store, group_id, &entry.consumer_id).await? {
            state.pending_count = state.pending_count.saturating_sub(1);
            storage::save_consumer_state(&*self.store, group_id, &state).await?;
        }

        Ok(())
    }

    async fn pending_count(&self, group_id: &ConsumerGroupId, consumer_id: &ConsumerId) -> Result<u32> {
        let state = storage::load_consumer_state(&*self.store, group_id, consumer_id).await?;
        Ok(state.pending_count)
    }

    async fn get_pending(&self, group_id: &ConsumerGroupId, cursor: u64) -> Result<Option<PendingEntry>> {
        storage::load_pending_entry(&*self.store, group_id, cursor).await
    }
}

#[cfg(test)]
mod tests {
    // Integration tests will be in a separate file since they require a KV store
}
