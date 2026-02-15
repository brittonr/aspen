//! Queue enqueue operations.

use anyhow::Result;
use anyhow::bail;
use aspen_constants::coordination::DEFAULT_QUEUE_DEDUP_TTL_MS;
use aspen_constants::coordination::MAX_QUEUE_BATCH_SIZE;
use aspen_constants::coordination::MAX_QUEUE_ITEM_SIZE;
use aspen_constants::coordination::MAX_QUEUE_ITEM_TTL_MS;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use super::EnqueueOptions;
use super::QueueConfig;
use super::QueueManager;
use super::types::DeduplicationEntry;
use super::types::QueueItem;
use crate::sequence::SequenceGenerator;
use crate::types::now_unix_ms;
use crate::verified;

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Enqueue an item.
    ///
    /// Returns the item ID on success.
    pub async fn enqueue(&self, name: &str, payload: Vec<u8>, options: EnqueueOptions) -> Result<u64> {
        // Validate payload size
        if payload.len() > MAX_QUEUE_ITEM_SIZE as usize {
            bail!("payload size {} exceeds max {}", payload.len(), MAX_QUEUE_ITEM_SIZE);
        }

        let queue_key = verified::queue_metadata_key(name);

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
            let dedup_key = verified::dedup_key(name, dedup_id);
            if let Some(entry) = self.read_dedup_entry(&dedup_key).await?
                && !entry.is_expired()
            {
                debug!(name, dedup_id, item_id = entry.item_id, "duplicate detected");
                return Ok(entry.item_id);
            }
        }

        // Generate item ID
        let seq_key = verified::sequence_key(name);
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
        let item_key = verified::item_key(name, item_id);
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
            let dedup_key = verified::dedup_key(name, dedup_id);
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

        debug_assert!(item_id > 0, "QUEUE: enqueued item must have positive ID");
        debug_assert!(item.enqueued_at_ms > 0, "QUEUE: enqueued item must have positive timestamp");

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

        // Verify FIFO ordering: IDs must be monotonically increasing
        debug_assert!(
            item_ids.windows(2).all(|w| w[0] < w[1]),
            "QUEUE: batch enqueue must produce monotonically increasing IDs"
        );
        assert!(
            item_ids.len() <= MAX_QUEUE_BATCH_SIZE as usize,
            "QUEUE: batch result exceeds max batch size: {} > {MAX_QUEUE_BATCH_SIZE}",
            item_ids.len()
        );

        Ok(item_ids)
    }
}
