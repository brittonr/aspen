//! PijulStore: High-level coordinator for Pijul operations.
//!
//! This module provides the main entry point for Pijul operations in Aspen,
//! coordinating between change storage, channel management, and the pristine.

mod change;
mod channel;
mod helpers;
mod repo;
mod sync_ops;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::prelude::*;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use parking_lot::RwLock;
use tokio::sync::broadcast;

use super::change_store::AspenChangeStore;
use super::pristine::PristineManager;
use super::refs::ChannelUpdateEvent;
use super::refs::PijulRefStore;
use super::types::ChangeHash;
use super::types::PijulRepoIdentity;

/// Events emitted by the PijulStore.
#[derive(Debug, Clone)]
pub enum PijulStoreEvent {
    /// A new repository was created.
    RepoCreated {
        repo_id: RepoId,
        identity: PijulRepoIdentity,
    },
    /// A change was recorded.
    ChangeRecorded {
        repo_id: RepoId,
        channel: String,
        change_hash: ChangeHash,
    },
    /// A change was applied from a remote source.
    ChangeApplied {
        repo_id: RepoId,
        channel: String,
        change_hash: ChangeHash,
    },
    /// Channel head updated (forwarded from PijulRefStore).
    ChannelUpdated(ChannelUpdateEvent),
}

/// High-level Pijul store coordinator.
///
/// Provides a unified API for all Pijul operations, coordinating between:
/// - `AspenChangeStore`: Change storage in iroh-blobs
/// - `PijulRefStore`: Channel heads in Raft KV
/// - Pristine management (sanakirja databases)
///
/// # Example
///
/// ```ignore
/// let store = PijulStore::new(blob_store, kv_store, data_dir).await?;
///
/// // Create a repository
/// let identity = PijulRepoIdentity::new("my-project", delegates);
/// let repo_id = store.create_repo(identity).await?;
///
/// // Create a channel
/// store.create_channel(&repo_id, "main").await?;
///
/// // Store a change (pre-serialized)
/// let change_hash = store.store_change(&repo_id, "main", &change_bytes, metadata).await?;
/// ```
pub struct PijulStore<B: BlobStore, K: KeyValueStore + ?Sized> {
    /// Change storage backed by iroh-blobs.
    changes: Arc<AspenChangeStore<B>>,

    /// Channel head storage backed by Raft KV.
    refs: Arc<PijulRefStore<K>>,

    /// KV store for repository metadata.
    kv: Arc<K>,

    /// Base path for pristine databases.
    data_dir: PathBuf,

    /// Pristine database manager.
    pristines: Arc<PristineManager>,

    /// Event sender for store events.
    event_tx: broadcast::Sender<PijulStoreEvent>,

    /// Cached repository identities.
    repo_cache: RwLock<HashMap<RepoId, PijulRepoIdentity>>,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> PijulStore<B, K> {
    /// Create a new PijulStore.
    ///
    /// # Arguments
    ///
    /// - `blobs`: Blob store for change storage
    /// - `kv`: KV store for metadata and channel heads
    /// - `data_dir`: Base directory for pristine databases
    pub fn new(blobs: Arc<B>, kv: Arc<K>, data_dir: impl Into<PathBuf>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let data_dir = data_dir.into();

        let changes = Arc::new(AspenChangeStore::new(blobs));
        let refs = Arc::new(PijulRefStore::new(Arc::clone(&kv)));
        let pristines = Arc::new(PristineManager::new(&data_dir));

        Self {
            changes,
            refs,
            kv,
            data_dir,
            pristines,
            event_tx,
            repo_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to store events.
    pub fn subscribe(&self) -> broadcast::Receiver<PijulStoreEvent> {
        self.event_tx.subscribe()
    }

    /// Get the change store.
    pub fn changes(&self) -> &Arc<AspenChangeStore<B>> {
        &self.changes
    }

    /// Get the ref store.
    pub fn refs(&self) -> &Arc<PijulRefStore<K>> {
        &self.refs
    }
}

/// Result of a pristine sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of changes fetched from cluster.
    pub changes_fetched: u32,
    /// Number of changes applied to pristine.
    pub changes_applied: u32,
    /// Whether the pristine was already in sync.
    pub already_synced: bool,
    /// Conflict state after sync (if checked).
    pub conflicts: Option<super::types::ChannelConflictState>,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_blob::InMemoryBlobStore;
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;
    use crate::types::ChangeMetadata;
    use crate::types::PijulAuthor;

    fn test_delegates() -> Vec<iroh::PublicKey> {
        vec![]
    }

    #[tokio::test]
    async fn test_create_and_get_repo() {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulStore::new(blobs, kv, "/tmp/test-pijul");

        let identity = PijulRepoIdentity::new("test-repo", test_delegates());
        let repo_id = store.create_repo(identity.clone()).await.unwrap();

        // Get the repo
        let retrieved = store.get_repo(&repo_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test-repo");

        // Should exist
        assert!(store.repo_exists(&repo_id).await.unwrap());

        // Creating again should fail
        let result = store.create_repo(identity).await;
        assert!(matches!(result, Err(crate::error::PijulError::RepoAlreadyExists { .. })));
    }

    #[tokio::test]
    async fn test_store_and_get_change() {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulStore::new(blobs, kv, "/tmp/test-pijul");

        // Create repo first
        let identity = PijulRepoIdentity::new("test-repo", test_delegates());
        let repo_id = store.create_repo(identity).await.unwrap();

        // Store a change
        let change_data = b"fake compressed pijul change";
        let metadata = ChangeMetadata {
            hash: ChangeHash([0u8; 32]), // Will be replaced
            repo_id,
            channel: "main".to_string(),
            message: "Test change".to_string(),
            authors: vec![PijulAuthor::from_name_email("Test", "test@example.com")],
            dependencies: vec![],
            size_bytes: change_data.len() as u64,
            recorded_at_ms: chrono::Utc::now().timestamp_millis() as u64,
        };

        let hash = store.store_change(&repo_id, "main", change_data, metadata).await.unwrap();

        // Get the change
        let retrieved = store.get_change(&hash).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), change_data);

        // Channel should have the head
        let channel = store.get_channel(&repo_id, "main").await.unwrap();
        assert!(channel.is_some());
        assert_eq!(channel.unwrap().head, Some(hash));
    }
}
