//! Distributed FIFO queue with visibility timeout and dead letter queue.
//!
//! A queue maintains items that can be enqueued by producers and dequeued by consumers.
//! Items are locked with a visibility timeout during processing and returned to the queue
//! if not acknowledged within the timeout.
//!
//! Features:
//! - FIFO ordering via monotonic item IDs
//! - Visibility timeout for at-least-once delivery
//! - Dead letter queue for failed items
//! - Deduplication support
//! - Message group ordering
//! - TTL expiration for items

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use anyhow::bail;
use aspen_core::CAS_RETRY_INITIAL_BACKOFF_MS;
use aspen_core::CAS_RETRY_MAX_BACKOFF_MS;
use aspen_core::DEFAULT_QUEUE_DEDUP_TTL_MS;
use aspen_core::DEFAULT_QUEUE_POLL_INTERVAL_MS;
use aspen_core::DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::MAX_CAS_RETRIES;
use aspen_core::MAX_QUEUE_BATCH_SIZE;
use aspen_core::MAX_QUEUE_CLEANUP_BATCH;
use aspen_core::MAX_QUEUE_ITEM_SIZE;
use aspen_core::MAX_QUEUE_ITEM_TTL_MS;
use aspen_core::MAX_QUEUE_POLL_INTERVAL_MS;
use aspen_core::MAX_QUEUE_VISIBILITY_TIMEOUT_MS;
use aspen_core::ReadRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use crate::sequence::SequenceGenerator;
use crate::types::now_unix_ms;

/// Queue key prefix.
const QUEUE_PREFIX: &str = "__queue:";

/// Queue metadata state stored at `__queue:{name}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueState {
    /// Queue name.
    pub name: String,
    /// Maximum delivery attempts before moving to DLQ (0 = no limit).
    pub max_delivery_attempts: u32,
    /// Default visibility timeout in milliseconds.
    pub default_visibility_timeout_ms: u64,
    /// Default item TTL in milliseconds (0 = no expiration).
    pub default_ttl_ms: u64,
    /// Creation timestamp (Unix ms).
    pub created_at_ms: u64,
    /// Queue statistics.
    pub stats: QueueStats,
}

impl Default for QueueState {
    fn default() -> Self {
        Self {
            name: String::new(),
            max_delivery_attempts: 3,
            default_visibility_timeout_ms: DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS,
            default_ttl_ms: 0,
            created_at_ms: now_unix_ms(),
            stats: QueueStats::default(),
        }
    }
}

/// Queue statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueStats {
    /// Total items enqueued (monotonic counter).
    pub total_enqueued: u64,
    /// Total items successfully acked.
    pub total_acked: u64,
    /// Total items moved to DLQ.
    pub total_dlq: u64,
}

/// A single item in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    /// Unique item ID (monotonically increasing).
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Time enqueued (Unix ms).
    pub enqueued_at_ms: u64,
    /// Optional expiration deadline (Unix ms). 0 = no expiration.
    pub expires_at_ms: u64,
    /// Number of delivery attempts.
    pub delivery_attempts: u32,
    /// Optional message group ID for FIFO ordering within groups.
    pub message_group_id: Option<String>,
    /// Optional deduplication ID.
    pub deduplication_id: Option<String>,
}

impl QueueItem {
    /// Check if item has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at_ms > 0 && now_unix_ms() > self.expires_at_ms
    }
}

/// A pending item being processed by a consumer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingItem {
    /// Original item ID.
    pub item_id: u64,
    /// Original item payload.
    pub payload: Vec<u8>,
    /// Consumer ID processing this item.
    pub consumer_id: String,
    /// Receipt handle for acknowledgment.
    pub receipt_handle: String,
    /// When this item was dequeued (Unix ms).
    pub dequeued_at_ms: u64,
    /// Visibility timeout deadline (Unix ms).
    pub visibility_deadline_ms: u64,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time.
    pub enqueued_at_ms: u64,
    /// Message group ID if any.
    pub message_group_id: Option<String>,
}

impl PendingItem {
    /// Check if visibility timeout has expired.
    pub fn is_visibility_expired(&self) -> bool {
        now_unix_ms() > self.visibility_deadline_ms
    }
}

