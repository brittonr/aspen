//! ForgeNode - main coordinator for Forge operations.
//!
//! The ForgeNode ties together all Forge components and provides
//! a unified interface for repository operations.

use std::sync::Arc;

use aspen_blob::BlobStore;
use aspen_core::KeyValueStore;
use aspen_core::KeyValueStoreError;
use aspen_core::ReadConsistency;
use aspen_core::hlc::HLC;
use aspen_core::hlc::create_hlc;

use crate::cob::CobStore;
use crate::constants::KV_PREFIX_REPO_NAMES;
use crate::constants::KV_PREFIX_REPOS;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::git::GitBlobStore;
use crate::gossip::AnnouncementCallback;
use crate::gossip::ForgeGossipService;
use crate::identity::RepoId;
use crate::identity::RepoIdentity;
use crate::refs::RefStore;
use crate::sync::SyncService;
use crate::types::SignedObject;

/// Main coordinator for Forge operations.
///
/// Provides a unified interface for:
/// - Git object storage and retrieval
/// - Collaborative object management (issues, patches)
/// - Ref management (branches, tags)
/// - Object synchronization
///
/// # Example
///
/// ```ignore
/// let forge = ForgeNode::new(blob_store, kv_store, secret_key);
///
/// // Create a repository
/// let repo = forge.create_repo("my-project", vec![my_key], 1).await?;
///
/// // Create a commit
/// let tree = forge.git.create_tree(&[...]).await?;
/// let commit = forge.git.commit(tree, vec![], "Initial commit").await?;
///
/// // Push to main
/// forge.refs.set(&repo.id, "heads/main", commit).await?;
/// ```
pub struct ForgeNode<B: BlobStore, K: KeyValueStore + ?Sized> {
    /// Git object storage.
    pub git: GitBlobStore<B>,

    /// Collaborative object storage.
    pub cobs: CobStore<B, K>,

    /// Ref storage.
    pub refs: RefStore<K>,

    /// Sync service.
    pub sync: SyncService<B>,

    /// KV store reference.
    kv: Arc<K>,

    /// Secret key for signing.
    secret_key: iroh::SecretKey,

    /// Hybrid Logical Clock for deterministic timestamp ordering.
    hlc: HLC,

    /// Optional gossip service for real-time announcements.
    gossip: Option<Arc<ForgeGossipService>>,
}

impl<B: BlobStore, K: KeyValueStore + ?Sized> ForgeNode<B, K> {
    /// Create a new ForgeNode.
    ///
    /// # Arguments
    ///
    /// * `blobs` - Blob storage backend
    /// * `kv` - Key-value store backend
    /// * `secret_key` - Ed25519 secret key for signing
    pub fn new(blobs: Arc<B>, kv: Arc<K>, secret_key: iroh::SecretKey) -> Self {
        let node_id = hex::encode(secret_key.public().as_bytes());
        let hlc = create_hlc(&node_id);
        Self {
            git: GitBlobStore::new(blobs.clone(), secret_key.clone(), &node_id),
            cobs: CobStore::new(blobs.clone(), kv.clone(), secret_key.clone(), &node_id),
            refs: RefStore::new(kv.clone()),
            sync: SyncService::new(blobs),
            kv,
            secret_key,
            hlc,
            gossip: None,
        }
    }

    /// Enable gossip integration with the given iroh-gossip instance.
    ///
    /// This spawns background tasks for:
    /// - Broadcasting ref and COB updates as announcements
    /// - Receiving announcements from other nodes
    /// - Rate limiting incoming announcements
    ///
    /// # Arguments
    ///
    /// - `gossip`: The iroh-gossip instance to use
    /// - `handler`: Optional callback for incoming announcements (for auto-sync)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let handler = Arc::new(ForgeAnnouncementHandler::new(sync_tx, seeding_tx));
    /// forge.enable_gossip(gossip, Some(handler)).await?;
    /// ```
    pub async fn enable_gossip(
        &mut self,
        gossip: Arc<iroh_gossip::net::Gossip>,
        handler: Option<Arc<dyn AnnouncementCallback>>,
    ) -> ForgeResult<()> {
        let service = ForgeGossipService::spawn(
            gossip,
            self.secret_key.clone(),
            self.refs.subscribe(),
            self.cobs.subscribe(),
            handler,
        )
        .await
        .map_err(|e| ForgeError::GossipError { message: e.to_string() })?;

        self.gossip = Some(service);

        tracing::info!("forge gossip integration enabled");
        Ok(())
    }

