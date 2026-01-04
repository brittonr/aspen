//! PijulStore: High-level coordinator for Pijul operations.
//!
//! This module provides the main entry point for Pijul operations in Aspen,
//! coordinating between change storage, channel management, and the pristine.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use aspen_blob::BlobStore;
use aspen_core::KeyValueStore;
use aspen_forge::identity::RepoId;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::info;
use tracing::instrument;
use tracing::warn;

use super::apply::ApplyResult;
use super::apply::ChangeApplicator;
use super::apply::ChangeDirectory;
use super::change_store::AspenChangeStore;
use super::constants::KV_PREFIX_PIJUL_REPOS;
use super::constants::MAX_CHANNELS;
use super::error::PijulError;
use super::error::PijulResult;
use super::pristine::PristineManager;
use super::refs::ChannelUpdateEvent;
use super::refs::PijulRefStore;
use super::types::ChangeHash;
use super::types::ChangeMetadata;
use super::types::Channel;
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
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value: value_b64 },
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
        let _ = self.event_tx.send(PijulStoreEvent::RepoCreated { repo_id, identity });

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

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: aspen_core::ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => return Err(PijulError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(value_b64) => {
                let value =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &value_b64).map_err(|e| {
                        PijulError::Serialization {
                            message: format!("invalid base64: {}", e),
                        }
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
        use aspen_core::ScanRequest;

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

    /// Get a channel using local (stale) read consistency.
    ///
    /// This reads from the local state without requiring the Raft leader,
    /// making it suitable for use on follower nodes in sync handlers.
    /// The data may be slightly stale but will be eventually consistent.
    #[instrument(skip(self))]
    pub async fn get_channel_local(&self, repo_id: &RepoId, channel: &str) -> PijulResult<Option<Channel>> {
        use aspen_core::ReadConsistency;

        // Use stale read for channel exists check too
        let head = self.refs.get_channel_with_consistency(repo_id, channel, ReadConsistency::Stale).await?;

        // If we got a head, channel exists
        // If None, we can't distinguish between empty channel and non-existent channel with stale reads
        // For sync purposes, treat None as "no local data to compare"
        match head {
            Some(h) => Ok(Some(Channel {
                name: channel.to_string(),
                head: Some(h),
                updated_at_ms: chrono::Utc::now().timestamp_millis() as u64,
            })),
            None => Ok(None),
        }
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
    pub async fn fork_channel(&self, repo_id: &RepoId, source: &str, target: &str) -> PijulResult<Channel> {
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
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set {
                    key: meta_key,
                    value: meta_b64,
                },
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

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key: meta_key,
                consistency: aspen_core::ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => return Err(PijulError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(value_b64) => {
                let value =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &value_b64).map_err(|e| {
                        PijulError::Serialization {
                            message: format!("invalid base64: {}", e),
                        }
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
    pub async fn download_change(&self, hash: &ChangeHash, provider: iroh::PublicKey) -> PijulResult<()> {
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
    pub async fn apply_change(&self, repo_id: &RepoId, channel: &str, hash: &ChangeHash) -> PijulResult<ApplyResult> {
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

    /// Unrecord (remove) a change from a channel.
    ///
    /// This removes a change from a channel's history. The change itself is not
    /// deleted from storage (it may be referenced by other channels), but it
    /// will no longer be part of this channel's state.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository to unrecord the change from
    /// - `channel`: Channel to update
    /// - `hash`: Hash of the change to unrecord
    ///
    /// # Returns
    ///
    /// `true` if the change was successfully unrecorded, `false` if it was not
    /// in the channel.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The repository doesn't exist
    /// - The change has dependents that haven't been unrecorded yet
    /// - There's a pristine storage error
    #[instrument(skip(self))]
    pub async fn unrecord_change(&self, repo_id: &RepoId, channel: &str, hash: &ChangeHash) -> PijulResult<bool> {
        use libpijul::MutTxnTExt;

        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create the change directory for this repo
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        change_dir.ensure_dir()?;

        // Get the libpijul change store
        let store = change_dir.libpijul_store();

        // Convert our hash to libpijul's Hash
        let pijul_hash = ChangeDirectory::<B>::to_pijul_hash(hash);

        // Start a mutable transaction
        let mut txn = pristine.mut_txn_begin()?;

        // Open the channel
        let channel_ref = txn.open_or_create_channel(channel)?;

        // Unrecord the change using MutTxnTExt trait method
        let result = {
            let txn_inner = txn.txn_mut();
            txn_inner.unrecord(&store, &channel_ref, &pijul_hash, 0).map_err(|e| PijulError::UnrecordFailed {
                message: format!("{:?}", e),
            })?
        };

        // Commit the transaction
        txn.commit()?;

        if result {
            info!(repo_id = %repo_id, channel = channel, hash = %hash, "unrecorded change");
        } else {
            debug!(repo_id = %repo_id, channel = channel, hash = %hash, "change was not in channel");
        }

        Ok(result)
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
    // Pristine Sync Operations
    // ========================================================================

    /// Sync local pristine state with cluster for a channel.
    ///
    /// This method fetches missing changes from the cluster and applies them
    /// to bring the local pristine up to date with the cluster's channel head.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to sync
    ///
    /// # Returns
    ///
    /// The number of changes that were fetched and applied.
    #[instrument(skip(self))]
    pub async fn sync_channel_pristine(&self, repo_id: &RepoId, channel: &str) -> PijulResult<SyncResult> {
        use super::pristine::PristineHandle;

        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get the cluster's channel head
        let cluster_head = self.refs.get_channel(repo_id, channel).await?;
        if cluster_head.is_none() {
            debug!(repo_id = %repo_id, channel = channel, "channel has no changes");
            return Ok(SyncResult {
                changes_fetched: 0,
                changes_applied: 0,
                already_synced: true,
                conflicts: None,
            });
        }

        // Get all changes from the cluster's log
        let cluster_log = self.get_change_log(repo_id, channel, 10_000).await?;
        if cluster_log.is_empty() {
            return Ok(SyncResult {
                changes_fetched: 0,
                changes_applied: 0,
                already_synced: true,
                conflicts: None,
            });
        }

        // Get or create local pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Get changes that are already applied locally
        let local_changes = self.get_local_channel_changes(&pristine, channel)?;

        // Determine missing changes (cluster has but local doesn't)
        let missing: Vec<ChangeHash> =
            cluster_log.iter().map(|m| m.hash).filter(|h| !local_changes.contains(h)).collect();

        if missing.is_empty() {
            debug!(repo_id = %repo_id, channel = channel, "pristine already synced");
            return Ok(SyncResult {
                changes_fetched: 0,
                changes_applied: 0,
                already_synced: true,
                conflicts: None,
            });
        }

        info!(
            repo_id = %repo_id,
            channel = channel,
            missing = missing.len(),
            "syncing pristine with cluster"
        );

        // Build dependency-ordered list (oldest first)
        let ordered = self.order_changes_by_dependencies(&cluster_log, &missing);

        // Create change directory and applicator
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Fetch and apply each missing change in order
        let mut changes_fetched = 0u32;
        let mut changes_applied = 0u32;

        for hash in ordered {
            match applicator.fetch_and_apply(channel, &hash).await {
                Ok(_result) => {
                    changes_fetched += 1;
                    changes_applied += 1;
                    debug!(hash = %hash, "applied missing change");
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to apply change");
                    return Err(e);
                }
            }
        }

        info!(
            repo_id = %repo_id,
            channel = channel,
            fetched = changes_fetched,
            applied = changes_applied,
            "pristine sync complete"
        );

        Ok(SyncResult {
            changes_fetched,
            changes_applied,
            already_synced: false,
            conflicts: None,
        })
    }

    /// Rebuild a channel's pristine from scratch using cluster changes.
    ///
    /// This method creates a fresh pristine and applies all changes from the
    /// cluster's change log. Use this when the local pristine is corrupted
    /// or diverged from the cluster state.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to rebuild
    ///
    /// # Returns
    ///
    /// The number of changes that were applied to rebuild the pristine.
    #[instrument(skip(self))]
    pub async fn rebuild_channel_pristine(&self, repo_id: &RepoId, channel: &str) -> PijulResult<SyncResult> {
        // Verify repo exists
        if !self.repo_exists(repo_id).await? {
            return Err(PijulError::RepoNotFound {
                repo_id: repo_id.to_string(),
            });
        }

        // Get the cluster's change log
        let cluster_log = self.get_change_log(repo_id, channel, 10_000).await?;

        info!(
            repo_id = %repo_id,
            channel = channel,
            changes = cluster_log.len(),
            "rebuilding pristine from cluster"
        );

        // Delete the existing pristine to start fresh
        let pristine_path = self.pristine_path(repo_id);
        if pristine_path.exists() {
            std::fs::remove_dir_all(&pristine_path).map_err(|e| PijulError::Io {
                message: format!("failed to remove pristine: {}", e),
            })?;
        }

        // Create a fresh pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Build dependency-ordered list (oldest first)
        let all_hashes: Vec<ChangeHash> = cluster_log.iter().map(|m| m.hash).collect();
        let ordered = self.order_changes_by_dependencies(&cluster_log, &all_hashes);

        // Create change directory and applicator
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Apply all changes in order
        let mut changes_fetched = 0u32;
        let mut changes_applied = 0u32;

        for hash in ordered {
            match applicator.fetch_and_apply(channel, &hash).await {
                Ok(_result) => {
                    changes_fetched += 1;
                    changes_applied += 1;
                    debug!(hash = %hash, "applied change during rebuild");
                }
                Err(e) => {
                    warn!(hash = %hash, error = %e, "failed to apply change during rebuild");
                    return Err(e);
                }
            }
        }

        info!(
            repo_id = %repo_id,
            channel = channel,
            applied = changes_applied,
            "pristine rebuild complete"
        );

        Ok(SyncResult {
            changes_fetched,
            changes_applied,
            already_synced: false,
            conflicts: None,
        })
    }

    // ========================================================================
    // Conflict Detection
    // ========================================================================

    /// Check for conflicts on a channel by outputting to a temporary directory.
    ///
    /// This method outputs the channel's state to a temp directory and checks
    /// for any conflicts that libpijul detects. This is useful for detecting
    /// merge conflicts after syncing changes from multiple sources.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to check
    ///
    /// # Returns
    ///
    /// Conflict state including paths of conflicting files.
    #[instrument(skip(self))]
    pub async fn check_conflicts(
        &self,
        repo_id: &RepoId,
        channel: &str,
    ) -> PijulResult<super::types::ChannelConflictState> {
        use super::output::WorkingDirOutput;
        use super::types::ChannelConflictState;
        use super::types::FileConflict;

        // Get the pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create a temp directory for output
        let temp_dir = std::env::temp_dir().join(format!(
            "aspen-conflict-check-{}-{}",
            repo_id.to_hex(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()
        ));
        std::fs::create_dir_all(&temp_dir).map_err(|e| PijulError::Io {
            message: format!("failed to create temp directory: {}", e),
        })?;

        // Create change directory and outputter
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let outputter = WorkingDirOutput::new(pristine, change_dir, temp_dir.clone());

        // Output and check for conflicts
        let result = outputter.output(channel)?;

        let now_ms = chrono::Utc::now().timestamp_millis() as u64;

        // Convert libpijul conflicts to our format
        let conflicts: Vec<FileConflict> = result
            .conflicts
            .iter()
            .map(|c| {
                // Extract path and involved changes from libpijul Conflict
                let (path, involved_changes) = extract_conflict_info(c);
                FileConflict {
                    path,
                    involved_changes,
                    detected_at_ms: now_ms,
                }
            })
            .collect();

        // Clean up temp directory
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            debug!(path = %temp_dir.display(), error = %e, "failed to clean up temp directory");
        }

        let conflict_count = conflicts.len();
        if conflict_count > 0 {
            info!(
                repo_id = %repo_id,
                channel = channel,
                conflicts = conflict_count,
                "conflicts detected"
            );
        } else {
            debug!(repo_id = %repo_id, channel = channel, "no conflicts");
        }

        // Get the current channel head for the conflict state
        let current_head = self.refs.get_channel(repo_id, channel).await?;

        Ok(ChannelConflictState {
            conflicts,
            checked_at_ms: now_ms,
            head_at_check: current_head,
        })
    }

    /// Sync a channel and check for conflicts.
    ///
    /// This is a convenience method that combines `sync_channel_pristine` with
    /// `check_conflicts`, returning the sync result with conflict information.
    #[instrument(skip(self))]
    pub async fn sync_and_check_conflicts(&self, repo_id: &RepoId, channel: &str) -> PijulResult<SyncResult> {
        // First sync
        let mut result = self.sync_channel_pristine(repo_id, channel).await?;

        // Then check for conflicts if we applied changes
        if result.changes_applied > 0 {
            let conflict_state = self.check_conflicts(repo_id, channel).await?;
            result.conflicts = Some(conflict_state);
        }

        Ok(result)
    }

    /// Apply a downloaded change to the local pristine.
    ///
    /// This is a local-only operation that does NOT query Raft. It's used by
    /// follower nodes after P2P downloading a change - the change is already
    /// in the blob store, we just need to apply it to the local pristine.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name
    /// - `hash`: The change hash that was just downloaded
    ///
    /// # Returns
    ///
    /// Result indicating if the change was applied, with conflict information.
    #[instrument(skip(self))]
    pub async fn apply_downloaded_change(
        &self,
        repo_id: &RepoId,
        channel: &str,
        hash: &ChangeHash,
    ) -> PijulResult<SyncResult> {
        // Verify the change exists in our blob store
        if !self.changes.has_change(hash).await? {
            return Err(PijulError::ChangeNotFound { hash: hash.to_string() });
        }

        // Get or create local pristine
        let pristine = self.pristines.open_or_create(repo_id)?;

        // Create change directory and applicator
        let change_dir = ChangeDirectory::new(&self.data_dir, *repo_id, Arc::clone(&self.changes));
        let applicator = ChangeApplicator::new(pristine, change_dir);

        // Apply the change
        match applicator.fetch_and_apply(channel, hash).await {
            Ok(_result) => {
                info!(hash = %hash, channel = channel, "applied downloaded change to local pristine");

                // Check for conflicts
                let conflict_state = self.check_conflicts(repo_id, channel).await?;
                let has_conflicts = conflict_state.has_conflicts();

                Ok(SyncResult {
                    changes_fetched: 0, // Already fetched
                    changes_applied: 1,
                    already_synced: false,
                    conflicts: if has_conflicts { Some(conflict_state) } else { None },
                })
            }
            Err(e) => {
                // If it's already applied, that's fine
                if e.to_string().contains("already applied") || e.to_string().contains("ChangeIsDependency") {
                    debug!(hash = %hash, "change already applied to pristine");
                    Ok(SyncResult {
                        changes_fetched: 0,
                        changes_applied: 0,
                        already_synced: true,
                        conflicts: None,
                    })
                } else {
                    Err(e)
                }
            }
        }
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Get the path to a repository's pristine database.
    fn pristine_path(&self, repo_id: &RepoId) -> PathBuf {
        self.data_dir.join("pijul").join(repo_id.to_string()).join("pristine")
    }

    /// Get the set of changes already applied to a channel in the local pristine.
    ///
    /// Note: This is a simplified implementation that returns an empty set,
    /// relying on libpijul to handle duplicate application gracefully.
    /// A more optimized version could query the pristine's changeset.
    fn get_local_channel_changes(
        &self,
        _pristine: &super::pristine::PristineHandle,
        _channel: &str,
    ) -> PijulResult<std::collections::HashSet<ChangeHash>> {
        // For now, return empty set - libpijul will handle duplicates internally
        // by checking if a change is already applied before applying it again.
        // This is safe but may be less efficient for large changelogs.
        Ok(std::collections::HashSet::new())
    }

    /// Order changes by dependencies (oldest/no-deps first).
    fn order_changes_by_dependencies(&self, log: &[ChangeMetadata], target_hashes: &[ChangeHash]) -> Vec<ChangeHash> {
        use std::collections::HashMap;
        use std::collections::HashSet;
        use std::collections::VecDeque;

        let target_set: HashSet<_> = target_hashes.iter().copied().collect();
        let meta_map: HashMap<_, _> = log.iter().map(|m| (m.hash, m)).collect();

        // Kahn's algorithm for topological sort
        let mut in_degree: HashMap<ChangeHash, usize> = HashMap::new();
        let mut deps_of: HashMap<ChangeHash, Vec<ChangeHash>> = HashMap::new();

        for &hash in &target_set {
            in_degree.entry(hash).or_insert(0);
            if let Some(meta) = meta_map.get(&hash) {
                for dep in &meta.dependencies {
                    if target_set.contains(dep) {
                        *in_degree.entry(hash).or_insert(0) += 1;
                        deps_of.entry(*dep).or_default().push(hash);
                    }
                }
            }
        }

        // Start with changes that have no dependencies in target set
        let mut queue: VecDeque<_> =
            in_degree.iter().filter(|&(_, degree)| *degree == 0).map(|(&hash, _)| hash).collect();

        let mut ordered = Vec::with_capacity(target_hashes.len());

        while let Some(hash) = queue.pop_front() {
            ordered.push(hash);
            if let Some(dependents) = deps_of.get(&hash) {
                for &dep in dependents {
                    if let Some(degree) = in_degree.get_mut(&dep) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        ordered
    }
}

/// Extract conflict information from a libpijul Conflict.
///
/// This attempts to extract the file path and involved change hashes
/// from libpijul's conflict representation.
///
/// # Returns
///
/// A tuple of (file_path, involved_changes) where:
/// - file_path: The path of the conflicting file
/// - involved_changes: List of ChangeHash values involved in the conflict
fn extract_conflict_info(conflict: &libpijul::output::Conflict) -> (String, Vec<ChangeHash>) {
    use libpijul::output::Conflict;

    match conflict {
        // Name conflicts involve file path conflicts
        Conflict::Name { path, .. } => {
            let path_str = path.clone();
            // Name conflicts don't have specific change hashes easily accessible
            (path_str, Vec::new())
        }

        // Zombie conflicts are more complex - represent content conflicts
        Conflict::ZombieFile { path, .. } => {
            let path_str = path.clone();
            // Zombie conflicts contain change information but require deeper analysis
            // For now, return empty list - full extraction would require traversing
            // the conflict graph which is complex
            (path_str, Vec::new())
        }

        // Order conflicts involve ordering dependencies
        Conflict::Order { path, .. } => {
            let path_str = path.clone();
            // Order conflicts also contain change information that's difficult to extract
            (path_str, Vec::new())
        }

        // Cyclical order conflicts
        Conflict::Cyclic { path, .. } => {
            let path_str = path.clone();
            (path_str, Vec::new())
        }

        // Multiple names conflict
        Conflict::MultipleNames { path, .. } => {
            let path_str = path.clone();
            (path_str, Vec::new())
        }

        // Zombie conflicts (alternative form)
        Conflict::Zombie { path, .. } => {
            let path_str = path.clone();
            (path_str, Vec::new())
        }
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
    use aspen_blob::InMemoryBlobStore;
    use aspen_core::DeterministicKeyValueStore;

    use super::*;
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
