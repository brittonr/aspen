//! Core types and traits for the DocsExporter.
//!
//! Contains the `BatchEntry` type, `DocsWriter` trait, and `DocsExporter` struct.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use super::super::events::DocsEventBroadcaster;

/// Maximum time to buffer entries before forcing a flush.
/// Tiger Style: Bounded latency for export operations.
pub(crate) const BATCH_FLUSH_INTERVAL: Duration = Duration::from_millis(100);

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
    pub(crate) writer: Arc<dyn DocsWriter>,
    /// Cancellation token for shutdown.
    pub(crate) cancel: CancellationToken,
    /// Optional event broadcaster for hook integration.
    pub(crate) event_broadcaster: Option<Arc<DocsEventBroadcaster>>,
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
            event_broadcaster: None,
        }
    }

    /// Set the event broadcaster for hook integration.
    ///
    /// When set, the exporter will emit `EntryExported` events after successful
    /// batch writes, enabling external hook programs to react to export activity.
    #[must_use]
    pub fn with_event_broadcaster(mut self, broadcaster: Arc<DocsEventBroadcaster>) -> Self {
        self.event_broadcaster = Some(broadcaster);
        self
    }

    /// Get the event broadcaster, if any.
    pub fn event_broadcaster(&self) -> Option<&Arc<DocsEventBroadcaster>> {
        self.event_broadcaster.as_ref()
    }

    /// Shutdown the exporter.
    pub fn shutdown(&self) {
        self.cancel.cancel();
    }
}