    /// Check if gossip is enabled.
    pub fn has_gossip(&self) -> bool {
        self.gossip.is_some()
    }

    /// Get the gossip service if enabled.
    pub fn gossip(&self) -> Option<&Arc<ForgeGossipService>> {
        self.gossip.as_ref()
    }

    /// Set the announcement handler for gossip.
    ///
    /// This can be called after `enable_gossip` to register a handler
    /// (e.g., CI trigger handler) without needing mutable access to ForgeNode.
    ///
    /// Returns `Ok(())` if gossip is enabled and handler was set,
    /// or `Err` if gossip is not enabled.
    pub async fn set_gossip_handler(&self, handler: Option<Arc<dyn AnnouncementCallback>>) -> ForgeResult<()> {
        if let Some(gossip) = &self.gossip {
            gossip.set_handler(handler).await;
            Ok(())
        } else {
            Err(ForgeError::GossipError {
                message: "gossip not enabled".to_string(),
            })
        }
    }

    /// Subscribe to a repository's gossip topic for CI triggers.
    ///
    /// This enables receiving RefUpdate announcements from other nodes,
    /// which is required for multi-node CI auto-triggering.
    ///
    /// Note: This only subscribes to gossip. For full seeding (including
    /// blob replication), use `start_seeding()` instead.
    pub async fn subscribe_repo_gossip(&self, repo_id: &RepoId) -> ForgeResult<()> {
        if let Some(gossip) = &self.gossip {
            gossip.subscribe_repo(repo_id).await?;
            tracing::debug!(repo_id = %repo_id.to_hex(), "subscribed to repo gossip topic");
            Ok(())
        } else {
            Err(ForgeError::GossipNotInitialized)
        }
    }

    /// Unsubscribe from a repository's gossip topic.
    ///
    /// This stops receiving RefUpdate announcements for the repo.
    /// Used when CI watch is disabled for a repository.
    pub async fn unsubscribe_repo_gossip(&self, repo_id: &RepoId) -> ForgeResult<()> {
        if let Some(gossip) = &self.gossip {
            gossip.unsubscribe_repo(repo_id).await?;
            tracing::debug!(repo_id = %repo_id.to_hex(), "unsubscribed from repo gossip topic");
            Ok(())
        } else {
            Err(ForgeError::GossipNotInitialized)
        }
    }

    /// Get the public key of this node.
    pub fn public_key(&self) -> iroh::PublicKey {
        self.secret_key.public()
    }

    /// Get the secret key of this node.
    ///
    /// # Security
    ///
    /// This method exposes the node's signing key. Only use for:
    /// - Exporting keys for offline signing
    /// - Migration between nodes
    /// - Backup purposes
    ///
    /// Never expose this key over the network or store it unencrypted.
    pub fn secret_key(&self) -> &iroh::SecretKey {
        &self.secret_key
    }

    /// Get a reference to the KV store.
    ///
    /// Used for creating git bridge components (HashMappingStore).
    pub fn kv(&self) -> &Arc<K> {
        &self.kv
    }

    // ========================================================================
    // Repository Management
    // ========================================================================