/// An item in the dead letter queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLQItem {
    /// Original item ID.
    pub item_id: u64,
    /// Original payload.
    pub payload: Vec<u8>,
    /// Original enqueue time.
    pub enqueued_at_ms: u64,
    /// Delivery attempts before moving to DLQ.
    pub delivery_attempts: u32,
    /// Reason for moving to DLQ.
    pub reason: DLQReason,
    /// Time moved to DLQ (Unix ms).
    pub moved_at_ms: u64,
    /// Last error message (if any).
    pub last_error: Option<String>,
}

/// Reason for moving an item to the dead letter queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DLQReason {
    /// Exceeded max delivery attempts.
    MaxDeliveryAttemptsExceeded,
    /// Explicitly rejected by consumer.
    ExplicitlyRejected,
    /// Item expired while pending.
    ExpiredWhilePending,
}

/// Deduplication entry for exactly-once delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationEntry {
    /// Deduplication ID.
    pub dedup_id: String,
    /// Item ID that was created.
    pub item_id: u64,
    /// Expiration time (Unix ms).
    pub expires_at_ms: u64,
}

/// Configuration for creating a queue.
#[derive(Debug, Clone, Default)]
pub struct QueueConfig {
    /// Default visibility timeout in milliseconds.
    pub default_visibility_timeout_ms: Option<u64>,
    /// Default item TTL in milliseconds (0 = no expiration).
    pub default_ttl_ms: Option<u64>,
    /// Maximum delivery attempts before DLQ (0 = no limit).
    pub max_delivery_attempts: Option<u32>,
}

/// Options for enqueuing an item.
#[derive(Debug, Clone, Default)]
pub struct EnqueueOptions {
    /// Optional TTL in milliseconds (overrides queue default).
    pub ttl_ms: Option<u64>,
    /// Optional message group ID for FIFO ordering.
    pub message_group_id: Option<String>,
    /// Optional deduplication ID.
    pub deduplication_id: Option<String>,
}

/// Result of a dequeue operation.
#[derive(Debug, Clone)]
pub struct DequeuedItem {
    /// Item ID.
    pub item_id: u64,
    /// Item payload.
    pub payload: Vec<u8>,
    /// Receipt handle for ack/nack.
    pub receipt_handle: String,
    /// Number of delivery attempts (including this one).
    pub delivery_attempts: u32,
    /// Original enqueue time (Unix ms).
    pub enqueued_at_ms: u64,
    /// Visibility deadline (Unix ms).
    pub visibility_deadline_ms: u64,
}

/// Queue status information.
#[derive(Debug, Clone, Default)]
pub struct QueueStatus {
    /// Whether the queue exists.
    pub exists: bool,
    /// Approximate number of visible items.
    pub visible_count: u64,
    /// Approximate number of pending items.
    pub pending_count: u64,
    /// Approximate number of DLQ items.
    pub dlq_count: u64,
    /// Total items enqueued.
    pub total_enqueued: u64,
    /// Total items acked.
    pub total_acked: u64,
    /// Total items moved to DLQ.
    pub total_dlq: u64,
}

