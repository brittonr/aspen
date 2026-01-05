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

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_blob::store::BlobStore;
use aspen_core::KeyValueStore;
use aspen_core::ScanRequest;
use aspen_raft::log_subscriber::KvOperation;
use aspen_raft::log_subscriber::LogEntryPayload;
use async_trait::async_trait;
use iroh_docs::Author;
use iroh_docs::store::Store;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::constants::BACKGROUND_SYNC_INTERVAL;
use super::constants::EXPORT_BATCH_SIZE;
use super::constants::MAX_DOC_KEY_SIZE;
use super::constants::MAX_DOC_VALUE_SIZE;

/// Maximum time to buffer entries before forcing a flush.
/// Tiger Style: Bounded latency for export operations.
const BATCH_FLUSH_INTERVAL: Duration = Duration::from_millis(100);

/// A batch entry for efficient bulk writes.
#[derive(Debug, Clone)]
pub struct BatchEntry {
    /// The key to write.
    pub key: Vec<u8>,
    /// The value to write (empty for tombstone/delete).
    pub value: Vec<u8>,
    /// True if this is a delete operation (tombstone).
    pub is_delete: bool,
}

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

    /// Write a batch of entries for efficiency.
    ///
    /// Default implementation calls set_entry/delete_entry for each entry.
    /// Implementations can override for more efficient bulk operations.
    async fn write_batch(&self, entries: Vec<BatchEntry>) -> Result<()> {
        for entry in entries {
            if entry.is_delete {
                self.delete_entry(entry.key).await?;
            } else {
                self.set_entry(entry.key, entry.value).await?;
            }
        }
        Ok(())
    }
}

/// Exports KV operations to an iroh-docs namespace.
///
/// Listens to the log subscriber broadcast channel and converts
/// `KvOperation` events to docs entries in real-time.
pub struct DocsExporter {
    /// The docs writer implementation.
    writer: Arc<dyn DocsWriter>,
    /// Cancellation token for shutdown.
    cancel: CancellationToken,
}

impl DocsExporter {
    /// Create a new DocsExporter.
    ///
    /// # Arguments
    /// * `writer` - Implementation of DocsWriter for actual entry operations
    pub fn new(writer: Arc<dyn DocsWriter>) -> Self {
        Self {
            writer,
            cancel: CancellationToken::new(),
        }
    }

