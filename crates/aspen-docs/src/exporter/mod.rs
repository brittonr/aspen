//! DocsExporter - Real-time export of KV operations to iroh-docs.
//!
//! Subscribes to the Raft log subscriber broadcast channel and exports
//! KV operations (Set/Delete) to an iroh-docs namespace in real-time.
//!
//! # Architecture
//!
//! The exporter listens to the log broadcast channel and converts committed
//! KV operations to iroh-docs entries. This enables real-time synchronization
//! of the KV store to clients via iroh-docs CRDT replication.
//!
//! # Features
//!
//! - **Batch export**: Buffers entries and writes them in batches for efficiency
//! - **Background full-sync**: Periodic drift correction by scanning all KV entries

mod batch;
mod core;
mod sync;
mod writers;

pub use writers::BlobBackedDocsWriter;
pub use writers::InMemoryDocsWriter;
pub use writers::IrohDocsWriter;
pub use writers::SyncHandleDocsWriter;

pub use self::core::BatchEntry;
pub use self::core::DocsExporter;
pub use self::core::DocsWriter;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::writers::InMemoryDocsWriter;
    use super::*;
    use crate::constants::MAX_DOC_KEY_SIZE;
    use crate::constants::MAX_DOC_VALUE_SIZE;

    #[tokio::test]
    async fn test_export_set() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        exporter.export_set(b"key1", b"value1", 1).await.expect("export should succeed");

        assert_eq!(writer.get(b"key1").await, Some(b"value1".to_vec()));
    }

    #[tokio::test]
    async fn test_export_delete() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        // Set then delete
        exporter.export_set(b"key1", b"value1", 1).await.expect("export should succeed");
        exporter.export_delete(b"key1", 2).await.expect("delete should succeed");

        // Tombstone = empty value
        assert_eq!(writer.get(b"key1").await, Some(vec![]));
    }

    #[tokio::test]
    async fn test_skip_large_key() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let large_key = vec![0u8; MAX_DOC_KEY_SIZE + 1];
        exporter.export_set(&large_key, b"value", 1).await.expect("should skip without error");

        assert_eq!(writer.len().await, 0);
    }

    #[tokio::test]
    async fn test_skip_large_value() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let large_value = vec![0u8; MAX_DOC_VALUE_SIZE + 1];
        exporter.export_set(b"key1", &large_value, 1).await.expect("should skip without error");

        assert_eq!(writer.len().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_receiver_integration() {
        use aspen_core::hlc::SerializableTimestamp;
        use aspen_raft::log_subscriber::KvOperation;
        use aspen_raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
        use aspen_raft::log_subscriber::LogEntryPayload;
        use tokio::sync::broadcast;

        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        // Create broadcast channel (same as in bootstrap)
        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE as usize);

        // Subscribe and spawn exporter
        let receiver = sender.subscribe();
        let cancel = exporter.clone().spawn(receiver);

        // Create HLC for test timestamps
        let hlc = aspen_core::hlc::create_hlc("test-node");

        // Send a Set operation via broadcast
        let payload = LogEntryPayload {
            index: 1,
            term: 1,
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
            operation: KvOperation::Set {
                key: b"test-key".to_vec(),
                value: b"test-value".to_vec(),
            },
        };
        sender.send(payload).expect("send should succeed");

        // Give the exporter task time to process
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Verify the entry was exported
        assert_eq!(writer.get(b"test-key").await, Some(b"test-value".to_vec()));

        // Cancel the exporter
        cancel.cancel();
    }

    #[tokio::test]
    async fn test_broadcast_multi_operations() {
        use aspen_core::hlc::SerializableTimestamp;
        use aspen_raft::log_subscriber::KvOperation;
        use aspen_raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
        use aspen_raft::log_subscriber::LogEntryPayload;
        use tokio::sync::broadcast;

        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE as usize);
        let receiver = sender.subscribe();
        let cancel = exporter.clone().spawn(receiver);

        // Create HLC for test timestamps
        let hlc = aspen_core::hlc::create_hlc("test-node");

        // Send SetMulti operation
        sender
            .send(LogEntryPayload {
                index: 1,
                term: 1,
                hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                operation: KvOperation::SetMulti {
                    pairs: vec![
                        (b"key1".to_vec(), b"value1".to_vec()),
                        (b"key2".to_vec(), b"value2".to_vec()),
                    ],
                },
            })
            .expect("send should succeed");

        // Send Delete operation
        sender
            .send(LogEntryPayload {
                index: 2,
                term: 1,
                hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                operation: KvOperation::Delete { key: b"key1".to_vec() },
            })
            .expect("send should succeed");

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // key1 should be deleted (tombstone)
        assert_eq!(writer.get(b"key1").await, Some(vec![]));
        // key2 should exist
        assert_eq!(writer.get(b"key2").await, Some(b"value2".to_vec()));

        cancel.cancel();
    }

    #[tokio::test]
    async fn test_broadcast_skips_noop_and_membership() {
        use aspen_core::hlc::SerializableTimestamp;
        use aspen_raft::log_subscriber::KvOperation;
        use aspen_raft::log_subscriber::LOG_BROADCAST_BUFFER_SIZE;
        use aspen_raft::log_subscriber::LogEntryPayload;
        use tokio::sync::broadcast;

        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE as usize);
        let receiver = sender.subscribe();
        let cancel = exporter.clone().spawn(receiver);

        // Create HLC for test timestamps
        let hlc = aspen_core::hlc::create_hlc("test-node");

        // Send Noop (should be skipped)
        sender
            .send(LogEntryPayload {
                index: 1,
                term: 1,
                hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                operation: KvOperation::Noop,
            })
            .expect("send should succeed");

        // Send MembershipChange (should be skipped)
        sender
            .send(LogEntryPayload {
                index: 2,
                term: 1,
                hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                operation: KvOperation::MembershipChange {
                    description: "test change".into(),
                },
            })
            .expect("send should succeed");

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // No entries should be written
        assert!(writer.is_empty().await);

        cancel.cancel();
    }
}
