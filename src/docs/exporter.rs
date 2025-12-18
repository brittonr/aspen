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
//! # TODO
//!
//! - Add background full-sync for drift correction
//! - Implement batch export for efficiency

use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use iroh_docs::Author;
use iroh_docs::store::Store;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument, warn};

use crate::raft::log_subscriber::{KvOperation, LogEntryPayload};

use super::constants::{EXPORT_BATCH_SIZE, MAX_DOC_KEY_SIZE, MAX_DOC_VALUE_SIZE};

/// Trait for writing entries to a docs namespace.
///
/// This abstraction allows for easy testing and decoupling from
/// the concrete iroh-docs implementation.
#[async_trait]
pub trait DocsWriter: Send + Sync {
    /// Set an entry in the docs namespace.
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()>;

    /// Delete an entry (set to empty value for tombstone).
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()>;
}

/// Exports KV operations to an iroh-docs namespace.
///
/// Listens to the log subscriber broadcast channel and converts
/// `KvOperation` events to docs entries in real-time.
pub struct DocsExporter<W: DocsWriter> {
    /// The docs writer implementation.
    writer: Arc<W>,
    /// Cancellation token for shutdown.
    cancel: CancellationToken,
}

impl<W: DocsWriter + 'static> DocsExporter<W> {
    /// Create a new DocsExporter.
    ///
    /// # Arguments
    /// * `writer` - Implementation of DocsWriter for actual entry operations
    pub fn new(writer: Arc<W>) -> Self {
        Self {
            writer,
            cancel: CancellationToken::new(),
        }
    }

    /// Start exporting from a log subscriber broadcast channel.
    ///
    /// Spawns a background task that listens to the broadcast channel
    /// and exports KV operations to the docs namespace.
    pub fn spawn(
        self: Arc<Self>,
        mut receiver: broadcast::Receiver<LogEntryPayload>,
    ) -> CancellationToken {
        let cancel = self.cancel.clone();
        let exporter = self.clone();

        tokio::spawn(async move {
            info!("DocsExporter started");

            loop {
                tokio::select! {
                    _ = exporter.cancel.cancelled() => {
                        info!("DocsExporter shutting down");
                        break;
                    }
                    result = receiver.recv() => {
                        match result {
                            Ok(payload) => {
                                if let Err(e) = exporter.process_payload(payload).await {
                                    error!(error = %e, "failed to process log entry");
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                warn!("log subscriber channel closed");
                                break;
                            }
                            Err(broadcast::error::RecvError::Lagged(count)) => {
                                warn!(lagged = count, "DocsExporter lagged behind");
                                // Continue processing - we'll catch up
                            }
                        }
                    }
                }
            }
        });

        cancel
    }

    /// Process a single log entry payload.
    async fn process_payload(&self, payload: LogEntryPayload) -> Result<()> {
        match &payload.operation {
            KvOperation::Set { key, value } => {
                self.export_set(key, value, payload.index).await?;
            }
            KvOperation::SetMulti { pairs } => {
                for (key, value) in pairs {
                    self.export_set(key, value, payload.index).await?;
                }
            }
            KvOperation::Delete { key } => {
                self.export_delete(key, payload.index).await?;
            }
            KvOperation::DeleteMulti { keys } => {
                for key in keys {
                    self.export_delete(key, payload.index).await?;
                }
            }
            KvOperation::Noop | KvOperation::MembershipChange { .. } => {
                // Skip non-KV operations
                debug!(log_index = payload.index, "skipping non-KV entry");
            }
        }
        Ok(())
    }

    /// Export a Set operation to docs.
    async fn export_set(&self, key: &[u8], value: &[u8], log_index: u64) -> Result<()> {
        // Validate sizes
        if key.len() > MAX_DOC_KEY_SIZE {
            warn!(
                key_len = key.len(),
                max = MAX_DOC_KEY_SIZE,
                "key too large for docs export, skipping"
            );
            return Ok(());
        }
        if value.len() > MAX_DOC_VALUE_SIZE {
            warn!(
                value_len = value.len(),
                max = MAX_DOC_VALUE_SIZE,
                "value too large for docs export, skipping"
            );
            return Ok(());
        }

        self.writer.set_entry(key.to_vec(), value.to_vec()).await?;

        debug!(log_index, key_len = key.len(), "exported Set to docs");
        Ok(())
    }

    /// Export a Delete operation to docs.
    async fn export_delete(&self, key: &[u8], log_index: u64) -> Result<()> {
        self.writer.delete_entry(key.to_vec()).await?;

        debug!(log_index, key_len = key.len(), "exported Delete to docs");
        Ok(())
    }

    /// Perform a full sync from the state machine to docs.
    ///
    /// Used for initial population and drift correction.
    /// Scans all keys in the KV store and exports them to docs.
    #[allow(dead_code)]
    pub async fn full_sync<F, Fut>(&self, scan_fn: F) -> Result<u64>
    where
        F: Fn(Option<String>, u32) -> Fut,
        Fut: std::future::Future<Output = Result<Vec<(String, String)>>>,
    {
        info!("starting full sync");

        let mut exported = 0u64;
        let mut continuation: Option<String> = None;

        loop {
            // Scan a batch of keys
            let entries = scan_fn(continuation.clone(), EXPORT_BATCH_SIZE).await?;

            if entries.is_empty() {
                break;
            }

            // Export each entry
            for (key, value) in &entries {
                // Skip if too large
                if key.len() > MAX_DOC_KEY_SIZE || value.len() > MAX_DOC_VALUE_SIZE {
                    continue;
                }

                self.writer
                    .set_entry(key.as_bytes().to_vec(), value.as_bytes().to_vec())
                    .await?;

                exported += 1;
            }

            // Update continuation for next batch
            if entries.len() < EXPORT_BATCH_SIZE as usize {
                break;
            }
            continuation = entries.last().map(|(k, _)| k.clone());
        }

        info!(exported, "full sync complete");

        Ok(exported)
    }

    /// Shutdown the exporter.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}

// ============================================================================
// Concrete iroh-docs Implementation
// ============================================================================

/// Concrete implementation of `DocsWriter` using iroh-docs.
///
/// Uses a `Store` and `Author` to write entries to an iroh-docs namespace.
/// The store must have a replica with write capability for the namespace.
///
/// # Example
///
/// ```ignore
/// use aspen::docs::IrohDocsWriter;
/// use iroh_docs::store::Store;
/// use iroh_docs::Author;
///
/// // Create store and get namespace
/// let store = Store::memory();
/// let namespace_id = /* ... */;
/// let author = Author::new(/* ... */);
///
/// let writer = IrohDocsWriter::new(store, namespace_id, author);
/// writer.set_entry(b"key".to_vec(), b"value".to_vec()).await?;
/// ```
pub struct IrohDocsWriter {
    /// The iroh-docs store (wrapped in Mutex for interior mutability).
    store: tokio::sync::Mutex<Store>,
    /// Namespace ID for the document.
    namespace_id: iroh_docs::NamespaceId,
    /// Author for signing entries.
    author: Author,
}

impl IrohDocsWriter {
    /// Create a new IrohDocsWriter.
    ///
    /// # Arguments
    /// * `store` - The iroh-docs store
    /// * `namespace_id` - The namespace ID for the document
    /// * `author` - The author for signing entries
    pub fn new(store: Store, namespace_id: iroh_docs::NamespaceId, author: Author) -> Self {
        Self {
            store: tokio::sync::Mutex::new(store),
            namespace_id,
            author,
        }
    }

    /// Get the namespace ID of this writer.
    pub fn namespace_id(&self) -> iroh_docs::NamespaceId {
        self.namespace_id
    }

    /// Get the author of this writer.
    pub fn author(&self) -> &Author {
        &self.author
    }
}

#[async_trait]
impl DocsWriter for IrohDocsWriter {
    #[instrument(skip(self, value), fields(key_len = key.len(), value_len = value.len()))]
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // Lock the store for writing
        let mut store = self.store.lock().await;

        // Open the replica for writing
        let mut replica = store
            .open_replica(&self.namespace_id)
            .context("failed to open replica")?;

        // hash_and_insert computes the BLAKE3 hash and inserts the entry
        // Note: This stores the entry metadata (hash, size, timestamp) in the replica,
        // but the actual content data needs to be stored separately in a blob store.
        let _hash = replica
            .hash_and_insert(&key, &self.author, &value)
            .context("failed to insert entry into replica")?;

        debug!(namespace = %self.namespace_id, "entry inserted into docs");
        Ok(())
    }

    #[instrument(skip(self), fields(key_len = key.len()))]
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // Lock the store for writing
        let mut store = self.store.lock().await;

        // Open the replica for writing
        let mut replica = store
            .open_replica(&self.namespace_id)
            .context("failed to open replica")?;

        // In iroh-docs, deletion is done by inserting an empty entry as a tombstone.
        // The delete_prefix method does this by inserting an empty entry at the key.
        // Note: delete_prefix signature is (prefix, author)
        let _deleted_count = replica
            .delete_prefix(&key, &self.author)
            .context("failed to delete entry from replica")?;

        debug!(namespace = %self.namespace_id, "entry deleted from docs");
        Ok(())
    }
}

