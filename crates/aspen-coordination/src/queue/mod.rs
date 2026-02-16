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

mod ack_nack;
mod create_delete;
mod dequeue;
mod dlq;
mod enqueue;
mod helpers;
mod stats;
mod types;
mod visibility;

use std::sync::Arc;

use aspen_traits::KeyValueStore;
pub use types::DLQItem;
pub use types::DLQReason;
pub use types::DequeuedItem;
pub use types::EnqueueOptions;
pub use types::PendingItem;
pub use types::QueueConfig;
pub use types::QueueItem;
pub use types::QueueState;
pub use types::QueueStats;
pub use types::QueueStatus;

/// Manager for distributed queue operations.
pub struct QueueManager<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> QueueManager<S> {
    /// Create a new queue manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use aspen_testing::DeterministicKeyValueStore;

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
        assert!(status.does_exist);
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
