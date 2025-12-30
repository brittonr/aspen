//! Channel head storage using Raft consensus.
//!
//! This module implements Pijul channel (branch) head storage using Aspen's
//! Raft KV store, ensuring strong consistency across the cluster.
//!
//! ## Empty Channels
//!
//! Channels can exist without a head (before any changes are recorded).
//! We use the sentinel value "EMPTY" to represent this state:
//! - Key doesn't exist → channel doesn't exist
//! - Value is "EMPTY" → channel exists but has no head
//! - Value is hex hash → channel has that head

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{debug, instrument};

use crate::api::{KeyValueStore, ReadConsistency, ReadRequest, ScanRequest, WriteCommand, WriteRequest};
use crate::forge::identity::RepoId;

use super::constants::{KV_PREFIX_PIJUL_CHANNELS, MAX_CHANNELS, MAX_CHANNEL_NAME_LENGTH_BYTES};
use super::error::{PijulError, PijulResult};
use super::types::ChangeHash;

/// Sentinel value for channels that exist but have no head yet.
const EMPTY_CHANNEL_MARKER: &str = "EMPTY";

/// Event emitted when a channel head is updated.
#[derive(Debug, Clone)]
pub struct ChannelUpdateEvent {
    /// Repository ID.
    pub repo_id: RepoId,
    /// Channel name (e.g., "main").
    pub channel: String,
    /// New head hash.
    pub new_head: ChangeHash,
    /// Previous head hash (if known).
    pub old_head: Option<ChangeHash>,
    /// Merkle root after the update (if known).
    ///
    /// This is set when the update comes from applying a change through
    /// the pristine. It may be zeroed if the update came from direct
    /// ref manipulation without pristine involvement.
    pub merkle: [u8; 32],
}

/// Storage for Pijul channel heads using Raft consensus.
///
/// All channel head updates go through Raft to ensure strong consistency.
/// This means all nodes in the cluster will agree on which change is the
/// head of each channel.
///
/// # Key Format
///
/// Channel heads are stored with the key format:
/// ```text
/// pijul:channels:{repo_id}:{channel_name}
/// ```
///
/// For example:
/// ```text
/// pijul:channels:abc123...:main
/// pijul:channels:abc123...:feature/new-ui
/// ```
pub struct PijulRefStore<K: KeyValueStore + ?Sized> {
    kv: Arc<K>,
    /// Event sender for channel updates.
    event_tx: broadcast::Sender<ChannelUpdateEvent>,
}

impl<K: KeyValueStore + ?Sized> Clone for PijulRefStore<K> {
    fn clone(&self) -> Self {
        Self {
            kv: Arc::clone(&self.kv),
            event_tx: self.event_tx.clone(),
        }
    }
}