    /// Start exporting from a log subscriber broadcast channel with batching.
    ///
    /// Spawns a background task that listens to the broadcast channel
    /// and exports KV operations to the docs namespace in batches for efficiency.
    ///
    /// # Batching Behavior
    ///
    /// - Entries are buffered until EXPORT_BATCH_SIZE is reached or BATCH_FLUSH_INTERVAL elapses
    /// - On shutdown, any remaining buffered entries are flushed
    pub fn spawn(self: Arc<Self>, mut receiver: broadcast::Receiver<LogEntryPayload>) -> CancellationToken {
        let cancel = self.cancel.clone();
        let exporter = self.clone();

        tokio::spawn(async move {
            info!(
                batch_size = EXPORT_BATCH_SIZE,
                flush_interval_ms = BATCH_FLUSH_INTERVAL.as_millis(),
                "DocsExporter started with batching"
            );

            let mut batch: Vec<BatchEntry> = Vec::with_capacity(EXPORT_BATCH_SIZE as usize);
            let mut flush_interval = tokio::time::interval(BATCH_FLUSH_INTERVAL);
            flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = exporter.cancel.cancelled() => {
                        // Flush remaining entries on shutdown
                        if !batch.is_empty()
                            && let Err(e) = exporter.flush_batch(&mut batch).await
                        {
                            error!(error = %e, "failed to flush batch on shutdown");
                        }
                        info!("DocsExporter shutting down");
                        break;
                    }
                    _ = flush_interval.tick() => {
                        // Time-based flush to bound latency
                        if !batch.is_empty()
                            && let Err(e) = exporter.flush_batch(&mut batch).await
                        {
                            error!(error = %e, "failed to flush batch on interval");
                        }
                    }
                    result = receiver.recv() => {
                        match result {
                            Ok(payload) => {
                                exporter.collect_payload_to_batch(payload, &mut batch);

                                // Size-based flush when batch is full
                                if batch.len() >= EXPORT_BATCH_SIZE as usize
                                    && let Err(e) = exporter.flush_batch(&mut batch).await
                                {
                                    error!(error = %e, "failed to flush full batch");
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                // Flush remaining entries before exit
                                if !batch.is_empty()
                                    && let Err(e) = exporter.flush_batch(&mut batch).await
                                {
                                    error!(error = %e, "failed to flush batch on channel close");
                                }
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

    /// Start exporting with periodic background full-sync for drift correction.
    ///
    /// In addition to real-time export, this spawns a background task that
    /// periodically scans all KV entries and exports them to ensure consistency.
    /// This catches any entries that may have been missed due to lag or restarts.
    ///
    /// # Arguments
    /// * `receiver` - Broadcast receiver for real-time log entries
    /// * `kv_store` - Reference to the KV store for scanning all entries
    pub fn spawn_with_full_sync<KV>(
        self: Arc<Self>,
        receiver: broadcast::Receiver<LogEntryPayload>,
        kv_store: Arc<KV>,
    ) -> CancellationToken
    where
        KV: KeyValueStore + 'static,
    {
        // Spawn the real-time exporter with batching
        let cancel = self.clone().spawn(receiver);

        // Spawn background full-sync task
        let exporter = self.clone();
        let cancel_clone = self.cancel.clone();

        tokio::spawn(async move {
            info!(interval_secs = BACKGROUND_SYNC_INTERVAL.as_secs(), "background full-sync started");

            let mut sync_interval = tokio::time::interval(BACKGROUND_SYNC_INTERVAL);
            sync_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            // Skip the first immediate tick
            sync_interval.tick().await;

            loop {
                tokio::select! {
                    _ = cancel_clone.cancelled() => {
                        info!("background full-sync shutting down");
                        break;
                    }
                    _ = sync_interval.tick() => {
                        info!("starting periodic full-sync");
                        match exporter.full_sync_from_kv(&kv_store).await {
                            Ok(count) => {
                                info!(exported = count, "periodic full-sync completed");
                            }
                            Err(e) => {
                                error!(error = %e, "periodic full-sync failed");
                            }
                        }
                    }
                }
            }
        });

        cancel
    }

    /// Process a single log entry payload (non-batched, for testing).
    #[allow(dead_code)]
    async fn process_payload(&self, payload: LogEntryPayload) -> Result<()> {
        match &payload.operation {
            KvOperation::Set { key, value }
            | KvOperation::SetWithTTL { key, value, .. }
            | KvOperation::SetWithLease { key, value, .. } => {
                self.export_set(key, value, payload.index).await?;
            }
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => {
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
            KvOperation::CompareAndSwap { key, new_value, .. } => {
                // CAS is a conditional set - export the new value if the operation succeeded
                self.export_set(key, new_value, payload.index).await?;
            }
            KvOperation::CompareAndDelete { key, .. } => {
                // CAS delete - export the deletion if the operation succeeded
                self.export_delete(key, payload.index).await?;
            }
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                // Process each operation in the batch
                for (is_set, key, value) in operations {
                    if *is_set {
                        self.export_set(key, value, payload.index).await?;
                    } else {
                        self.export_delete(key, payload.index).await?;
                    }
                }
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => {
                // Skip non-KV operations
                debug!(log_index = payload.index, "skipping non-KV entry");
            }
            KvOperation::Transaction { success, failure, .. } => {
                // Process all put/delete operations from both branches
                // Note: Only one branch executes at runtime, but log subscriber
                // doesn't track which succeeded, so we export both conservatively
                let process_ops = |ops: &[(u8, Vec<u8>, Vec<u8>)]| {
                    for (op_type, key, value) in ops {
                        match *op_type {
                            0 => { /* Put */ }
                            1 => { /* Delete */ }
                            _ => { /* Get/Range - no export needed */ }
                        }
                        // For now, skip transaction ops in single-entry path
                        // They'll be handled via batch processing
                        let _ = (key, value);
                    }
                };
                process_ops(success);
                process_ops(failure);
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                // Process all write operations
                for (is_set, key, value) in write_set {
                    if *is_set {
                        self.export_set(key, value, payload.index).await?;
                    } else {
                        self.export_delete(key, payload.index).await?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Collect entries from a log payload into the batch buffer.
    ///
    /// Filters out entries that exceed size limits.
    fn collect_payload_to_batch(&self, payload: LogEntryPayload, batch: &mut Vec<BatchEntry>) {
        match payload.operation {
            KvOperation::Set { key, value }
            | KvOperation::SetWithTTL { key, value, .. }
            | KvOperation::SetWithLease { key, value, .. } => {
                if key.len() <= MAX_DOC_KEY_SIZE && value.len() <= MAX_DOC_VALUE_SIZE {
                    batch.push(BatchEntry {
                        key,
                        value,
                        is_delete: false,
                    });
                } else {
                    warn!(key_len = key.len(), value_len = value.len(), "entry too large for docs export, skipping");
                }
            }
            KvOperation::SetMulti { pairs }
            | KvOperation::SetMultiWithTTL { pairs, .. }
            | KvOperation::SetMultiWithLease { pairs, .. } => {
                for (key, value) in pairs {
                    if key.len() <= MAX_DOC_KEY_SIZE && value.len() <= MAX_DOC_VALUE_SIZE {
                        batch.push(BatchEntry {
                            key,
                            value,
                            is_delete: false,
                        });
                    }
                }
            }
            KvOperation::Delete { key } => {
                batch.push(BatchEntry {
                    key,
                    value: vec![],
                    is_delete: true,
                });
            }
            KvOperation::DeleteMulti { keys } => {
                for key in keys {
                    batch.push(BatchEntry {
                        key,
                        value: vec![],
                        is_delete: true,
                    });
                }
            }
            KvOperation::CompareAndSwap { key, new_value, .. } => {
                // CAS is a conditional set - export the new value
                if key.len() <= MAX_DOC_KEY_SIZE && new_value.len() <= MAX_DOC_VALUE_SIZE {
                    batch.push(BatchEntry {
                        key,
                        value: new_value,
                        is_delete: false,
                    });
                }
            }
            KvOperation::CompareAndDelete { key, .. } => {
                // CAS delete - export the deletion
                batch.push(BatchEntry {
                    key,
                    value: vec![],
                    is_delete: true,
                });
            }
            KvOperation::Batch { operations } | KvOperation::ConditionalBatch { operations, .. } => {
                // Process each operation in the batch
                for (is_set, key, value) in operations {
                    if is_set {
                        if key.len() <= MAX_DOC_KEY_SIZE && value.len() <= MAX_DOC_VALUE_SIZE {
                            batch.push(BatchEntry {
                                key,
                                value,
                                is_delete: false,
                            });
                        }
                    } else {
                        batch.push(BatchEntry {
                            key,
                            value: vec![],
                            is_delete: true,
                        });
                    }
                }
            }
            KvOperation::Noop
            | KvOperation::MembershipChange { .. }
            | KvOperation::LeaseGrant { .. }
            | KvOperation::LeaseRevoke { .. }
            | KvOperation::LeaseKeepalive { .. } => {
                // Skip non-KV operations
            }
            KvOperation::Transaction { success, failure, .. } => {
                // Process put/delete ops from transaction branches
                for (op_type, key, value) in success.into_iter().chain(failure.into_iter()) {
                    match op_type {
                        0 => {
                            // Put
                            if key.len() <= MAX_DOC_KEY_SIZE && value.len() <= MAX_DOC_VALUE_SIZE {
                                batch.push(BatchEntry {
                                    key,
                                    value,
                                    is_delete: false,
                                });
                            }
                        }
                        1 => {
                            // Delete
                            batch.push(BatchEntry {
                                key,
                                value: vec![],
                                is_delete: true,
                            });
                        }
                        _ => {
                            // Get/Range - no export needed
                        }
                    }
                }
            }
            KvOperation::OptimisticTransaction { write_set, .. } => {
                // Process all write operations
                for (is_set, key, value) in write_set {
                    if is_set {
                        if key.len() <= MAX_DOC_KEY_SIZE && value.len() <= MAX_DOC_VALUE_SIZE {
                            batch.push(BatchEntry {
                                key,
                                value,
                                is_delete: false,
                            });
                        }
                    } else {
                        batch.push(BatchEntry {
                            key,
                            value: vec![],
                            is_delete: true,
                        });
                    }
                }
            }
        }
    }

    /// Flush the batch buffer to the docs writer.
    async fn flush_batch(&self, batch: &mut Vec<BatchEntry>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let count = batch.len();
        debug!(count, "flushing export batch");

        // Take ownership of batch contents
        let entries = std::mem::take(batch);

        self.writer.write_batch(entries).await?;

        debug!(count, "batch flushed successfully");
        Ok(())
    }

    /// Perform a full sync from a KeyValueStore implementation.
    ///
    /// Scans all keys in the KV store and exports them to docs.
    /// Used for initial population and periodic drift correction.
    pub async fn full_sync_from_kv<KV>(&self, kv_store: &KV) -> Result<u64>
    where
        KV: KeyValueStore,
    {
        info!("starting full sync from KV store");

        let mut exported = 0u64;
        let mut continuation_token: Option<String> = None;

        loop {
            // Scan a batch of keys
            let result = kv_store
                .scan(ScanRequest {
                    prefix: String::new(), // All keys
                    limit: Some(EXPORT_BATCH_SIZE),
                    continuation_token: continuation_token.clone(),
                })
                .await
                .context("failed to scan KV store")?;

            if result.entries.is_empty() {
                break;
            }

            // Collect entries for batch write
            let batch_entries: Vec<BatchEntry> = result
                .entries
                .iter()
                .filter(|entry| entry.key.len() <= MAX_DOC_KEY_SIZE && entry.value.len() <= MAX_DOC_VALUE_SIZE)
                .map(|entry| BatchEntry {
                    key: entry.key.as_bytes().to_vec(),
                    value: entry.value.as_bytes().to_vec(),
                    is_delete: false,
                })
                .collect();

            let batch_count = batch_entries.len() as u64;

            // Write batch
            self.writer.write_batch(batch_entries).await?;

            exported += batch_count;

            // Check for more pages
            if !result.is_truncated {
                break;
            }
            continuation_token = result.continuation_token;
        }

        info!(exported, "full sync complete");
        Ok(exported)
    }

    /// Export a Set operation to docs (non-batched, for testing).
    #[allow(dead_code)]
    async fn export_set(&self, key: &[u8], value: &[u8], log_index: u64) -> Result<()> {
        // Validate sizes
        if key.len() > MAX_DOC_KEY_SIZE {
            warn!(key_len = key.len(), max = MAX_DOC_KEY_SIZE, "key too large for docs export, skipping");
            return Ok(());
        }
        if value.len() > MAX_DOC_VALUE_SIZE {
            warn!(value_len = value.len(), max = MAX_DOC_VALUE_SIZE, "value too large for docs export, skipping");
            return Ok(());
        }

        self.writer.set_entry(key.to_vec(), value.to_vec()).await?;

        debug!(log_index, key_len = key.len(), "exported Set to docs");
        Ok(())
    }

    /// Export a Delete operation to docs (non-batched, for testing).
    #[allow(dead_code)]
    async fn export_delete(&self, key: &[u8], log_index: u64) -> Result<()> {
        self.writer.delete_entry(key.to_vec()).await?;

        debug!(log_index, key_len = key.len(), "exported Delete to docs");
        Ok(())
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
        let mut replica = store.open_replica(&self.namespace_id).context("failed to open replica")?;

        // hash_and_insert computes the BLAKE3 hash and inserts the entry
        // Note: This stores the entry metadata (hash, size, timestamp) in the replica,
        // but the actual content data needs to be stored separately in a blob store.
        let _hash =
            replica.hash_and_insert(&key, &self.author, &value).context("failed to insert entry into replica")?;

        debug!(namespace = %self.namespace_id, "entry inserted into docs");
        Ok(())
    }

    #[instrument(skip(self), fields(key_len = key.len()))]
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // Lock the store for writing
        let mut store = self.store.lock().await;

        // Open the replica for writing
        let mut replica = store.open_replica(&self.namespace_id).context("failed to open replica")?;

        // In iroh-docs, deletion is done by inserting an empty entry as a tombstone.
        // The delete_prefix method does this by inserting an empty entry at the key.
        // Note: delete_prefix signature is (prefix, author)
        let _deleted_count =
            replica.delete_prefix(&key, &self.author).context("failed to delete entry from replica")?;

        debug!(namespace = %self.namespace_id, "entry deleted from docs");
        Ok(())
    }
}

// ============================================================================
// SyncHandle-based Implementation for P2P Sync
// ============================================================================

/// Docs writer implementation that uses `SyncHandle` for P2P sync compatibility.
///
/// Unlike `IrohDocsWriter` which owns the Store directly, this implementation
/// works with a `SyncHandle` that manages the Store. This allows both the
/// DocsExporter and the P2P sync protocol handler to share the same Store.
///
/// Use this when P2P sync is enabled, use `IrohDocsWriter` when only local
/// export is needed (no P2P sync).
pub struct SyncHandleDocsWriter {
    /// Sync handle for replica operations.
    sync_handle: iroh_docs::actor::SyncHandle,
    /// Namespace ID for the document.
    namespace_id: iroh_docs::NamespaceId,
    /// Author for signing entries.
    author: Author,
}

impl SyncHandleDocsWriter {
    /// Create a new SyncHandleDocsWriter.
    ///
    /// # Arguments
    /// * `sync_handle` - The sync handle from DocsSyncResources
    /// * `namespace_id` - The namespace ID for the document
    /// * `author` - The author for signing entries
    pub fn new(
        sync_handle: iroh_docs::actor::SyncHandle,
        namespace_id: iroh_docs::NamespaceId,
        author: Author,
    ) -> Self {
        Self {
            sync_handle,
            namespace_id,
            author,
        }
    }
}

#[async_trait]
impl DocsWriter for SyncHandleDocsWriter {
    #[instrument(skip(self, value), fields(key_len = key.len(), value_len = value.len()))]
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // Compute BLAKE3 hash of the value (same as iroh-blobs uses)
        let hash = iroh_blobs::Hash::new(&value);
        let len = value.len() as u64;
        let author_id = self.author.id();

        // Insert entry metadata via sync handle
        // Note: The actual content should be stored in iroh-blobs for P2P fetching.
        // For now, this stores the metadata (hash, len) which allows sync of entry info.
        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, len)
            .await
            .context("failed to insert entry via sync handle")?;

        debug!(namespace = %self.namespace_id, hash = %hash.fmt_short(), "entry inserted via sync handle");
        Ok(())
    }

    #[instrument(skip(self), fields(key_len = key.len()))]
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // iroh-docs doesn't support empty entries, so we use a tombstone marker
        // This is a convention: a single null byte indicates deletion
        const TOMBSTONE: &[u8] = b"\x00";
        let hash = iroh_blobs::Hash::new(TOMBSTONE);
        let author_id = self.author.id();

        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, TOMBSTONE.len() as u64)
            .await
            .context("failed to delete entry via sync handle")?;

        debug!(namespace = %self.namespace_id, "entry deleted via sync handle (tombstone)");
        Ok(())
    }
}

// ============================================================================
// SyncHandle + Blob Store Implementation for Full P2P Content Transfer
// ============================================================================

/// Docs writer that stores content in iroh-blobs for full P2P content transfer.
///
/// This writer combines iroh-docs (for entry metadata sync) with iroh-blobs
/// (for actual content storage). When entries sync to peers, they can fetch
/// the actual content via iroh-blobs using the hash in the entry metadata.
///
/// # Data Flow
///
/// 1. Content is stored in iroh-blobs → returns Hash
/// 2. Entry metadata (key, hash, len) is stored in iroh-docs
/// 3. iroh-docs syncs entry metadata to peers
/// 4. Peers can fetch content from iroh-blobs using the Hash
pub struct BlobBackedDocsWriter {
    /// Sync handle for replica operations.
    sync_handle: iroh_docs::actor::SyncHandle,
    /// Namespace ID for the document.
    namespace_id: iroh_docs::NamespaceId,
    /// Author for signing entries.
    author: Author,
    /// Blob store for content storage.
    blob_store: std::sync::Arc<aspen_blob::store::IrohBlobStore>,
}

impl BlobBackedDocsWriter {
    /// Create a new BlobBackedDocsWriter.
    ///
    /// # Arguments
    /// * `sync_handle` - The sync handle from DocsSyncResources
    /// * `namespace_id` - The namespace ID for the document
    /// * `author` - The author for signing entries
    /// * `blob_store` - The blob store for content storage
    pub fn new(
        sync_handle: iroh_docs::actor::SyncHandle,
        namespace_id: iroh_docs::NamespaceId,
        author: Author,
        blob_store: std::sync::Arc<aspen_blob::store::IrohBlobStore>,
    ) -> Self {
        Self {
            sync_handle,
            namespace_id,
            author,
            blob_store,
        }
    }
}

#[async_trait]
impl DocsWriter for BlobBackedDocsWriter {
    #[instrument(skip(self, value), fields(key_len = key.len(), value_len = value.len()))]
    async fn set_entry(&self, key: Vec<u8>, value: Vec<u8>) -> Result<()> {
        // Store content in iroh-blobs first
        let blob_result = self.blob_store.add_bytes(&value).await.context("failed to store content in blob store")?;

        let hash = blob_result.blob_ref.hash;
        let len = blob_result.blob_ref.size;
        let author_id = self.author.id();

        // Insert entry metadata via sync handle
        // Now the content is available for P2P fetching via iroh-blobs
        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, len)
            .await
            .context("failed to insert entry via sync handle")?;

        debug!(
            namespace = %self.namespace_id,
            hash = %hash.fmt_short(),
            size = len,
            "entry inserted with blob content"
        );
        Ok(())
    }

    #[instrument(skip(self), fields(key_len = key.len()))]
    async fn delete_entry(&self, key: Vec<u8>) -> Result<()> {
        // iroh-docs doesn't support empty entries, so we use a tombstone marker
        const TOMBSTONE: &[u8] = b"\x00";

        // Store tombstone in blobs for consistency
        let blob_result =
            self.blob_store.add_bytes(TOMBSTONE).await.context("failed to store tombstone in blob store")?;

        let hash = blob_result.blob_ref.hash;
        let author_id = self.author.id();

        self.sync_handle
            .insert_local(self.namespace_id, author_id, bytes::Bytes::from(key), hash, TOMBSTONE.len() as u64)
            .await
            .context("failed to delete entry via sync handle")?;

        debug!(namespace = %self.namespace_id, "entry deleted via sync handle (tombstone in blob)");
        Ok(())
    }
}

// ============================================================================
// In-Memory Implementation for Testing
// ============================================================================

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
        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);

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

        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);
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

        let (sender, _) = broadcast::channel::<LogEntryPayload>(LOG_BROADCAST_BUFFER_SIZE);
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