/// In-memory docs writer for development and testing.
///
/// Stores entries in a thread-safe HashMap. Useful when:
/// - No iroh-docs store is available yet
/// - Testing DocsExporter integration
/// - Development environments without persistent storage
///
/// In production, use `IrohDocsWriter` for actual iroh-docs CRDT sync.
pub struct InMemoryDocsWriter {
    entries: tokio::sync::RwLock<std::collections::HashMap<Vec<u8>, Vec<u8>>>,
}

impl Default for InMemoryDocsWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDocsWriter {
    /// Create a new in-memory docs writer.
    pub fn new() -> Self {
        Self {
            entries: tokio::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get an entry by key.
    pub async fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.entries.read().await.get(key).cloned()
    }

    /// Get the number of entries.
    pub async fn len(&self) -> usize {
        self.entries.read().await.len()
    }

    /// Check if the writer is empty.
    pub async fn is_empty(&self) -> bool {
        self.entries.read().await.is_empty()
    }
}

#[async_trait]
impl DocsWriter for InMemoryDocsWriter {
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        self.entries.write().await.insert(key, value);
        Ok(())
    }

    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // Use empty value as tombstone (like iroh-docs)
        self.entries.write().await.insert(key, vec![]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::InMemoryDocsWriter;
    use super::*;

    #[tokio::test]
    async fn test_export_set() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        exporter
            .export_set(b"key1", b"value1", 1)
            .await
            .expect("export should succeed");

        assert_eq!(writer.get(b"key1").await, Some(b"value1".to_vec()));
    }