impl<K: KeyValueStore + ?Sized> PijulRefStore<K> {
    /// Create a new channel store.
    pub fn new(kv: Arc<K>) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self { kv, event_tx }
    }

    /// Subscribe to channel update events.
    ///
    /// Returns a receiver that will receive events when channel heads are updated.
    /// This can be used to trigger gossip broadcasts or sync operations.
    pub fn subscribe(&self) -> broadcast::Receiver<ChannelUpdateEvent> {
        self.event_tx.subscribe()
    }

    /// Get a channel's current head.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name (e.g., "main")
    ///
    /// # Returns
    ///
    /// The change hash the channel head points to, or `None` if the channel
    /// has no head (either doesn't exist or is empty).
    ///
    /// Use [`channel_exists`] to distinguish between a non-existent channel
    /// and an empty channel.
    #[instrument(skip(self))]
    pub async fn get_channel(
        &self,
        repo_id: &RepoId,
        channel: &str,
    ) -> PijulResult<Option<ChangeHash>> {
        self.get_channel_with_consistency(repo_id, channel, ReadConsistency::Linearizable).await
    }

    /// Get channel head with a specific read consistency level.
    ///
    /// Use `ReadConsistency::Stale` for local reads that don't need to go
    /// through the Raft leader (useful for sync handlers on follower nodes).
    #[instrument(skip(self))]
    pub async fn get_channel_with_consistency(
        &self,
        repo_id: &RepoId,
        channel: &str,
        consistency: ReadConsistency,
    ) -> PijulResult<Option<ChangeHash>> {
        let key = self.channel_key(repo_id, channel);

        let result = match self.kv.read(ReadRequest { key, consistency }).await {
            Ok(r) => r,
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => {
                return Ok(None);
            }
            Err(e) => return Err(PijulError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(value) if value == EMPTY_CHANNEL_MARKER => {
                debug!(repo_id = %repo_id, channel = channel, "channel exists but is empty");
                Ok(None)
            }
            Some(hex_hash) => {
                let hash = ChangeHash::from_hex(&hex_hash).map_err(|e| PijulError::InvalidChange {
                    message: format!("invalid channel head hash: {}", e),
                })?;
                debug!(repo_id = %repo_id, channel = channel, head = %hash, "got channel head");
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    /// Create an empty channel (without a head).
    ///
    /// This creates a channel marker so that `channel_exists` returns true,
    /// but `get_channel` returns `None` until a change is applied.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name (e.g., "main")
    #[instrument(skip(self))]
    pub async fn create_empty_channel(
        &self,
        repo_id: &RepoId,
        channel: &str,
    ) -> PijulResult<()> {
        self.validate_channel_name(channel)?;

        let key = self.channel_key(repo_id, channel);

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set { key, value: EMPTY_CHANNEL_MARKER.to_string() },
            })
            .await?;

        debug!(repo_id = %repo_id, channel = channel, "created empty channel");
        Ok(())
    }

    /// Set a channel's head to a new value.
    ///
    /// This goes through Raft consensus, so all nodes will agree on the update.
    /// After successful update, emits a `ChannelUpdateEvent` for gossip broadcast.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name (e.g., "main")
    /// - `head`: The change hash to set as the new head
    #[instrument(skip(self))]
    pub async fn set_channel(
        &self,
        repo_id: &RepoId,
        channel: &str,
        head: ChangeHash,
    ) -> PijulResult<()> {
        self.validate_channel_name(channel)?;

        // Get old head for the event
        let old_head = self.get_channel(repo_id, channel).await.ok().flatten();

        let key = self.channel_key(repo_id, channel);
        let value = head.to_hex();

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        debug!(repo_id = %repo_id, channel = channel, head = %head, "set channel head");

        // Emit event for gossip
        // Note: merkle is zeroed here since RefStore doesn't have pristine access.
        // Higher-level callers (like PijulStore) that have pristine access should
        // use set_channel_with_merkle() instead.
        let _ = self.event_tx.send(ChannelUpdateEvent {
            repo_id: *repo_id,
            channel: channel.to_string(),
            new_head: head,
            old_head,
            merkle: [0u8; 32],
        });

        Ok(())
    }

    /// Update a channel head with compare-and-swap semantics.
    ///
    /// Only updates if the current head matches `expected`. This prevents
    /// lost updates in concurrent scenarios.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name
    /// - `expected`: Expected current head (None if channel should not exist)
    /// - `new_head`: New head to set
    ///
    /// # Errors
    ///
    /// Returns `PijulError::ChannelConflict` if the current head doesn't match expected.
    #[instrument(skip(self))]
    pub async fn compare_and_set_channel(
        &self,
        repo_id: &RepoId,
        channel: &str,
        expected: Option<ChangeHash>,
        new_head: ChangeHash,
    ) -> PijulResult<()> {
        self.validate_channel_name(channel)?;

        let current = self.get_channel(repo_id, channel).await?;

        if current != expected {
            return Err(PijulError::ChannelConflict {
                expected: expected.map(|h| h.to_hex()),
                found: current.map(|h| h.to_hex()),
            });
        }

        self.set_channel(repo_id, channel, new_head).await
    }

    /// Delete a channel.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    /// - `channel`: Channel name to delete
    #[instrument(skip(self))]
    pub async fn delete_channel(&self, repo_id: &RepoId, channel: &str) -> PijulResult<()> {
        let key = self.channel_key(repo_id, channel);

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await?;

        debug!(repo_id = %repo_id, channel = channel, "deleted channel");
        Ok(())
    }

    /// List all channels for a repository.
    ///
    /// # Arguments
    ///
    /// - `repo_id`: Repository ID
    ///
    /// # Returns
    ///
    /// A list of (channel_name, head_hash) tuples. Empty channels have `None` as head.
    #[instrument(skip(self))]
    pub async fn list_channels(&self, repo_id: &RepoId) -> PijulResult<Vec<(String, Option<ChangeHash>)>> {
        let prefix = format!("{}{}:", KV_PREFIX_PIJUL_CHANNELS, repo_id);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: prefix.clone(),
                limit: Some(MAX_CHANNELS),
                continuation_token: None,
            })
            .await?;

        let mut channels = Vec::with_capacity(result.entries.len());
        for kv in result.entries {
            // Extract channel name from key
            let channel_name = kv.key.strip_prefix(&prefix).unwrap_or(&kv.key).to_string();

            // Parse hash (or None if empty channel marker)
            let head = if kv.value == EMPTY_CHANNEL_MARKER {
                None
            } else {
                Some(ChangeHash::from_hex(&kv.value).map_err(|e| PijulError::InvalidChange {
                    message: format!("invalid channel head hash: {}", e),
                })?)
            };

            channels.push((channel_name, head));
        }

        debug!(repo_id = %repo_id, count = channels.len(), "listed channels");
        Ok(channels)
    }

    /// Check if a channel exists (including empty channels).
    ///
    /// Returns `true` if the channel exists, even if it has no head yet.
    #[instrument(skip(self))]
    pub async fn channel_exists(&self, repo_id: &RepoId, channel: &str) -> PijulResult<bool> {
        let key = self.channel_key(repo_id, channel);

        match self.kv.read(ReadRequest { key, consistency: ReadConsistency::Linearizable }).await {
            Ok(result) => Ok(result.kv.is_some()),
            Err(crate::api::KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(PijulError::from(e)),
        }
    }

    /// Count channels for a repository.
    #[instrument(skip(self))]
    pub async fn count_channels(&self, repo_id: &RepoId) -> PijulResult<u32> {
        let channels = self.list_channels(repo_id).await?;
        Ok(channels.len() as u32)
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Build the KV key for a channel.
    fn channel_key(&self, repo_id: &RepoId, channel: &str) -> String {
        format!("{}{}:{}", KV_PREFIX_PIJUL_CHANNELS, repo_id, channel)
    }

    /// Validate a channel name.
    fn validate_channel_name(&self, channel: &str) -> PijulResult<()> {
        if channel.is_empty() {
            return Err(PijulError::InvalidChannelName {
                channel: channel.to_string(),
            });
        }

        if channel.len() > MAX_CHANNEL_NAME_LENGTH_BYTES as usize {
            return Err(PijulError::InvalidChannelName {
                channel: format!("channel name too long: {} > {}", channel.len(), MAX_CHANNEL_NAME_LENGTH_BYTES),
            });
        }

        // Disallow control characters and some problematic characters
        if channel.chars().any(|c| c.is_control() || c == '\0') {
            return Err(PijulError::InvalidChannelName {
                channel: channel.to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;

    fn test_repo_id() -> RepoId {
        RepoId([1u8; 32])
    }

    fn test_hash() -> ChangeHash {
        ChangeHash([2u8; 32])
    }

    fn test_hash_2() -> ChangeHash {
        ChangeHash([3u8; 32])
    }

    #[tokio::test]
    async fn test_set_and_get_channel() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulRefStore::new(kv);

        let repo_id = test_repo_id();
        let hash = test_hash();

        // Channel doesn't exist initially
        assert!(store.get_channel(&repo_id, "main").await.unwrap().is_none());

        // Set channel
        store.set_channel(&repo_id, "main", hash).await.unwrap();

        // Now it exists
        let result = store.get_channel(&repo_id, "main").await.unwrap();
        assert_eq!(result, Some(hash));
    }

    #[tokio::test]
    async fn test_list_channels() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulRefStore::new(kv);

        let repo_id = test_repo_id();

        // Set multiple channels
        store.set_channel(&repo_id, "main", test_hash()).await.unwrap();
        store.set_channel(&repo_id, "develop", test_hash_2()).await.unwrap();

        // List them
        let channels = store.list_channels(&repo_id).await.unwrap();
        assert_eq!(channels.len(), 2);

        let names: Vec<_> = channels.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"develop"));

        // Verify heads are present
        for (name, head) in &channels {
            assert!(head.is_some(), "channel {} should have a head", name);
        }
    }

    #[tokio::test]
    async fn test_empty_channel() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulRefStore::new(kv);

        let repo_id = test_repo_id();

        // Create an empty channel
        store.create_empty_channel(&repo_id, "empty").await.unwrap();

        // Channel should exist
        assert!(store.channel_exists(&repo_id, "empty").await.unwrap());

        // But get_channel should return None (no head)
        assert!(store.get_channel(&repo_id, "empty").await.unwrap().is_none());

        // List should include it with None head
        let channels = store.list_channels(&repo_id).await.unwrap();
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].0, "empty");
        assert!(channels[0].1.is_none());

        // Setting a head should work
        store.set_channel(&repo_id, "empty", test_hash()).await.unwrap();
        assert_eq!(store.get_channel(&repo_id, "empty").await.unwrap(), Some(test_hash()));
    }

    #[tokio::test]
    async fn test_compare_and_set() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulRefStore::new(kv);

        let repo_id = test_repo_id();
        let hash1 = test_hash();
        let hash2 = test_hash_2();

        // Set initial value
        store.set_channel(&repo_id, "main", hash1).await.unwrap();

        // CAS with correct expected value should succeed
        store.compare_and_set_channel(&repo_id, "main", Some(hash1), hash2).await.unwrap();

        // Verify update
        assert_eq!(store.get_channel(&repo_id, "main").await.unwrap(), Some(hash2));

        // CAS with wrong expected value should fail
        let result = store.compare_and_set_channel(&repo_id, "main", Some(hash1), hash1).await;
        assert!(matches!(result, Err(PijulError::ChannelConflict { .. })));
    }

    #[tokio::test]
    async fn test_delete_channel() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulRefStore::new(kv);

        let repo_id = test_repo_id();
        let hash = test_hash();

        // Set then delete
        store.set_channel(&repo_id, "main", hash).await.unwrap();
        store.delete_channel(&repo_id, "main").await.unwrap();

        // Should be gone
        assert!(store.get_channel(&repo_id, "main").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_invalid_channel_name() {
        let kv = Arc::new(DeterministicKeyValueStore::new());
        let store = PijulRefStore::new(kv);

        let repo_id = test_repo_id();
        let hash = test_hash();

        // Empty name should fail
        let result = store.set_channel(&repo_id, "", hash).await;
        assert!(matches!(result, Err(PijulError::InvalidChannelName { .. })));

        // Name with null byte should fail
        let result = store.set_channel(&repo_id, "test\0name", hash).await;
        assert!(matches!(result, Err(PijulError::InvalidChannelName { .. })));
    }
}