/// Manager for distributed queue operations.
pub struct QueueManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Create a new queue manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    /// Create or configure a queue.
    ///
    /// Returns (created, queue_state) where created is false if queue already existed.
    pub async fn create(&self, name: &str, config: QueueConfig) -> Result<(bool, QueueState)> {
        let key = format!("{}{}", QUEUE_PREFIX, name);
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let current = self.read_queue_state(&key).await?;

            match current {
                None => {
                    // Create new queue
                    let state = QueueState {
                        name: name.to_string(),
                        max_delivery_attempts: config.max_delivery_attempts.unwrap_or(3),
                        default_visibility_timeout_ms: config
                            .default_visibility_timeout_ms
                            .unwrap_or(DEFAULT_QUEUE_VISIBILITY_TIMEOUT_MS)
                            .min(MAX_QUEUE_VISIBILITY_TIMEOUT_MS),
                        default_ttl_ms: config.default_ttl_ms.unwrap_or(0).min(MAX_QUEUE_ITEM_TTL_MS),
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
        let queue_key = format!("{}{}", QUEUE_PREFIX, name);
        let items_prefix = format!("{}{}:items:", QUEUE_PREFIX, name);
        let pending_prefix = format!("{}{}:pending:", QUEUE_PREFIX, name);
        let dlq_prefix = format!("{}{}:dlq:", QUEUE_PREFIX, name);
        let dedup_prefix = format!("{}{}:dedup:", QUEUE_PREFIX, name);
        let seq_key = format!("{}{}:seq", QUEUE_PREFIX, name);

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

    /// Enqueue an item.
    ///
    /// Returns the item ID on success.
    pub async fn enqueue(&self, name: &str, payload: Vec<u8>, options: EnqueueOptions) -> Result<u64> {
        // Validate payload size
        if payload.len() > MAX_QUEUE_ITEM_SIZE as usize {
            bail!("payload size {} exceeds max {}", payload.len(), MAX_QUEUE_ITEM_SIZE);
        }

        let queue_key = format!("{}{}", QUEUE_PREFIX, name);

        // Ensure queue exists and get config
        let queue_state = match self.read_queue_state(&queue_key).await? {
            Some(state) => state,
            None => {
                // Auto-create queue with defaults
                let (_, state) = self.create(name, QueueConfig::default()).await?;
                state
            }
        };

        // Check deduplication
        if let Some(ref dedup_id) = options.deduplication_id {
            let dedup_key = format!("{}{}:dedup:{}", QUEUE_PREFIX, name, dedup_id);
            if let Some(entry) = self.read_dedup_entry(&dedup_key).await?
                && !entry.is_expired()
            {
                debug!(name, dedup_id, item_id = entry.item_id, "duplicate detected");
                return Ok(entry.item_id);
            }
        }

        // Generate item ID
        let seq_key = format!("{}{}:seq", QUEUE_PREFIX, name);
        let seq_gen = SequenceGenerator::new(self.store.clone(), &seq_key, Default::default());
        let item_id = seq_gen.next().await?;

        let now = now_unix_ms();
        let ttl_ms = options.ttl_ms.unwrap_or(queue_state.default_ttl_ms).min(MAX_QUEUE_ITEM_TTL_MS);
        let expires_at_ms = if ttl_ms > 0 { now + ttl_ms } else { 0 };

        let item = QueueItem {
            item_id,
            payload,
            enqueued_at_ms: now,
            expires_at_ms,
            delivery_attempts: 0,
            message_group_id: options.message_group_id,
            deduplication_id: options.deduplication_id.clone(),
        };

        // Store item
        let item_key = format!("{}{}:items:{:020}", QUEUE_PREFIX, name, item_id);
        let item_json = serde_json::to_string(&item)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: item_key,
                    value: item_json,
                },
            })
            .await?;

        // Store dedup entry if provided
        if let Some(ref dedup_id) = options.deduplication_id {
            let dedup_key = format!("{}{}:dedup:{}", QUEUE_PREFIX, name, dedup_id);
            let dedup_entry = DeduplicationEntry {
                dedup_id: dedup_id.clone(),
                item_id,
                expires_at_ms: now + DEFAULT_QUEUE_DEDUP_TTL_MS,
            };
            let dedup_json = serde_json::to_string(&dedup_entry)?;
            let _ = self
                .store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: dedup_key,
                        value: dedup_json,
                    },
                })
                .await;
        }

        // Update stats
        self.increment_enqueue_count(name).await?;

        debug!(name, item_id, "item enqueued");
        Ok(item_id)
    }

    /// Enqueue multiple items in a batch.
    ///
    /// Returns the list of item IDs on success.
    pub async fn enqueue_batch(&self, name: &str, items: Vec<(Vec<u8>, EnqueueOptions)>) -> Result<Vec<u64>> {
        if items.len() > MAX_QUEUE_BATCH_SIZE as usize {
            bail!("batch size {} exceeds max {}", items.len(), MAX_QUEUE_BATCH_SIZE);
        }

        let mut item_ids = Vec::with_capacity(items.len());
        for (payload, options) in items {
            let item_id = self.enqueue(name, payload, options).await?;
            item_ids.push(item_id);
        }
        Ok(item_ids)
    }

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

        let queue_key = format!("{}{}", QUEUE_PREFIX, name);
        let queue_state = match self.read_queue_state(&queue_key).await? {
            Some(state) => state,
            None => return Ok(vec![]), // Queue doesn't exist
        };

        let items_prefix = format!("{}{}:items:", QUEUE_PREFIX, name);

        // Get list of message groups currently pending (for FIFO ordering)
        let pending_groups = self.get_pending_message_groups(name).await?;

        // Scan for available items
        let item_keys = self.scan_keys(&items_prefix, max_items * 2).await?;

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
            let receipt_handle = format!("{}:{}:{}", item.item_id, now, rand::random::<u64>());

            // Create pending item
            let pending = PendingItem {
                item_id: item.item_id,
                payload: item.payload.clone(),
                consumer_id: consumer_id.to_string(),
                receipt_handle: receipt_handle.clone(),
                dequeued_at_ms: now,
                visibility_deadline_ms: now + visibility_timeout_ms,
                delivery_attempts: item.delivery_attempts + 1,
                enqueued_at_ms: item.enqueued_at_ms,
                message_group_id: item.message_group_id.clone(),
            };

            // Try to claim item with CAS
            let pending_key = format!("{}{}:pending:{:020}", QUEUE_PREFIX, name, item.item_id);
            let pending_json = serde_json::to_string(&pending)?;

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: pending_key.clone(),
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

    /// Peek at items without removing them.
    ///
    /// Returns up to `max_items` items from the front of the queue.
    pub async fn peek(&self, name: &str, max_items: u32) -> Result<Vec<QueueItem>> {
        let max_items = max_items.min(MAX_QUEUE_BATCH_SIZE);
        let items_prefix = format!("{}{}:items:", QUEUE_PREFIX, name);

        let item_keys = self.scan_keys(&items_prefix, max_items).await?;
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

    /// Acknowledge successful processing of an item.
    ///
    /// Deletes the item from the pending queue.
    pub async fn ack(&self, name: &str, receipt_handle: &str) -> Result<()> {
        let item_id = self.parse_receipt_handle(receipt_handle)?;
        let pending_key = format!("{}{}:pending:{:020}", QUEUE_PREFIX, name, item_id);

        // Verify receipt handle matches
        if let Some(pending) = self.read_json::<PendingItem>(&pending_key).await? {
            if pending.receipt_handle != receipt_handle {
                bail!("receipt handle mismatch - item may have been redelivered");
            }

            // Delete pending item
            self.delete_key(&pending_key).await?;

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
        let pending_key = format!("{}{}:pending:{:020}", QUEUE_PREFIX, name, item_id);

        let pending: PendingItem = match self.read_json(&pending_key).await? {
            Some(p) => p,
            None => bail!("item not found or already processed"),
        };

        if pending.receipt_handle != receipt_handle {
            bail!("receipt handle mismatch - item may have been redelivered");
        }

        // Get queue config for max delivery attempts
        let queue_key = format!("{}{}", QUEUE_PREFIX, name);
        let queue_state = self.read_queue_state(&queue_key).await?.unwrap_or_default();

        let should_dlq = move_to_dlq
            || (queue_state.max_delivery_attempts > 0
                && pending.delivery_attempts >= queue_state.max_delivery_attempts);

        if should_dlq {
            // Move to DLQ
            let reason = if move_to_dlq {
                DLQReason::ExplicitlyRejected
            } else {
                DLQReason::MaxDeliveryAttemptsExceeded
            };

            let dlq_item = DLQItem {
                item_id: pending.item_id,
                payload: pending.payload,
                enqueued_at_ms: pending.enqueued_at_ms,
                delivery_attempts: pending.delivery_attempts,
                reason,
                moved_at_ms: now_unix_ms(),
                last_error: error_message,
            };

            let dlq_key = format!("{}{}:dlq:{:020}", QUEUE_PREFIX, name, pending.item_id);
            let dlq_json = serde_json::to_string(&dlq_item)?;

            self.store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: dlq_key,
                        value: dlq_json,
                    },
                })
                .await?;

            // Update DLQ stats
            self.increment_dlq_count(name).await?;

            debug!(name, item_id, "item moved to DLQ");
        } else {
            // Return to queue with incremented delivery attempts
            let item = QueueItem {
                item_id: pending.item_id,
                payload: pending.payload,
                enqueued_at_ms: pending.enqueued_at_ms,
                expires_at_ms: 0, // Reset expiration on retry
                delivery_attempts: pending.delivery_attempts,
                message_group_id: pending.message_group_id,
                deduplication_id: None, // Don't dedupe retries
            };

            let item_key = format!("{}{}:items:{:020}", QUEUE_PREFIX, name, pending.item_id);
            let item_json = serde_json::to_string(&item)?;

            self.store
                .write(WriteRequest {
                    command: WriteCommand::Set {
                        key: item_key,
                        value: item_json,
                    },
                })
                .await?;

            debug!(name, item_id, "item returned to queue");
        }

        // Delete pending item
        self.delete_key(&pending_key).await?;

        Ok(())
    }

    /// Extend the visibility timeout for a pending item.
    ///
    /// Returns the new visibility deadline.
    pub async fn extend_visibility(&self, name: &str, receipt_handle: &str, additional_timeout_ms: u64) -> Result<u64> {
        let additional_timeout_ms = additional_timeout_ms.min(MAX_QUEUE_VISIBILITY_TIMEOUT_MS);
        let item_id = self.parse_receipt_handle(receipt_handle)?;
        let pending_key = format!("{}{}:pending:{:020}", QUEUE_PREFIX, name, item_id);
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            let pending: PendingItem = match self.read_json(&pending_key).await? {
                Some(p) => p,
                None => bail!("item not found or already processed"),
            };

            if pending.receipt_handle != receipt_handle {
                bail!("receipt handle mismatch - item may have been redelivered");
            }

            let new_deadline = now_unix_ms() + additional_timeout_ms;
            let mut new_pending = pending.clone();
            new_pending.visibility_deadline_ms = new_deadline;

            let old_json = serde_json::to_string(&pending)?;
            let new_json = serde_json::to_string(&new_pending)?;

            match self
                .store
                .write(WriteRequest {
                    command: WriteCommand::CompareAndSwap {
                        key: pending_key.clone(),
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

    /// Get queue status.
    pub async fn status(&self, name: &str) -> Result<QueueStatus> {
        let queue_key = format!("{}{}", QUEUE_PREFIX, name);

        let queue_state = match self.read_queue_state(&queue_key).await? {
            Some(state) => state,
            None => return Ok(QueueStatus::default()),
        };

        let items_prefix = format!("{}{}:items:", QUEUE_PREFIX, name);
        let pending_prefix = format!("{}{}:pending:", QUEUE_PREFIX, name);
        let dlq_prefix = format!("{}{}:dlq:", QUEUE_PREFIX, name);

        let visible_keys = self.scan_keys(&items_prefix, MAX_QUEUE_CLEANUP_BATCH).await?;
        let pending_keys = self.scan_keys(&pending_prefix, MAX_QUEUE_CLEANUP_BATCH).await?;
        let dlq_keys = self.scan_keys(&dlq_prefix, MAX_QUEUE_CLEANUP_BATCH).await?;

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

    /// Get items from the dead letter queue.
    pub async fn get_dlq(&self, name: &str, max_items: u32) -> Result<Vec<DLQItem>> {
        let max_items = max_items.min(MAX_QUEUE_BATCH_SIZE);
        let dlq_prefix = format!("{}{}:dlq:", QUEUE_PREFIX, name);

        let keys = self.scan_keys(&dlq_prefix, max_items).await?;
        let mut items = Vec::new();

        for key in keys {
            if let Some(item) = self.read_json::<DLQItem>(&key).await? {
                items.push(item);
            }
        }

        Ok(items)
    }

    /// Move a DLQ item back to the main queue.
    pub async fn redrive_dlq(&self, name: &str, item_id: u64) -> Result<()> {
        let dlq_key = format!("{}{}:dlq:{:020}", QUEUE_PREFIX, name, item_id);

        let dlq_item: DLQItem = match self.read_json(&dlq_key).await? {
            Some(item) => item,
            None => bail!("DLQ item not found"),
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

        let item_key = format!("{}{}:items:{:020}", QUEUE_PREFIX, name, dlq_item.item_id);
        let item_json = serde_json::to_string(&item)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: item_key,
                    value: item_json,
                },
            })
            .await?;

        // Delete from DLQ
        self.delete_key(&dlq_key).await?;

        debug!(name, item_id, "item redriven from DLQ");
        Ok(())
    }

    // =========================================================================
    // Internal helpers
    // =========================================================================

    /// Cleanup expired pending items.
    async fn cleanup_expired_pending(&self, name: &str) -> Result<u32> {
        let pending_prefix = format!("{}{}:pending:", QUEUE_PREFIX, name);
        let keys = self.scan_keys(&pending_prefix, MAX_QUEUE_CLEANUP_BATCH).await?;

        let mut cleaned = 0u32;

        for key in keys {
            if let Some(pending) = self.read_json::<PendingItem>(&key).await?
                && pending.is_visibility_expired()
            {
                // Return to queue
                let item = QueueItem {
                    item_id: pending.item_id,
                    payload: pending.payload,
                    enqueued_at_ms: pending.enqueued_at_ms,
                    expires_at_ms: 0,
                    delivery_attempts: pending.delivery_attempts,
                    message_group_id: pending.message_group_id,
                    deduplication_id: None,
                };

                let item_key = format!("{}{}:items:{:020}", QUEUE_PREFIX, name, pending.item_id);
                let item_json = serde_json::to_string(&item)?;

                // Try to return item - may fail if already processed
                let _ = self
                    .store
                    .write(WriteRequest {
                        command: WriteCommand::Set {
                            key: item_key,
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
    async fn get_pending_message_groups(&self, name: &str) -> Result<Vec<String>> {
        let pending_prefix = format!("{}{}:pending:", QUEUE_PREFIX, name);
        let keys = self.scan_keys(&pending_prefix, MAX_QUEUE_CLEANUP_BATCH).await?;

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

    /// Move an item to the DLQ.
    async fn move_to_dlq(&self, name: &str, item: &QueueItem, reason: DLQReason, error: Option<String>) -> Result<()> {
        let dlq_item = DLQItem {
            item_id: item.item_id,
            payload: item.payload.clone(),
            enqueued_at_ms: item.enqueued_at_ms,
            delivery_attempts: item.delivery_attempts,
            reason,
            moved_at_ms: now_unix_ms(),
            last_error: error,
        };

        let dlq_key = format!("{}{}:dlq:{:020}", QUEUE_PREFIX, name, item.item_id);
        let dlq_json = serde_json::to_string(&dlq_item)?;

        self.store
            .write(WriteRequest {
                command: WriteCommand::Set {
                    key: dlq_key,
                    value: dlq_json,
                },
            })
            .await?;

        self.increment_dlq_count(name).await?;
        Ok(())
    }

    /// Parse item ID from receipt handle.
    fn parse_receipt_handle(&self, receipt_handle: &str) -> Result<u64> {
        let parts: Vec<&str> = receipt_handle.split(':').collect();
        if parts.is_empty() {
            bail!("invalid receipt handle format");
        }
        parts[0].parse().map_err(|_| anyhow::anyhow!("invalid item ID in receipt handle"))
    }

    /// Read queue state.
    async fn read_queue_state(&self, key: &str) -> Result<Option<QueueState>> {
        self.read_json(key).await
    }

    /// Read deduplication entry.
    async fn read_dedup_entry(&self, key: &str) -> Result<Option<DeduplicationEntry>> {
        self.read_json(key).await
    }

    /// Read and deserialize JSON from a key.
    async fn read_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
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
    async fn scan_keys(&self, prefix: &str, limit: u32) -> Result<Vec<String>> {
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
    async fn delete_key(&self, key: &str) -> Result<bool> {
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

    /// Increment enqueue count in queue stats.
    async fn increment_enqueue_count(&self, name: &str) -> Result<()> {
        self.update_queue_stats(name, |stats| stats.total_enqueued += 1).await
    }

    /// Increment ack count in queue stats.
    async fn increment_ack_count(&self, name: &str) -> Result<()> {
        self.update_queue_stats(name, |stats| stats.total_acked += 1).await
    }

    /// Increment DLQ count in queue stats.
    async fn increment_dlq_count(&self, name: &str) -> Result<()> {
        self.update_queue_stats(name, |stats| stats.total_dlq += 1).await
    }

    /// Update queue stats atomically.
    async fn update_queue_stats<F>(&self, name: &str, update_fn: F) -> Result<()>
    where F: Fn(&mut QueueStats) {
        let queue_key = format!("{}{}", QUEUE_PREFIX, name);
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

impl DeduplicationEntry {
    /// Check if entry has expired.
    fn is_expired(&self) -> bool {
        now_unix_ms() > self.expires_at_ms
    }
}

#[cfg(test)]
mod tests {
    use aspen_core::inmemory::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_enqueue_dequeue_single_item() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        // Enqueue
        let item_id = manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();
        assert!(item_id > 0);

        // Dequeue
        let items = manager.dequeue("test", "consumer1", 10, 30000).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, item_id);
        assert_eq!(items[0].payload, b"hello");

        // Ack
        manager.ack("test", &items[0].receipt_handle).await.unwrap();

        // Queue should be empty
        let items = manager.dequeue("test", "consumer1", 10, 30000).await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_fifo_ordering() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        // Enqueue multiple items
        let id1 = manager.enqueue("test", b"first".to_vec(), EnqueueOptions::default()).await.unwrap();
        let id2 = manager.enqueue("test", b"second".to_vec(), EnqueueOptions::default()).await.unwrap();
        let id3 = manager.enqueue("test", b"third".to_vec(), EnqueueOptions::default()).await.unwrap();

        assert!(id1 < id2);
        assert!(id2 < id3);

        // Dequeue - should get in FIFO order
        let items = manager.dequeue("test", "consumer1", 3, 30000).await.unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].item_id, id1);
        assert_eq!(items[1].item_id, id2);
        assert_eq!(items[2].item_id, id3);
    }

    #[tokio::test]
    async fn test_visibility_timeout() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        // Enqueue
        manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();

        // Dequeue with very short timeout (1ms)
        let items = manager.dequeue("test", "consumer1", 10, 1).await.unwrap();
        assert_eq!(items.len(), 1);

        // Wait for visibility timeout
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Should be able to dequeue again after cleanup
        manager.cleanup_expired_pending("test").await.unwrap();
        let items = manager.dequeue("test", "consumer2", 10, 30000).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].delivery_attempts, 2);
    }

    #[tokio::test]
    async fn test_ack_removes_item() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();

        let items = manager.dequeue("test", "consumer1", 10, 30000).await.unwrap();
        manager.ack("test", &items[0].receipt_handle).await.unwrap();

        // Should not be able to ack again
        let result = manager.ack("test", &items[0].receipt_handle).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_nack_returns_to_queue() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();

        let items = manager.dequeue("test", "consumer1", 10, 30000).await.unwrap();
        manager.nack("test", &items[0].receipt_handle, false, None).await.unwrap();

        // Should be able to dequeue again
        let items = manager.dequeue("test", "consumer2", 10, 30000).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].delivery_attempts, 2);
    }

    #[tokio::test]
    async fn test_max_delivery_attempts_moves_to_dlq() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        // Create queue with max 2 delivery attempts
        manager
            .create("test", QueueConfig {
                max_delivery_attempts: Some(2),
                ..Default::default()
            })
            .await
            .unwrap();

        manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();

        // First delivery
        let items = manager.dequeue("test", "c1", 10, 30000).await.unwrap();
        manager.nack("test", &items[0].receipt_handle, false, None).await.unwrap();

        // Second delivery - should move to DLQ
        let items = manager.dequeue("test", "c2", 10, 30000).await.unwrap();
        manager.nack("test", &items[0].receipt_handle, false, None).await.unwrap();

        // Queue should be empty
        let items = manager.dequeue("test", "c3", 10, 30000).await.unwrap();
        assert!(items.is_empty());

        // DLQ should have the item
        let dlq_items = manager.get_dlq("test", 10).await.unwrap();
        assert_eq!(dlq_items.len(), 1);
        assert_eq!(dlq_items[0].payload, b"hello");
    }

    #[tokio::test]
    async fn test_peek_does_not_remove() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();

        // Peek
        let items = manager.peek("test", 10).await.unwrap();
        assert_eq!(items.len(), 1);

        // Peek again - still there
        let items = manager.peek("test", 10).await.unwrap();
        assert_eq!(items.len(), 1);

        // Dequeue - should still get it
        let items = manager.dequeue("test", "consumer1", 10, 30000).await.unwrap();
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn test_queue_status() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        // Create and enqueue
        manager.enqueue("test", b"item1".to_vec(), EnqueueOptions::default()).await.unwrap();
        manager.enqueue("test", b"item2".to_vec(), EnqueueOptions::default()).await.unwrap();

        let status = manager.status("test").await.unwrap();
        assert!(status.exists);
        assert_eq!(status.visible_count, 2);
        assert_eq!(status.pending_count, 0);
        assert_eq!(status.total_enqueued, 2);

        // Dequeue one
        let items = manager.dequeue("test", "c1", 1, 30000).await.unwrap();
        let status = manager.status("test").await.unwrap();
        assert_eq!(status.visible_count, 1);
        assert_eq!(status.pending_count, 1);

        // Ack
        manager.ack("test", &items[0].receipt_handle).await.unwrap();
        let status = manager.status("test").await.unwrap();
        assert_eq!(status.visible_count, 1);
        assert_eq!(status.pending_count, 0);
        assert_eq!(status.total_acked, 1);
    }

    #[tokio::test]
    async fn test_redrive_from_dlq() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();

        // Dequeue and nack to DLQ
        let items = manager.dequeue("test", "c1", 10, 30000).await.unwrap();
        manager.nack("test", &items[0].receipt_handle, true, None).await.unwrap();

        // Verify in DLQ
        let dlq_items = manager.get_dlq("test", 10).await.unwrap();
        assert_eq!(dlq_items.len(), 1);

        // Redrive
        manager.redrive_dlq("test", dlq_items[0].item_id).await.unwrap();

        // DLQ should be empty
        let dlq_items = manager.get_dlq("test", 10).await.unwrap();
        assert!(dlq_items.is_empty());

        // Should be back in queue
        let items = manager.dequeue("test", "c2", 10, 30000).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].delivery_attempts, 1); // Reset
    }

    #[tokio::test]
    async fn test_extend_visibility() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        manager.enqueue("test", b"hello".to_vec(), EnqueueOptions::default()).await.unwrap();

        let items = manager.dequeue("test", "c1", 10, 1000).await.unwrap();
        let original_deadline = items[0].visibility_deadline_ms;

        // Extend visibility
        let new_deadline = manager.extend_visibility("test", &items[0].receipt_handle, 60000).await.unwrap();

        assert!(new_deadline > original_deadline);
    }

    #[tokio::test]
    async fn test_batch_enqueue() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        let items = vec![
            (b"item1".to_vec(), EnqueueOptions::default()),
            (b"item2".to_vec(), EnqueueOptions::default()),
            (b"item3".to_vec(), EnqueueOptions::default()),
        ];

        let ids = manager.enqueue_batch("test", items).await.unwrap();
        assert_eq!(ids.len(), 3);
        assert!(ids[0] < ids[1]);
        assert!(ids[1] < ids[2]);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let manager = QueueManager::new(store);

        let opts = EnqueueOptions {
            deduplication_id: Some("dedup-123".to_string()),
            ..Default::default()
        };

        // First enqueue
        let id1 = manager.enqueue("test", b"hello".to_vec(), opts.clone()).await.unwrap();

        // Second enqueue with same dedup ID should return same ID
        let id2 = manager.enqueue("test", b"hello".to_vec(), opts.clone()).await.unwrap();

        assert_eq!(id1, id2);

        // Only one item in queue
        let items = manager.peek("test", 10).await.unwrap();
        assert_eq!(items.len(), 1);
    }
}