    #[tokio::test]
    async fn test_export_delete() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        // Set then delete
        exporter
            .export_set(b"key1", b"value1", 1)
            .await
            .expect("export should succeed");
        exporter
            .export_delete(b"key1", 2)
            .await
            .expect("delete should succeed");

        // Tombstone = empty value
        assert_eq!(writer.get(b"key1").await, Some(vec![]));
    }

    #[tokio::test]
    async fn test_skip_large_key() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let large_key = vec![0u8; MAX_DOC_KEY_SIZE + 1];
        exporter
            .export_set(&large_key, b"value", 1)
            .await
            .expect("should skip without error");

        assert_eq!(writer.len().await, 0);
    }

    #[tokio::test]
    async fn test_skip_large_value() {
        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let large_value = vec![0u8; MAX_DOC_VALUE_SIZE + 1];
        exporter
            .export_set(b"key1", &large_value, 1)
            .await
            .expect("should skip without error");

        assert_eq!(writer.len().await, 0);
    }

    #[tokio::test]
    async fn test_broadcast_receiver_integration() {
        use crate::raft::log_subscriber::{
            KvOperation, LOG_BROADCAST_BUFFER_SIZE, LogEntryPayload,
        };
        use tokio::sync::broadcast;

        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        // Create broadcast channel (same as in bootstrap)
        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);

        // Subscribe and spawn exporter
        let receiver = sender.subscribe();
        let cancel = exporter.clone().spawn(receiver);

        // Send a Set operation via broadcast
        let payload = LogEntryPayload {
            index: 1,
            term: 1,
            committed_at_ms: 12345,
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
        use crate::raft::log_subscriber::{
            KvOperation, LOG_BROADCAST_BUFFER_SIZE, LogEntryPayload,
        };
        use tokio::sync::broadcast;

        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);
        let receiver = sender.subscribe();
        let cancel = exporter.clone().spawn(receiver);

        // Send SetMulti operation
        sender
            .send(LogEntryPayload {
                index: 1,
                term: 1,
                committed_at_ms: 12345,
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
                committed_at_ms: 12346,
                operation: KvOperation::Delete {
                    key: b"key1".to_vec(),
                },
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
        use crate::raft::log_subscriber::{
            KvOperation, LOG_BROADCAST_BUFFER_SIZE, LogEntryPayload,
        };
        use tokio::sync::broadcast;

        let writer = Arc::new(InMemoryDocsWriter::new());
        let exporter = Arc::new(DocsExporter::new(writer.clone()));

        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);
        let receiver = sender.subscribe();
        let cancel = exporter.clone().spawn(receiver);

        // Send Noop (should be skipped)
        sender
            .send(LogEntryPayload {
                index: 1,
                term: 1,
                committed_at_ms: 12345,
                operation: KvOperation::Noop,
            })
            .expect("send should succeed");

        // Send MembershipChange (should be skipped)
        sender
            .send(LogEntryPayload {
                index: 2,
                term: 1,
                committed_at_ms: 12346,
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
