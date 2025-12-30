//! PijulStore: High-level coordinator for Pijul operations.
//!
//! This module provides the main entry point for Pijul operations in Aspen,
//! coordinating between change storage, channel management, and the pristine.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing::{debug, info, instrument, warn};

use crate::api::KeyValueStore;
use crate::blob::BlobStore;
use crate::forge::identity::RepoId;

use super::apply::{ApplyResult, ChangeApplicator, ChangeDirectory};
use super::change_store::AspenChangeStore;
use super::constants::{KV_PREFIX_PIJUL_REPOS, MAX_CHANNELS};
use super::error::{PijulError, PijulResult};
use super::pristine::PristineManager;
use super::refs::{ChannelUpdateEvent, PijulRefStore};
use super::types::{ChangeHash, ChangeMetadata, Channel, PijulRepoIdentity};

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

    // ========================================================================
    // Repository Management
    // ========================================================================

    /// Create a new Pijul repository.
    ///
    /// # Arguments
    ///
    /// - `identity`: Repository identity with name, delegates, etc.
    ///
    /// # Returns
    ///
    /// The computed `RepoId` (BLAKE3 hash of the identity).
    #[instrument(skip(self, identity), fields(name = %identity.name))]
    pub async fn create_repo(&self, identity: PijulRepoIdentity) -> PijulResult<RepoId> {
        let repo_id = identity.repo_id();

        // Check if already exists
        if self.get_repo(&repo_id).await?.is_some() {
            return Err(PijulError::RepoAlreadyExists {
                repo_id: repo_id.to_string(),
            });
        }

        // Store identity in KV
        let key = format!("{}{}", KV_PREFIX_PIJUL_REPOS, repo_id);
        let value = postcard::to_allocvec(&identity)?;
        let value_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &value);

        self.kv
            .write(crate::api::WriteRequest {
                command: crate::api::WriteCommand::Set { key, value: value_b64 },
            })
            .await?;

        // Create pristine directory
        let pristine_dir = self.pristine_path(&repo_id);
        tokio::fs::create_dir_all(&pristine_dir).await?;

        // Create the default channel (empty, no head yet)
        let default_channel = identity.default_channel.clone();
        self.refs.create_empty_channel(&repo_id, &default_channel).await?;

        // Cache the identity
        {
            let mut cache = self.repo_cache.write();
            cache.insert(repo_id, identity.clone());
        }

        info!(repo_id = %repo_id, name = %identity.name, default_channel = %default_channel, "created Pijul repository");

        // Emit event
        let _ = self.event_tx.send(PijulStoreEvent::RepoCreated {
            repo_id,
            identity,
        });

        Ok(repo_id)
    }

    /// Get a repository's identity.
    #[instrument(skip(self))]
    pub async fn get_repo(&self, repo_id: &RepoId) -> PijulResult<Option<PijulRepoIdentity>> {
        // Check cache first
        {
            let cache = self.repo_cache.read();
            if let Some(identity) = cache.get(repo_id) {
                return Ok(Some(identity.clone()));
            }
        }

        // Load from KV
        let key = format!("{}{}", KV_PREFIX_PIJUL_REPOS, repo_id);

        let result = match self.kv.read(crate::api::ReadRequest {
            key,
            consistency: crate::api::ReadConsistency::Linearizable,
        }).await {
            Ok(r) => r,
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => return Err(PijulError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(value_b64) => {
                let value = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &value_b64)
                    .map_err(|e| PijulError::Serialization {
                        message: format!("invalid base64: {}", e),
                    })?;

                let identity: PijulRepoIdentity = postcard::from_bytes(&value)?;

                // Cache it
                {
                    let mut cache = self.repo_cache.write();
                    cache.insert(*repo_id, identity.clone());
                }

                Ok(Some(identity))
            }
            None => Ok(None),
        }
    }

    /// Check if a repository exists.
    pub async fn repo_exists(&self, repo_id: &RepoId) -> PijulResult<bool> {
        Ok(self.get_repo(repo_id).await?.is_some())
    }

    /// List all repositories.
    ///
    /// Returns a list of (repo_id, identity) pairs for all known repositories.
    ///
    /// # Arguments
    ///
    /// - `limit`: Maximum number of repos to return (default 100, max 1000)
    #[instrument(skip(self))]
    pub async fn list_repos(&self, limit: u32) -> PijulResult<Vec<(RepoId, PijulRepoIdentity)>> {
        use crate::api::ScanRequest;

        let limit = limit.min(1000);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: KV_PREFIX_PIJUL_REPOS.to_string(),
                limit: Some(limit),
                continuation_token: None,
            })
            .await?;

        let mut repos = Vec::with_capacity(result.entries.len());

        for entry in result.entries {
            // Key format: "pijul:repos:{repo_id}"
            let repo_id_str = entry.key.strip_prefix(KV_PREFIX_PIJUL_REPOS).unwrap_or(&entry.key);
            let repo_id = match RepoId::from_hex(repo_id_str) {
                Ok(id) => id,
                Err(_) => {
                    debug!(key = %entry.key, "skipping invalid repo_id in KV");
                    continue;
                }
            };

            // Decode identity
            let value = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &entry.value) {
                Ok(v) => v,
                Err(e) => {
                    debug!(key = %entry.key, error = %e, "skipping repo with invalid base64");
                    continue;
                }
            };

            let identity: PijulRepoIdentity = match postcard::from_bytes(&value) {
                Ok(id) => id,
                Err(e) => {
                    debug!(key = %entry.key, error = %e, "skipping repo with invalid identity");
                    continue;
                }
            };

            repos.push((repo_id, identity));
        }

        Ok(repos)
    }

    // ========================================================================
    // Channel Management
    // ========================================================================

    /// Create a new channel in a repository.
    #[instrument(skip(self))]
    pub async fn create_channel(&self, repo_id: &RepoId, channel: &str) -> PijulResult<()> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Check channel count limit
        let count = self.refs.count_channels(repo_id).await?;
        if count >= MAX_CHANNELS {
            return Err(PijulError::TooManyChannels {
                count,
                max: MAX_CHANNELS,
            });
        }

        // Check if channel already exists
        if self.refs.channel_exists(repo_id, channel).await? {
            return Err(PijulError::ChannelAlreadyExists {
                channel: channel.to_string(),
            });
        }

        // Create the empty channel in the ref store
        self.refs.create_empty_channel(repo_id, channel).await?;

        info!(repo_id = %repo_id, channel = channel, "created channel");
        Ok(())
    }

    /// Get a channel's current state.
    ///
    /// Returns `Some(Channel)` if the channel exists (with or without a head),
    /// or `None` if the channel doesn't exist.
    #[instrument(skip(self))]
    pub async fn get_channel(&self, repo_id: &RepoId, channel: &str) -> PijulResult<Option<Channel>> {
        // First check if channel exists
        if !self.refs.channel_exists(repo_id, channel).await? {
            return Ok(None);
        }

        // Channel exists, get its head (may be None for empty channels)
        let head = self.refs.get_channel(repo_id, channel).await?;

        Ok(Some(Channel {
            name: channel.to_string(),
            head,
            updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
        }))
    }

    /// List all channels in a repository.
    pub async fn list_channels(&self, repo_id: &RepoId) -> PijulResult<Vec<Channel>> {
        let channel_heads = self.refs.list_channels(repo_id).await?;

        Ok(channel_heads
            .into_iter()
            .map(|(name, head)| Channel {
                name,
                head,
                updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            })
            .collect())
    }

    /// Delete a channel from a repository.
    ///
    /// This removes the channel head reference from the KV store.
    /// The changes themselves are not deleted (they may be referenced by other channels).
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to delete
    #[instrument(skip(self))]
    pub async fn delete_channel(&self, repo_id: &RepoId, channel: &str) -> PijulResult<()> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Check if channel exists
        if !self.refs.channel_exists(repo_id, channel).await? {
            return Err(PijulError::ChannelNotFound {
                channel: channel.to_string(),
            });
        }

        // Delete the channel
        self.refs.delete_channel(repo_id, channel).await?;

        info!(repo_id = %repo_id, channel = channel, "deleted channel");
        Ok(())
    }

    /// Fork a channel, creating a new channel with the same head.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `source`: Source channel name to fork from
    /// - `target`: Target channel name to create
    #[instrument(skip(self))]
    pub async fn fork_channel(
        &self,
        repo_id: &RepoId,
        source: &str,
        target: &str,
    ) -> PijulResult<Channel> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Check channel count limit
        let count = self.refs.count_channels(repo_id).await?;
        if count >= MAX_CHANNELS {
            return Err(PijulError::TooManyChannels {
                count,
                max: MAX_CHANNELS,
            });
        }

        // Check if target already exists
        if self.refs.channel_exists(repo_id, target).await? {
            return Err(PijulError::ChannelAlreadyExists {
                channel: target.to_string(),
            });
        }

        // Check source channel exists
        if !self.refs.channel_exists(repo_id, source).await? {
            return Err(PijulError::ChannelNotFound {
                channel: source.to_string(),
            });
        }

        // Get source channel's head (may be None for empty channels)
        let source_head = self.refs.get_channel(repo_id, source).await?;

        // Create target channel with the same head (or empty if source is empty)
        match source_head {
            Some(head) => {
                self.refs.set_channel(repo_id, target, head).await?;
                info!(repo_id = %repo_id, source = source, target = target, head = %head, "forked channel");
                Ok(Channel::with_head(target, head))
            }
            None => {
                self.refs.create_empty_channel(repo_id, target).await?;
                info!(repo_id = %repo_id, source = source, target = target, "forked empty channel");
                Ok(Channel::new(target))
            }
        }
    }

    // ========================================================================
    // Change Operations
    // ========================================================================

    /// Store a change and update the channel head.
    ///
    /// This is the primary method for recording new changes. The change should
    /// already be serialized in libpijul's format (bincode + zstd compressed).
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to store the change in
    /// - `channel`: Channel to update
    /// - `change_bytes`: Serialized change data
    /// - `metadata`: Metadata about the change
    ///
    /// # Returns
    ///
    /// The BLAKE3 hash of the stored change.
    #[instrument(skip(self, change_bytes, metadata), fields(size = change_bytes.len()))]
    pub async fn store_change(
        &self,
        repo_id: &RepoId,
        channel: &str,
        change_bytes: &[u8],
        metadata: ChangeMetadata,
    ) -> PijulResult<ChangeHash> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Store the change in blob store
        let hash = self.changes.store_change(change_bytes).await?;

        // Store metadata in KV
        let meta_key = format!("{}{}:{}", super::constants::KV_PREFIX_PIJUL_CHANGE_META, repo_id, hash);
        let meta_bytes = postcard::to_allocvec(&metadata)?;
        let meta_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &meta_bytes);

        self.kv
            .write(crate::api::WriteRequest {
                command: crate::api::WriteCommand::Set { key: meta_key, value: meta_b64 },
            })
            .await?;

        // Update channel head
        self.refs.set_channel(repo_id, channel, hash).await?;

        info!(repo_id = %repo_id, channel = channel, hash = %hash, "stored change");

        // Emit event
        let _ = self.event_tx.send(PijulStoreEvent::ChangeRecorded {
            repo_id: *repo_id,
            channel: channel.to_string(),
            change_hash: hash,
        });

        Ok(hash)
    }

    /// Get a change by hash.
    ///
    /// Returns the raw serialized change bytes.
    pub async fn get_change(&self, hash: &ChangeHash) -> PijulResult<Option<Vec<u8>>> {
        self.changes.get_change(hash).await
    }

    /// Check if a change exists.
    pub async fn has_change(&self, hash: &ChangeHash) -> PijulResult<bool> {
        self.changes.has_change(hash).await
    }

    /// Get change metadata.
    #[instrument(skip(self))]
    pub async fn get_change_metadata(
        &self,
        repo_id: &RepoId,
        hash: &ChangeHash,
    ) -> PijulResult<Option<ChangeMetadata>> {
        let meta_key = format!("{}{}:{}", super::constants::KV_PREFIX_PIJUL_CHANGE_META, repo_id, hash);

        let result = match self.kv.read(crate::api::ReadRequest {
            key: meta_key,
            consistency: crate::api::ReadConsistency::Linearizable,
        }).await {
            Ok(r) => r,
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => return Err(PijulError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(value_b64) => {
                let value = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &value_b64)
                    .map_err(|e| PijulError::Serialization {
                        message: format!("invalid base64: {}", e),
                    })?;

                let metadata: ChangeMetadata = postcard::from_bytes(&value)?;
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    // ========================================================================
    // Sync Operations
    // ========================================================================

    /// Download a change from a remote peer.
    #[instrument(skip(self))]
    pub async fn download_change(
        &self,
        hash: &ChangeHash,
        provider: iroh::PublicKey,
    ) -> PijulResult<()> {
        self.changes.download_from_peer(hash, provider).await
    }

    // ========================================================================
    // Apply Operations
    // ========================================================================

    /// Apply a change to a repository channel.
    ///
    /// This fetches the change from storage (if not cached locally) and applies
    /// it to the pristine database, updating the channel state.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to apply the change to
    /// - `channel`: Channel to update
    /// - `hash`: Hash of the change to apply
    ///
    /// # Returns
    ///
    /// The result of the apply operation including the number of operations
    /// applied and the new merkle hash.
    #[instrument(skip(self))]
    pub async fn apply_change(
        &self,
        repo_id: &RepoId,
        channel: &str,
        hash: &ChangeHash,
    ) -> PijulResult<ApplyResult> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get or create the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create the change directory for this repo
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));

        // Create the applicator
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Fetch and apply the change
        let result = applicator.fetch_and_apply(channel, hash).await?;

        // Update channel head in refs
        self.refs.set_channel(repo_id, channel, *hash).await?;

        info!(repo_id = %repo_id, channel = channel, hash = %hash, ops = result.changes_applied, "applied change");

        // Emit event
        let _ = self.event_tx.send(PijulStoreEvent::ChangeApplied {
            repo_id: *repo_id,
            channel: channel.to_string(),
            change_hash: *hash,
        });

        Ok(result)
    }

    /// Apply multiple changes in order.
    ///
    /// Changes are applied sequentially. If any change fails, the operation
    /// stops and returns an error.
    #[instrument(skip(self, hashes))]
    pub async fn apply_changes(
        &self,
        repo_id: &RepoId,
        channel: &str,
        hashes: &[ChangeHash],
    ) -> PijulResult<Vec<ApplyResult>> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get or create the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create the change directory for this repo
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));

        // Create the applicator
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Apply all changes
        let results = applicator.apply_changes(channel, hashes).await?;

        // Update channel head to the last applied change
        if let Some(last_hash) = hashes.last() {
            self.refs.set_channel(repo_id, channel, *last_hash).await?;

            // Emit event for each applied change
            for hash in hashes {
                let _ = self.event_tx.send(PijulStoreEvent::ChangeApplied {
                    repo_id: *repo_id,
                    channel: channel.to_string(),
                    change_hash: *hash,
                });
            }
        }

        info!(repo_id = %repo_id, channel = channel, count = results.len(), "applied changes");
        Ok(results)
    }

    // ========================================================================
    // Log Operations
    // ========================================================================

    /// Get the change log for a channel.
    ///
    /// Returns a list of changes from the channel head back through the
    /// dependency chain, limited by the specified count.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name
    /// - `limit`: Maximum number of changes to return (default 100, max 1000)
    ///
    /// # Returns
    ///
    /// A list of change metadata in reverse chronological order (newest first).
    #[instrument(skip(self))]
    pub async fn get_change_log(
        &self,
        repo_id: &RepoId,
        channel: &str,
        limit: u32,
    ) -> PijulResult<Vec<ChangeMetadata>> {
        let limit = limit.min(1000);

        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get channel head
        let head = match self.refs.get_channel(repo_id, channel).await? {
            Some(h) => h,
            None => {
                // Channel exists but has no changes
                return Ok(Vec::new());
            }
        };

        // Traverse the change DAG following dependencies
        let mut log = Vec::with_capacity(limit as usize);
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();

        queue.push_back(head);

        while let Some(hash) = queue.pop_front() {
            if log.len() >= limit as usize {
                break;
            }

            if visited.contains(&hash) {
                continue;
            }
            visited.insert(hash);

            // Get metadata for this change
            if let Some(meta) = self.get_change_metadata(repo_id, &hash).await? {
                // Add dependencies to the queue for BFS traversal
                for dep in &meta.dependencies {
                    if !visited.contains(dep) {
                        queue.push_back(*dep);
                    }
                }
                log.push(meta);
            } else {
                warn!(hash = %hash, "change metadata not found in log traversal");
            }
        }

        // Sort by recorded_at_ms descending (newest first)
        log.sort_by(|a, b| b.recorded_at_ms.cmp(&a.recorded_at_ms));

        debug!(repo_id = %repo_id, channel = channel, count = log.len(), "retrieved change log");
        Ok(log)
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Get the path to a repository's pristine database.
    fn pristine_path(&self, repo_id: &RepoId) -> PathBuf {
        self.data_dir.join("pijul").join(repo_id.to_string()).join("pristine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;
    use crate::blob::InMemoryBlobStore;
    use crate::pijul::types::PijulAuthor;

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
        assert!(matches!(result, Err(PijulError::RepoAlreadyExists { .. })));
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
