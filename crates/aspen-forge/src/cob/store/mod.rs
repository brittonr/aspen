//! COB storage and state resolution.

mod conflict_resolution;
mod issue_ops;
mod listing;
mod patch_ops;
mod persistence;
mod types;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_core::hlc::HLC;
use aspen_core::hlc::create_hlc;
use tokio::sync::broadcast;
pub use types::CobUpdateEvent;
pub use types::ConflictReport;
pub use types::ConflictingValue;
pub use types::FieldConflict;

/// Storage and resolution for collaborative objects.
pub struct CobStore<B: BlobStore, K: KeyValueStore + ?Sized> {
    blobs: Arc<B>,
    kv: Arc<K>,
    secret_key: iroh::SecretKey,
    /// Event sender for COB updates.
    event_tx: broadcast::Sender<CobUpdateEvent>,
    /// Hybrid Logical Clock for deterministic timestamp ordering.
    hlc: HLC,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> CobStore<B, K> {
    /// Create a new COB store.
    ///
    /// # Arguments
    ///
    /// * `blobs` - Blob storage backend
    /// * `kv` - Key-value store backend
    /// * `secret_key` - Ed25519 secret key for signing changes
    /// * `node_id` - Unique node identifier for HLC (e.g., public key hex)
    pub fn new(blobs: Arc<B>, kv: Arc<K>, secret_key: iroh::SecretKey, node_id: &str) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let hlc = create_hlc(node_id);
        Self {
            blobs,
            kv,
            secret_key,
            event_tx,
            hlc,
        }
    }

    /// Subscribe to COB update events.
    ///
    /// Returns a receiver that will receive events when COB changes are stored.
    /// This can be used to trigger gossip broadcasts or other side effects.
    pub fn subscribe(&self) -> broadcast::Receiver<CobUpdateEvent> {
        self.event_tx.subscribe()
    }
}