    /// Create a new repository.
    ///
    /// # Arguments
    ///
    /// - `name`: Repository name
    /// - `delegates`: Public keys of delegates who can update canonical refs
    /// - `threshold`: Number of delegate signatures required for updates
    ///
    /// # Returns
    ///
    /// The repository identity with its computed ID.
    pub async fn create_repo(
        &self,
        name: impl Into<String>,
        delegates: Vec<iroh::PublicKey>,
        threshold: u32,
    ) -> ForgeResult<RepoIdentity> {
        let name = name.into();

        // Check if a repo with this name already exists (name-based duplicate check)
        let name_key = format!("{}{}", KV_PREFIX_REPO_NAMES, name);
        let name_exists = match self
            .kv
            .read(aspen_core::ReadRequest {
                key: name_key.clone(),
                consistency: ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r.kv.is_some(),
            Err(KeyValueStoreError::NotFound { .. }) => false,
            Err(e) => return Err(ForgeError::from(e)),
        };

        if name_exists {
            return Err(ForgeError::RepoNameAlreadyExists { name });
        }

        let identity = RepoIdentity::new(name, delegates, threshold)?;
        let repo_id = identity.repo_id();

        // Sign and store identity with HLC timestamp
        let signed = SignedObject::new(identity.clone(), &self.secret_key, &self.hlc)?;
        let bytes = signed.to_bytes();

        let identity_key = format!("{}{}:identity", KV_PREFIX_REPOS, repo_id.to_hex());

        // Store both the identity and the name index atomically via SetMulti
        self.kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::SetMulti {
                    pairs: vec![
                        (identity_key, base64::Engine::encode(&base64::prelude::BASE64_STANDARD, &bytes)),
                        (name_key, repo_id.to_hex()),
                    ],
                },
            })
            .await?;

        // Announce repo creation via gossip if enabled
        if let Some(ref gossip) = self.gossip
            && let Err(e) = gossip.announce_repo_created(&repo_id, &identity.name).await
        {
            tracing::warn!(repo_id = %repo_id.to_hex(), "failed to announce repo creation: {}", e);
        }

        Ok(identity)
    }

    /// Get a repository's identity.
    pub async fn get_repo(&self, repo_id: &RepoId) -> ForgeResult<RepoIdentity> {
        let key = format!("{}{}:identity", KV_PREFIX_REPOS, repo_id.to_hex());

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(KeyValueStoreError::NotFound { .. }) => {
                return Err(ForgeError::RepoNotFound {
                    repo_id: repo_id.to_hex(),
                });
            }
            Err(e) => return Err(ForgeError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(encoded) => {
                let bytes = base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &encoded).map_err(|e| {
                    ForgeError::InvalidRepoIdentity {
                        message: format!("invalid base64: {}", e),
                    }
                })?;

                let signed: SignedObject<RepoIdentity> = SignedObject::from_bytes(&bytes)?;
                signed.verify()?;

                Ok(signed.payload)
            }
            None => Err(ForgeError::RepoNotFound {
                repo_id: repo_id.to_hex(),
            }),
        }
    }

    /// Check if a repository exists.
    pub async fn repo_exists(&self, repo_id: &RepoId) -> ForgeResult<bool> {
        let key = format!("{}{}:identity", KV_PREFIX_REPOS, repo_id.to_hex());

        match self
            .kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => Ok(r.kv.is_some()),
            Err(KeyValueStoreError::NotFound { .. }) => Ok(false),
            Err(e) => Err(ForgeError::from(e)),
        }
    }

    // ========================================================================
    // Peer Tracking (Seeding)
    // ========================================================================

    /// Add a peer as a seeder for a repository.
    ///
    /// Seeders are nodes that have a copy of the repository and can provide
    /// objects during P2P sync.
    pub async fn add_seeding_peer(&self, repo_id: &RepoId, peer: iroh::PublicKey) -> ForgeResult<()> {
        use crate::constants::KV_PREFIX_SEEDING;

        let key = format!("{}{}", KV_PREFIX_SEEDING, repo_id.to_hex());

        // Get existing seeders
        let mut seeders = self.get_seeding_peers(repo_id).await.unwrap_or_default();

        // Add new peer if not already present
        if !seeders.contains(&peer) {
            seeders.push(peer);

            // Serialize as JSON array of hex public keys
            let value: Vec<String> = seeders.iter().map(|k| k.to_string()).collect();
            let json =
                serde_json::to_string(&value).map_err(|e| ForgeError::InvalidObject { message: e.to_string() })?;

            self.kv
                .write(aspen_core::WriteRequest {
                    command: aspen_core::WriteCommand::Set { key, value: json },
                })
                .await?;
        }

        Ok(())
    }

    /// Remove a peer from the seeders list.
    pub async fn remove_seeding_peer(&self, repo_id: &RepoId, peer: &iroh::PublicKey) -> ForgeResult<()> {
        use crate::constants::KV_PREFIX_SEEDING;

        let key = format!("{}{}", KV_PREFIX_SEEDING, repo_id.to_hex());

        let mut seeders = self.get_seeding_peers(repo_id).await.unwrap_or_default();

        if let Some(pos) = seeders.iter().position(|k| k == peer) {
            seeders.remove(pos);

            let value: Vec<String> = seeders.iter().map(|k| k.to_string()).collect();
            let json =
                serde_json::to_string(&value).map_err(|e| ForgeError::InvalidObject { message: e.to_string() })?;

            self.kv
                .write(aspen_core::WriteRequest {
                    command: aspen_core::WriteCommand::Set { key, value: json },
                })
                .await?;
        }

        Ok(())
    }

    /// Get all seeders for a repository.
    pub async fn get_seeding_peers(&self, repo_id: &RepoId) -> ForgeResult<Vec<iroh::PublicKey>> {
        use crate::constants::KV_PREFIX_SEEDING;

        let key = format!("{}{}", KV_PREFIX_SEEDING, repo_id.to_hex());

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(KeyValueStoreError::NotFound { .. }) => return Ok(vec![]),
            Err(e) => return Err(ForgeError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(json) => {
                let keys: Vec<String> =
                    serde_json::from_str(&json).map_err(|e| ForgeError::InvalidObject { message: e.to_string() })?;

                let mut seeders = Vec::new();
                for key_str in keys {
                    if let Ok(pk) = key_str.parse() {
                        seeders.push(pk);
                    }
                }
                Ok(seeders)
            }
            None => Ok(vec![]),
        }
    }

    /// Start seeding a repository.
    ///
    /// This will:
    /// 1. Subscribe to the repository's gossip topic (if gossip enabled)
    /// 2. Add ourselves to the seeders list
    /// 3. Announce that we are seeding (if gossip enabled)
    ///
    /// Other nodes will receive the seeding announcement and can request
    /// objects from us during P2P sync.
    pub async fn start_seeding(&self, repo_id: &RepoId) -> ForgeResult<()> {
        // Subscribe to repo topic first
        if let Some(ref gossip) = self.gossip {
            gossip.subscribe_repo(repo_id).await?;
        }

        // Add ourselves as a seeder
        self.add_seeding_peer(repo_id, self.public_key()).await?;

        // Announce seeding
        if let Some(ref gossip) = self.gossip {
            gossip.announce_seeding(repo_id).await?;
        }

        tracing::info!(repo_id = %repo_id.to_hex(), "started seeding repository");
        Ok(())
    }

    /// Stop seeding a repository.
    ///
    /// This will:
    /// 1. Announce that we are no longer seeding (if gossip enabled)
    /// 2. Remove ourselves from the seeders list
    /// 3. Unsubscribe from the repository's gossip topic (if gossip enabled)
    pub async fn stop_seeding(&self, repo_id: &RepoId) -> ForgeResult<()> {
        // Announce unseeding first (while still subscribed)
        if let Some(ref gossip) = self.gossip {
            gossip.announce_unseeding(repo_id).await?;
        }

        // Remove ourselves as a seeder
        self.remove_seeding_peer(repo_id, &self.public_key()).await?;

        // Unsubscribe from repo topic
        if let Some(ref gossip) = self.gossip {
            gossip.unsubscribe_repo(repo_id).await?;
        }

        tracing::info!(repo_id = %repo_id.to_hex(), "stopped seeding repository");
        Ok(())
    }

    /// Shutdown the ForgeNode, including gossip service.
    ///
    /// This gracefully stops all background tasks and releases resources.
    pub async fn shutdown(&mut self) -> ForgeResult<()> {
        if let Some(gossip) = self.gossip.take() {
            // Shutdown takes Arc<Self>, so we can call it directly
            if let Err(e) = gossip.shutdown().await {
                tracing::warn!("gossip shutdown error: {}", e);
            }
        }

        tracing::info!("forge node shutdown complete");
        Ok(())
    }

    // ========================================================================
    // Federation Settings Persistence
    // ========================================================================

    /// Set federation settings for a resource.
    ///
    /// Persists the settings to KV storage. Settings control how this resource
    /// participates in cross-cluster federation (Public, AllowList, or Disabled).
    ///
    /// Key format: `forge:federation:settings:{fed_id}`
    /// Value: JSON-serialized `FederationSettings`
    pub async fn set_federation_settings(
        &self,
        fed_id: &aspen_cluster::federation::FederatedId,
        settings: &aspen_cluster::federation::FederationSettings,
    ) -> ForgeResult<()> {
        use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;

        let key = format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, fed_id);

        let value =
            serde_json::to_string(settings).map_err(|e| ForgeError::InvalidObject { message: e.to_string() })?;

        self.kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value },
            })
            .await?;

        tracing::debug!(fed_id = %fed_id.short(), mode = ?settings.mode, "persisted federation settings");
        Ok(())
    }

    /// Get federation settings for a resource.
    ///
    /// Returns `None` if no settings have been persisted for this resource,
    /// meaning federation is effectively disabled.
    pub async fn get_federation_settings(
        &self,
        fed_id: &aspen_cluster::federation::FederatedId,
    ) -> ForgeResult<Option<aspen_cluster::federation::FederationSettings>> {
        use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;

        let key = format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, fed_id);

        let result = match self
            .kv
            .read(aspen_core::ReadRequest {
                key,
                consistency: ReadConsistency::Linearizable,
            })
            .await
        {
            Ok(r) => r,
            Err(KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(ForgeError::from(e)),
        };

        match result.kv.map(|kv| kv.value) {
            Some(json) => {
                let settings: aspen_cluster::federation::FederationSettings =
                    serde_json::from_str(&json).map_err(|e| ForgeError::InvalidObject { message: e.to_string() })?;
                Ok(Some(settings))
            }
            None => Ok(None),
        }
    }

    /// Delete federation settings for a resource.
    ///
    /// After deletion, the resource is no longer federated (effectively Disabled mode).
    pub async fn delete_federation_settings(&self, fed_id: &aspen_cluster::federation::FederatedId) -> ForgeResult<()> {
        use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;

        let key = format!("{}{}", KV_PREFIX_FEDERATION_SETTINGS, fed_id);

        // Delete returns OK even if key doesn't exist (idempotent)
        self.kv.delete(aspen_core::DeleteRequest { key }).await?;

        tracing::debug!(fed_id = %fed_id.short(), "deleted federation settings");
        Ok(())
    }

    /// Scan and count federated resources.
    ///
    /// Returns the count of resources that have federation settings with
    /// `mode != Disabled`. This is useful for federation status reporting.
    ///
    /// Tiger Style: Limits scan to 10,000 results (MAX_SCAN_RESULTS).
    pub async fn count_federated_resources(&self) -> ForgeResult<u32> {
        use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;

        let results = self
            .kv
            .scan(aspen_core::ScanRequest {
                prefix: KV_PREFIX_FEDERATION_SETTINGS.to_string(),
                limit: Some(10_000), // Tiger Style: bounded scan
                continuation_token: None,
            })
            .await?;

        let mut count = 0u32;
        for kv in results.entries {
            // Parse settings and check if not Disabled
            if let Ok(settings) = serde_json::from_str::<aspen_cluster::federation::FederationSettings>(&kv.value)
                && !matches!(settings.mode, aspen_cluster::federation::FederationMode::Disabled)
            {
                count = count.saturating_add(1);
            }
        }

        Ok(count)
    }

    /// List all federated resources with their settings.
    ///
    /// Returns resources that have federation settings with `mode != Disabled`.
    /// Supports pagination via `start_after` and `limit`.
    ///
    /// # Arguments
    ///
    /// * `start_after` - Optional key to start after (for pagination)
    /// * `limit` - Maximum number of results (capped at 1000)
    pub async fn list_federated_resources(
        &self,
        start_after: Option<String>,
        limit: u32,
    ) -> ForgeResult<Vec<(aspen_cluster::federation::FederatedId, aspen_cluster::federation::FederationSettings)>> {
        use crate::constants::KV_PREFIX_FEDERATION_SETTINGS;

        // Tiger Style: Cap limit to prevent unbounded results
        let limit = limit.min(1000);

        let results = self
            .kv
            .scan(aspen_core::ScanRequest {
                prefix: KV_PREFIX_FEDERATION_SETTINGS.to_string(),
                limit: Some(limit),
                continuation_token: start_after,
            })
            .await?;

        let mut federated = Vec::new();
        for kv in results.entries {
            // Extract FederatedId from key suffix
            let fed_id_str = kv.key.strip_prefix(KV_PREFIX_FEDERATION_SETTINGS).unwrap_or(&kv.key);

            // Parse FederatedId and settings
            if let (Ok(fed_id), Ok(settings)) = (
                fed_id_str.parse::<aspen_cluster::federation::FederatedId>(),
                serde_json::from_str::<aspen_cluster::federation::FederationSettings>(&kv.value),
            ) {
                // Only include active federation (not Disabled)
                if !matches!(settings.mode, aspen_cluster::federation::FederationMode::Disabled) {
                    federated.push((fed_id, settings));
                }
            }
        }

        Ok(federated)
    }

    // ========================================================================
    // High-Level Operations
    // ========================================================================

    /// Initialize a repository with an initial commit.
    ///
    /// Creates an empty tree, initial commit, and sets heads/main.
    pub async fn init_repo(&self, repo_id: &RepoId, message: &str) -> ForgeResult<blake3::Hash> {
        // Create empty tree
        let tree = self.git.create_tree(&[]).await?;

        // Create initial commit
        let commit = self.git.commit(tree, vec![], message).await?;

        // Set main branch
        self.refs.set(repo_id, "heads/main", commit).await?;

        Ok(commit)
    }

    /// Get the current commit for the default branch.
    pub async fn get_head(&self, repo_id: &RepoId) -> ForgeResult<Option<blake3::Hash>> {
        let identity = self.get_repo(repo_id).await?;
        let ref_name = format!("heads/{}", identity.default_branch);
        self.refs.get(repo_id, &ref_name).await
    }
}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;
    use aspen_core::DeterministicKeyValueStore;

    use super::*;

    async fn create_test_node() -> ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore> {
        let blobs = Arc::new(InMemoryBlobStore::new());
        let kv = DeterministicKeyValueStore::new();
        let secret_key = iroh::SecretKey::generate(&mut rand::rng());
        ForgeNode::new(blobs, kv, secret_key)
    }

    #[tokio::test]
    async fn test_create_repo() {
        let node = create_test_node().await;

        let identity = node.create_repo("my-project", vec![node.public_key()], 1).await.expect("should create repo");

        assert_eq!(identity.name, "my-project");
        assert_eq!(identity.delegates.len(), 1);

        // Should be able to retrieve
        let retrieved = node.get_repo(&identity.repo_id()).await.expect("should get");
        assert_eq!(retrieved.name, identity.name);
    }

    #[tokio::test]
    async fn test_repo_already_exists() {
        let node = create_test_node().await;

        let _identity = node.create_repo("my-project", vec![node.public_key()], 1).await.expect("should create repo");

        // Try to get a non-existent repo
        let fake_id = RepoId::from_hash(blake3::hash(b"nonexistent"));
        assert!(node.get_repo(&fake_id).await.is_err());
    }

    #[tokio::test]
    async fn test_init_repo() {
        let node = create_test_node().await;

        let identity = node.create_repo("my-project", vec![node.public_key()], 1).await.expect("should create repo");

        let commit = node.init_repo(&identity.repo_id(), "Initial commit").await.expect("should init");

        // Should be able to get head
        let head = node.get_head(&identity.repo_id()).await.expect("should get head");
        assert_eq!(head, Some(commit));
    }
}
