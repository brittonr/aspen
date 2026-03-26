//! ForgeNode - main coordinator for Forge operations.
//!
//! The ForgeNode ties together all Forge components and provides
//! a unified interface for repository operations.

use std::sync::Arc;

use aspen_blob::prelude::*;
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

/// Result of a dry-run merge check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MergeCheckResult {
    /// Whether the patch can be merged right now.
    pub mergeable: bool,
    /// Which merge strategies are available (e.g. `["merge", "squash", "fast-forward"]`).
    pub available_strategies: Vec<String>,
    /// Conflicting file paths (empty if no conflicts).
    pub conflicts: Vec<String>,
    /// Whether branch protection rules are satisfied.
    pub protection_satisfied: bool,
    /// Reason protection is blocking (None if satisfied).
    pub protection_reason: Option<String>,
}

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

    /// Enable gossip with automatic DAG sync.
    ///
    /// This is the recommended way to set up gossip — it creates the
    /// announcement handler, spawns a [`DagSyncWorker`] to process
    /// incoming sync requests, and registers everything with the gossip
    /// service.
    ///
    /// When a peer announces a ref update or COB change, the worker
    /// automatically connects to the peer via DAG sync over QUIC and
    /// pulls the missing objects.
    ///
    /// # Arguments
    ///
    /// * `gossip` - The iroh-gossip instance
    /// * `endpoint` - Iroh endpoint for outbound DAG sync connections
    /// * `cancel` - Cancellation token to stop the worker on shutdown
    ///
    /// # Returns
    ///
    /// The `JoinHandle` for the background worker task.
    pub async fn enable_gossip_with_dag_sync(
        &mut self,
        gossip: Arc<iroh_gossip::net::Gossip>,
        endpoint: iroh::Endpoint,
        cancel: tokio_util::sync::CancellationToken,
    ) -> ForgeResult<tokio::task::JoinHandle<()>>
    where
        B: Send + Sync + 'static,
    {
        let (handler, sync_rx, _seeding_rx) = crate::gossip::ForgeAnnouncementHandler::with_channels(256);

        let sync_service = Arc::new(SyncService::new(self.sync.blobs().clone()));
        let worker = crate::sync::DagSyncWorker::new(sync_service, endpoint);
        let worker_handle = worker.spawn(sync_rx, cancel);

        self.enable_gossip(gossip, Some(Arc::new(handler))).await?;

        tracing::info!("forge dag sync worker started");
        Ok(worker_handle)
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

    /// Create a `DagSyncProtocolHandler` for this node's blob store.
    ///
    /// Register the returned handler on the iroh Router to serve
    /// DAG sync requests from peers:
    ///
    /// ```ignore
    /// let handler = forge.dag_sync_handler();
    /// router_builder.dag_sync(handler);
    /// ```
    pub fn dag_sync_handler(&self) -> aspen_dag::DagSyncProtocolHandler
    where B: Send + Sync + 'static {
        let sync = Arc::new(SyncService::new(self.sync.blobs().clone()));
        sync.into_dag_sync_handler()
    }

    /// Get the public key of this node.
    pub fn public_key(&self) -> iroh::PublicKey {
        self.secret_key.public()
    }

    /// Write a key-value entry to the underlying KV store.
    ///
    /// Used by federation sync to persist synced ref state.
    pub async fn write_kv(&self, key: &str, value: &str) -> ForgeResult<()> {
        self.kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set {
                    key: key.to_string(),
                    value: value.to_string(),
                },
            })
            .await?;
        Ok(())
    }

    /// Resolve the signing key and npub for an operation.
    ///
    /// If a `UserContext` is provided, uses the user's assigned key and npub.
    /// Otherwise falls back to the node's default key with no npub.
    pub fn signing_context<'a>(
        &'a self,
        user: Option<&'a crate::identity::nostr_mapping::UserContext>,
    ) -> (&'a iroh::SecretKey, Option<&'a str>) {
        match user {
            Some(ctx) => (&ctx.signing_key, Some(&ctx.npub)),
            None => (&self.secret_key, None),
        }
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

    /// List all repositories on this node.
    ///
    /// Scans the KV store for repo identity entries. Returns up to 1000 repos.
    pub async fn list_repos(&self) -> ForgeResult<Vec<RepoIdentity>> {
        let scan_result = self
            .kv
            .scan(aspen_core::ScanRequest {
                prefix: KV_PREFIX_REPO_NAMES.to_string(),
                limit_results: Some(1000),
                continuation_token: None,
            })
            .await
            .map_err(ForgeError::from)?;

        let mut repos = Vec::new();
        for kv in scan_result.entries {
            // The name index stores repo_id hex as the value
            let repo_id_hex = kv.value;
            if let Ok(repo_id) = RepoId::from_hex(&repo_id_hex) {
                match self.get_repo(&repo_id).await {
                    Ok(identity) => repos.push(identity),
                    Err(ForgeError::RepoNotFound { .. }) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(repos)
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
                limit_results: Some(10_000), // Tiger Style: bounded scan
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
                limit_results: Some(limit),
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
    // Fork Operations
    // ========================================================================

    /// Fork a repository, creating a new repo linked to the upstream.
    ///
    /// The fork:
    /// 1. Creates a new `RepoIdentity` with `ForkInfo` pointing to the upstream
    /// 2. Copies all refs from the upstream repository
    /// 3. Starts seeding the fork (if gossip is enabled)
    ///
    /// Git objects are NOT duplicated — they're shared via iroh-blobs content
    /// addressing. Only the refs (branch/tag pointers) are copied.
    ///
    /// # Arguments
    ///
    /// * `upstream_repo_id` - Repository to fork from
    /// * `name` - Name for the new fork
    /// * `delegates` - Delegates for the fork (typically the forking user)
    /// * `threshold` - Signature threshold for the fork
    ///
    /// # Errors
    ///
    /// * `ForgeError::RepoNotFound` if the upstream doesn't exist
    /// * `ForgeError::RepoNameAlreadyExists` if the fork name is taken
    pub async fn fork_repo(
        &self,
        upstream_repo_id: &RepoId,
        name: impl Into<String>,
        delegates: Vec<iroh::PublicKey>,
        threshold: u32,
    ) -> ForgeResult<RepoIdentity> {
        let name = name.into();

        // Verify upstream exists
        let upstream = self.get_repo(upstream_repo_id).await?;

        // Create the fork identity with ForkInfo
        let fork_info = crate::identity::ForkInfo {
            upstream_repo_id: *upstream_repo_id,
            upstream_cluster: None, // Same cluster
        };

        let mut identity = RepoIdentity::new(name, delegates, threshold)?
            .with_fork_info(fork_info)
            .with_default_branch(upstream.default_branch.clone());

        if let Some(desc) = &upstream.description {
            identity = identity.with_description(format!("Fork of {}: {}", upstream.name, desc))?;
        }

        let repo_id = identity.repo_id();

        // Check if name already exists
        let name_key = format!("{}{}", crate::constants::KV_PREFIX_REPO_NAMES, identity.name);
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
            return Err(ForgeError::RepoNameAlreadyExists {
                name: identity.name.clone(),
            });
        }

        // Store the fork identity
        let signed = SignedObject::new(identity.clone(), &self.secret_key, &self.hlc)?;
        let bytes = signed.to_bytes();
        let identity_key = format!("{}{}:identity", crate::constants::KV_PREFIX_REPOS, repo_id.to_hex());

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

        // Copy all refs from upstream
        let upstream_refs = self.refs.list(upstream_repo_id).await?;
        for (ref_name, hash) in &upstream_refs {
            self.refs.set(&repo_id, ref_name, *hash).await?;
        }

        // Start seeding if gossip is available
        if self.gossip.is_some()
            && let Err(e) = self.start_seeding(&repo_id).await
        {
            tracing::warn!(repo_id = %repo_id.to_hex(), "failed to start seeding fork: {}", e);
        }

        tracing::info!(
            fork_id = %repo_id.to_hex(),
            upstream_id = %upstream_repo_id.to_hex(),
            "forked repository"
        );

        Ok(identity)
    }

    // ========================================================================
    // Mirror Operations
    // ========================================================================

    /// Set mirror configuration for a repository.
    ///
    /// Persists the config to KV at `forge:mirror:{repo_id}`.
    /// The interval is clamped to [60, 3600] seconds.
    ///
    /// # Errors
    ///
    /// * `ForgeError::MirrorLimitReached` if max mirrors exceeded
    /// * `ForgeError::RepoNotFound` if the repo doesn't exist
    pub async fn set_mirror_config(&self, repo_id: &RepoId, config: &crate::mirror::MirrorConfig) -> ForgeResult<()> {
        use crate::constants::KV_PREFIX_MIRROR;
        use crate::constants::MAX_MIRRORS_PER_NODE;

        // Check mirror count limit (only for new mirrors)
        let existing = self.get_mirror_config(repo_id).await?;
        if existing.is_none() {
            let count = self.count_mirrors().await?;
            if count >= MAX_MIRRORS_PER_NODE {
                return Err(ForgeError::MirrorLimitReached {
                    count,
                    max: MAX_MIRRORS_PER_NODE,
                });
            }
        }

        let key = format!("{}{}", KV_PREFIX_MIRROR, repo_id.to_hex());
        let value = serde_json::to_string(config).map_err(|e| ForgeError::Serialization { message: e.to_string() })?;

        self.kv
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value },
            })
            .await?;

        tracing::debug!(repo_id = %repo_id.to_hex(), interval = config.interval_secs, "mirror config set");
        Ok(())
    }

    /// Get mirror configuration for a repository.
    pub async fn get_mirror_config(&self, repo_id: &RepoId) -> ForgeResult<Option<crate::mirror::MirrorConfig>> {
        use crate::constants::KV_PREFIX_MIRROR;

        let key = format!("{}{}", KV_PREFIX_MIRROR, repo_id.to_hex());

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
                let config: crate::mirror::MirrorConfig =
                    serde_json::from_str(&json).map_err(|e| ForgeError::InvalidObject { message: e.to_string() })?;
                Ok(Some(config))
            }
            None => Ok(None),
        }
    }

    /// Delete mirror configuration for a repository.
    pub async fn delete_mirror_config(&self, repo_id: &RepoId) -> ForgeResult<()> {
        use crate::constants::KV_PREFIX_MIRROR;

        let key = format!("{}{}", KV_PREFIX_MIRROR, repo_id.to_hex());
        self.kv.delete(aspen_core::DeleteRequest { key }).await?;

        tracing::debug!(repo_id = %repo_id.to_hex(), "mirror config deleted");
        Ok(())
    }

    /// Get mirror status for a repository.
    pub async fn get_mirror_status(&self, repo_id: &RepoId) -> ForgeResult<Option<crate::mirror::MirrorStatus>> {
        let config = self.get_mirror_config(repo_id).await?;
        match config {
            Some(config) => {
                let now_ms =
                    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
                        as u64;
                let is_due = config.is_due(now_ms);
                Ok(Some(crate::mirror::MirrorStatus { config, is_due }))
            }
            None => Ok(None),
        }
    }

    /// Count active mirrors on this node.
    ///
    /// Tiger Style: Bounded scan with MAX_MIRRORS_PER_NODE + 1 limit.
    async fn count_mirrors(&self) -> ForgeResult<u32> {
        use crate::constants::KV_PREFIX_MIRROR;
        use crate::constants::MAX_MIRRORS_PER_NODE;

        let results = self
            .kv
            .scan(aspen_core::ScanRequest {
                prefix: KV_PREFIX_MIRROR.to_string(),
                limit_results: Some(MAX_MIRRORS_PER_NODE.saturating_add(1)),
                continuation_token: None,
            })
            .await?;

        Ok(results.entries.len() as u32)
    }

    // ========================================================================
    // High-Level Operations
    // ========================================================================

    /// Read a file from a specific commit.
    ///
    /// Walks commit → tree → blob to read file content at a specific path.
    /// Supports nested paths by traversing subdirectory trees.
    ///
    /// # Arguments
    ///
    /// * `repo_id` - Repository containing the commit
    /// * `commit_hash` - Commit to read from
    /// * `path` - File path (e.g., "README.md" or "dir/subdir/file.txt")
    ///
    /// # Returns
    ///
    /// * `Ok(Some(bytes))` - File content
    /// * `Ok(None)` - File not found at this path
    /// * `Err(...)` - Git object fetch/parse error
    ///
    /// # Example
    ///
    /// ```ignore
    /// let content = forge.read_file_at_commit(&repo_id, &commit_hash, ".aspen/ci.ncl").await?;
    /// if let Some(config_bytes) = content {
    ///     // Parse CI config
    /// }
    /// ```
    pub async fn read_file_at_commit(
        &self,
        _repo_id: &RepoId,
        commit_hash: &blake3::Hash,
        path: &str,
    ) -> ForgeResult<Option<Vec<u8>>> {
        // Get the commit
        let commit = self.git.get_commit(commit_hash).await?;

        // Get the root tree
        let mut current_tree = self.git.get_tree(&commit.tree()).await?;

        // Split path by '/' and walk through subdirectories
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if components.is_empty() {
            return Ok(None);
        }

        // Walk through all components except the last
        for component in &components[..components.len() - 1] {
            // Find matching directory entry
            let entry = current_tree.entries.iter().find(|e| e.name == *component && e.is_directory());

            match entry {
                Some(dir_entry) => {
                    // Recurse into subdirectory
                    current_tree = self.git.get_tree(&dir_entry.hash()).await?;
                }
                None => {
                    // Directory component not found
                    return Ok(None);
                }
            }
        }

        // Handle the final component (the file name)
        let file_name = components[components.len() - 1];
        let file_entry = current_tree.entries.iter().find(|e| e.name == file_name && e.is_file());

        match file_entry {
            Some(entry) => {
                // Read the blob content
                let content = self.git.get_blob(&entry.hash()).await?;
                Ok(Some(content))
            }
            None => {
                // File not found
                Ok(None)
            }
        }
    }

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

    // ========================================================================
    // Diff Operations
    // ========================================================================

    /// Compute a structured diff between two commits.
    ///
    /// Resolves each commit to its root tree and produces a list of
    /// `DiffEntry` records showing which files were added, removed,
    /// or modified.
    ///
    /// # Arguments
    ///
    /// * `commit_a` - Old (base) commit
    /// * `commit_b` - New (head) commit
    /// * `include_content` - Whether to load file content for each entry
    pub async fn diff_commits(
        &self,
        commit_a: &blake3::Hash,
        commit_b: &blake3::Hash,
        include_content: bool,
    ) -> ForgeResult<crate::git::DiffResult> {
        let opts = crate::git::DiffOptions { include_content };
        crate::git::diff_commits(&self.git, commit_a, commit_b, &opts).await
    }

    /// Compute a diff for a patch against its target branch.
    ///
    /// Resolves the patch to get its current head commit and base,
    /// then diffs the base against the head.
    pub async fn diff_patch(&self, repo_id: &RepoId, patch_id: &blake3::Hash) -> ForgeResult<crate::git::DiffResult> {
        let patch = self.cobs.resolve_patch(repo_id, patch_id).await?;

        let base_hash = blake3::Hash::from_bytes(patch.base);
        let head_hash = blake3::Hash::from_bytes(patch.head);

        self.diff_commits(&base_hash, &head_hash, false).await
    }

    // ========================================================================
    // Merge Operations
    // ========================================================================

    /// Dry-run merge check. Returns mergeability status without side effects.
    ///
    /// Resolves the patch, checks branch protection, attempts a tree merge,
    /// and reports the result. No objects are written, no refs updated.
    pub async fn check_merge(&self, repo_id: &RepoId, patch_id: &blake3::Hash) -> ForgeResult<MergeCheckResult> {
        // Resolve patch state
        let patch = self.cobs.resolve_patch(repo_id, patch_id).await?;

        if !patch.state.is_open() {
            return Ok(MergeCheckResult {
                mergeable: false,
                available_strategies: vec![],
                conflicts: vec![],
                protection_satisfied: false,
                protection_reason: Some(format!("patch is {:?}, must be Open", patch.state)),
            });
        }

        let patch_head = blake3::Hash::from_bytes(patch.head);

        // Check branch protection
        let identity = self.get_repo(repo_id).await?;
        let target_ref = format!("heads/{}", identity.default_branch);
        let merge_checker = crate::protection::MergeChecker::new(self.kv.clone());
        let protection_result =
            merge_checker.check_merge_allowed(repo_id, &target_ref, &patch.head, &patch.approvals).await;

        let (protection_satisfied, protection_reason) = match protection_result {
            Ok(()) => (true, None),
            Err(ForgeError::MergeBlocked { reason }) => (false, Some(reason)),
            Err(e) => return Err(e),
        };

        // Check divergence and conflicts
        let target_commit = match self.refs.get(repo_id, &target_ref).await? {
            Some(c) => c,
            None => {
                return Ok(MergeCheckResult {
                    mergeable: false,
                    available_strategies: vec![],
                    conflicts: vec![],
                    protection_satisfied,
                    protection_reason: Some("target ref not found".to_string()),
                });
            }
        };

        let patch_base = blake3::Hash::from_bytes(patch.base);
        let is_fast_forward = target_commit == patch_base;

        // Determine available strategies
        let mut available_strategies = vec!["merge".to_string(), "squash".to_string()];
        if is_fast_forward {
            available_strategies.push("fast-forward".to_string());
        }

        // Check for conflicts (only if diverged)
        let conflicts = if !is_fast_forward {
            let target_tree = self.git.get_commit(&target_commit).await?.tree();
            let target_tree = self.git.get_tree(&target_tree).await?;

            let base_tree = self.git.get_commit(&patch_base).await?.tree();
            let base_tree = self.git.get_tree(&base_tree).await?;

            let head_tree = self.git.get_commit(&patch_head).await?.tree();
            let head_tree = self.git.get_tree(&head_tree).await?;

            let merge_result = crate::git::merge_trees(&self.git, &base_tree, &target_tree, &head_tree).await?;
            merge_result.conflicts.iter().map(|c| c.path.clone()).collect::<Vec<_>>()
        } else {
            vec![]
        };

        let mergeable = protection_satisfied && conflicts.is_empty();

        Ok(MergeCheckResult {
            mergeable,
            available_strategies,
            conflicts,
            protection_satisfied,
            protection_reason,
        })
    }

    /// Merge a patch into its target branch.
    ///
    /// Performs the full merge workflow:
    /// 1. Resolve the patch to get head commit and target ref
    /// 2. Check branch protection (approvals, CI status)
    /// 3. Three-way merge the trees (or fast-forward if possible)
    /// 4. Create merge commit and advance target ref via CAS
    /// 5. Transition patch COB to `Merged` state
    /// 6. Broadcast gossip announcement
    ///
    /// # Arguments
    ///
    /// * `repo_id` - Repository containing the patch
    /// * `patch_id` - Patch COB ID
    /// * `strategy` - Merge strategy (merge commit, fast-forward-only, squash)
    /// * `custom_message` - Optional override for the merge commit message
    ///
    /// # Errors
    ///
    /// * `ForgeError::MergeBlocked` - Branch protection not satisfied
    /// * `ForgeError::MergeConflicts` - Tree merge produced conflicts
    /// * `ForgeError::MergeCasExhausted` - CAS retries exhausted
    /// * `ForgeError::PatchNotMergeable` - Patch is not open
    /// * `ForgeError::FastForwardNotPossible` - FF-only but target diverged
    pub async fn merge_patch(
        &self,
        repo_id: &RepoId,
        patch_id: &blake3::Hash,
        strategy: crate::git::GitMergeStrategy,
        custom_message: Option<String>,
    ) -> ForgeResult<blake3::Hash> {
        use crate::constants::MAX_MERGE_CAS_RETRIES;
        use crate::git::GitMergeStrategy;

        // 1. Resolve patch state
        let patch = self.cobs.resolve_patch(repo_id, patch_id).await?;

        if !patch.state.is_open() {
            return Err(ForgeError::PatchNotMergeable {
                reason: format!("patch is {:?}, must be Open", patch.state),
            });
        }

        let patch_head = blake3::Hash::from_bytes(patch.head);

        // Determine target ref (default branch)
        let identity = self.get_repo(repo_id).await?;
        let target_ref = format!("heads/{}", identity.default_branch);

        // 2. Check branch protection
        let merge_checker = crate::protection::MergeChecker::new(self.kv.clone());
        merge_checker.check_merge_allowed(repo_id, &target_ref, &patch.head, &patch.approvals).await?;

        // 3-4. CAS retry loop
        let mut attempts = 0u32;
        loop {
            attempts = attempts.saturating_add(1);
            if attempts > MAX_MERGE_CAS_RETRIES {
                return Err(ForgeError::MergeCasExhausted {
                    attempts: MAX_MERGE_CAS_RETRIES,
                });
            }

            // Get current target ref
            let target_commit = self.refs.get(repo_id, &target_ref).await?.ok_or_else(|| ForgeError::RefNotFound {
                ref_name: target_ref.clone(),
            })?;

            let patch_base = blake3::Hash::from_bytes(patch.base);
            let is_fast_forward = target_commit == patch_base;

            // Fast-forward-only: reject diverged branches immediately
            if strategy == GitMergeStrategy::FastForwardOnly && !is_fast_forward {
                return Err(ForgeError::FastForwardNotPossible);
            }

            // Fast-forward path (for MergeCommit and FastForwardOnly when target == base)
            if is_fast_forward && strategy != GitMergeStrategy::Squash {
                match self.refs.compare_and_set(repo_id, &target_ref, Some(target_commit), patch_head).await {
                    Ok(()) => {
                        self.cobs.merge_patch(repo_id, patch_id, patch_head).await?;
                        return Ok(patch_head);
                    }
                    Err(ForgeError::RefConflict { .. }) => {
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }

            // Need a tree merge (diverged branches or squash)
            let (merged_tree_hash, target_for_parent) =
                self.compute_merged_tree(repo_id, &target_commit, &patch_base, &patch_head, is_fast_forward).await?;

            // Build the commit
            let message = custom_message.clone().unwrap_or_else(|| match strategy {
                GitMergeStrategy::Squash => format!("Squash merge patch '{}'", patch.title),
                _ => format!("Merge patch '{}'", patch.title),
            });

            let result_commit = match strategy {
                GitMergeStrategy::Squash => {
                    // Single-parent commit on the target branch
                    self.git.commit(merged_tree_hash, vec![target_commit], &message).await?
                }
                _ => {
                    // Two-parent merge commit
                    self.git.commit(merged_tree_hash, vec![target_for_parent, patch_head], &message).await?
                }
            };

            // CAS the ref
            match self.refs.compare_and_set(repo_id, &target_ref, Some(target_commit), result_commit).await {
                Ok(()) => {
                    self.cobs.merge_patch(repo_id, patch_id, result_commit).await?;
                    return Ok(result_commit);
                }
                Err(ForgeError::RefConflict { .. }) => {
                    tracing::debug!(attempt = attempts, "merge CAS failed, retrying");
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Compute the merged tree for a patch merge.
    ///
    /// When `is_fast_forward` is true (squash path), uses the patch head's tree directly.
    /// Otherwise performs a full three-way merge.
    async fn compute_merged_tree(
        &self,
        _repo_id: &RepoId,
        target_commit: &blake3::Hash,
        patch_base: &blake3::Hash,
        patch_head: &blake3::Hash,
        is_fast_forward: bool,
    ) -> ForgeResult<(blake3::Hash, blake3::Hash)> {
        if is_fast_forward {
            // Squash on a non-diverged branch: use the patch head's tree
            let head_tree = self.git.get_commit(patch_head).await?.tree();
            return Ok((head_tree, *target_commit));
        }

        // Full three-way merge
        let target_tree = self.git.get_commit(target_commit).await?.tree();
        let target_tree = self.git.get_tree(&target_tree).await?;

        let base_tree = self.git.get_commit(patch_base).await?.tree();
        let base_tree = self.git.get_tree(&base_tree).await?;

        let head_tree = self.git.get_commit(patch_head).await?.tree();
        let head_tree = self.git.get_tree(&head_tree).await?;

        let merge_result = crate::git::merge_trees(&self.git, &base_tree, &target_tree, &head_tree).await?;

        if !merge_result.is_clean() {
            return Err(ForgeError::MergeConflicts {
                count: merge_result.conflicts.len() as u32,
                conflicts: merge_result.conflicts,
            });
        }

        let merged_tree = merge_result.tree.ok_or_else(|| ForgeError::PatchNotMergeable {
            reason: "merge produced no tree despite no conflicts".to_string(),
        })?;

        let merged_tree_hash = self.git.create_tree(&merged_tree.entries).await?;
        Ok((merged_tree_hash, *target_commit))
    }
}

#[cfg(test)]
mod tests {
    use aspen_blob::InMemoryBlobStore;
    use aspen_testing::DeterministicKeyValueStore;

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

    // ========================================================================
    // Federation KV integration tests
    // ========================================================================

    #[tokio::test]
    async fn test_count_federated_resources_empty() {
        let node = create_test_node().await;
        let count = node.count_federated_resources().await.expect("should count");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_federated_resources_with_public_repos() {
        let node = create_test_node().await;

        // Store federation settings for two public repos
        let settings = aspen_cluster::federation::FederationSettings::public()
            .with_resource_type(crate::federation::FORGE_RESOURCE_TYPE);
        let json = serde_json::to_string(&settings).unwrap();

        let fed_id_1 = format!("{}repo1", crate::constants::KV_PREFIX_FEDERATION_SETTINGS);
        let fed_id_2 = format!("{}repo2", crate::constants::KV_PREFIX_FEDERATION_SETTINGS);

        node.kv()
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::SetMulti {
                    pairs: vec![(fed_id_1, json.clone()), (fed_id_2, json)],
                },
            })
            .await
            .unwrap();

        let count = node.count_federated_resources().await.expect("should count");
        assert_eq!(count, 2);
    }

    #[tokio::test]
    async fn test_count_federated_resources_excludes_disabled() {
        let node = create_test_node().await;

        let public_json = serde_json::to_string(
            &aspen_cluster::federation::FederationSettings::public()
                .with_resource_type(crate::federation::FORGE_RESOURCE_TYPE),
        )
        .unwrap();
        let disabled_json = serde_json::to_string(
            &aspen_cluster::federation::FederationSettings::disabled()
                .with_resource_type(crate::federation::FORGE_RESOURCE_TYPE),
        )
        .unwrap();

        let key_public = format!("{}repo_pub", crate::constants::KV_PREFIX_FEDERATION_SETTINGS);
        let key_disabled = format!("{}repo_off", crate::constants::KV_PREFIX_FEDERATION_SETTINGS);

        node.kv()
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::SetMulti {
                    pairs: vec![(key_public, public_json), (key_disabled, disabled_json)],
                },
            })
            .await
            .unwrap();

        let count = node.count_federated_resources().await.expect("should count");
        assert_eq!(count, 1, "disabled repos should not be counted");
    }

    #[tokio::test]
    async fn test_count_federated_resources_includes_allowlist() {
        let node = create_test_node().await;

        let peer_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let settings = aspen_cluster::federation::FederationSettings::allowlist(vec![peer_key])
            .with_resource_type(crate::federation::FORGE_RESOURCE_TYPE);
        let json = serde_json::to_string(&settings).unwrap();

        let key = format!("{}repo_al", crate::constants::KV_PREFIX_FEDERATION_SETTINGS);
        node.kv()
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::Set { key, value: json },
            })
            .await
            .unwrap();

        let count = node.count_federated_resources().await.expect("should count");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_list_federated_resources_empty() {
        let node = create_test_node().await;
        let list = node.list_federated_resources(None, 100).await.expect("should list");
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_list_federated_resources_filters_disabled() {
        let node = create_test_node().await;

        // Create a valid FederatedId for the key suffix
        let origin_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let local_id: [u8; 32] = blake3::hash(b"test-repo").into();
        let fed_id = aspen_cluster::federation::FederatedId::new(origin_key, local_id);

        let public_settings = aspen_cluster::federation::FederationSettings::public()
            .with_resource_type(crate::federation::FORGE_RESOURCE_TYPE);
        let disabled_settings = aspen_cluster::federation::FederationSettings::disabled()
            .with_resource_type(crate::federation::FORGE_RESOURCE_TYPE);

        let key_pub = format!("{}{}", crate::constants::KV_PREFIX_FEDERATION_SETTINGS, fed_id);

        // Use a different local_id for the disabled one
        let local_id_2: [u8; 32] = blake3::hash(b"disabled-repo").into();
        let fed_id_2 = aspen_cluster::federation::FederatedId::new(origin_key, local_id_2);
        let key_dis = format!("{}{}", crate::constants::KV_PREFIX_FEDERATION_SETTINGS, fed_id_2);

        node.kv()
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::SetMulti {
                    pairs: vec![
                        (key_pub, serde_json::to_string(&public_settings).unwrap()),
                        (key_dis, serde_json::to_string(&disabled_settings).unwrap()),
                    ],
                },
            })
            .await
            .unwrap();

        let list = node.list_federated_resources(None, 100).await.expect("should list");
        assert_eq!(list.len(), 1, "disabled repos should be filtered out");
        assert_eq!(list[0].1.mode, aspen_cluster::federation::FederationMode::Public);
    }

    #[tokio::test]
    async fn test_list_federated_resources_limit_capped() {
        let node = create_test_node().await;

        let origin_key = iroh::SecretKey::generate(&mut rand::rng()).public();
        let public_json = serde_json::to_string(
            &aspen_cluster::federation::FederationSettings::public()
                .with_resource_type(crate::federation::FORGE_RESOURCE_TYPE),
        )
        .unwrap();

        // Create 5 federated repos
        let mut pairs = Vec::new();
        for i in 0..5u8 {
            let local_id: [u8; 32] = blake3::hash(&[i]).into();
            let fed_id = aspen_cluster::federation::FederatedId::new(origin_key, local_id);
            let key = format!("{}{}", crate::constants::KV_PREFIX_FEDERATION_SETTINGS, fed_id);
            pairs.push((key, public_json.clone()));
        }

        node.kv()
            .write(aspen_core::WriteRequest {
                command: aspen_core::WriteCommand::SetMulti { pairs },
            })
            .await
            .unwrap();

        // Request with limit 3
        let list = node.list_federated_resources(None, 3).await.expect("should list");
        assert!(list.len() <= 3, "should respect limit");
    }

    #[tokio::test]
    async fn test_has_gossip_default_false() {
        let node = create_test_node().await;
        assert!(!node.has_gossip(), "gossip should be disabled by default");
    }

    // ========================================================================
    // read_file_at_commit tests
    // ========================================================================

    #[tokio::test]
    async fn test_read_file_at_commit_exists() {
        let node = create_test_node().await;

        // Store blob "hello world"
        let content = b"hello world";
        let blob_hash = node.git.store_blob(content.to_vec()).await.expect("should store blob");

        // Create tree with entry
        let tree_hash = node
            .git
            .create_tree(&[crate::git::TreeEntry::file("test.txt", blob_hash)])
            .await
            .expect("should create tree");

        // Create commit
        let commit_hash = node.git.commit(tree_hash, vec![], "Test commit").await.expect("should create commit");

        // Create a fake repo_id for the test
        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        // Read file from commit
        let result = node.read_file_at_commit(&repo_id, &commit_hash, "test.txt").await.expect("should read file");

        assert_eq!(result, Some(content.to_vec()));
    }

    #[tokio::test]
    async fn test_read_file_at_commit_missing() {
        let node = create_test_node().await;

        // Store blob
        let content = b"hello world";
        let blob_hash = node.git.store_blob(content.to_vec()).await.expect("should store blob");

        // Create tree with entry
        let tree_hash = node
            .git
            .create_tree(&[crate::git::TreeEntry::file("test.txt", blob_hash)])
            .await
            .expect("should create tree");

        // Create commit
        let commit_hash = node.git.commit(tree_hash, vec![], "Test commit").await.expect("should create commit");

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        // Request a non-existent file
        let result = node
            .read_file_at_commit(&repo_id, &commit_hash, "nonexistent.txt")
            .await
            .expect("should return Ok(None)");

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_read_file_at_commit_nested_path() {
        let node = create_test_node().await;

        // Store blob for the deeply nested file
        let content = b"nested content";
        let blob_hash = node.git.store_blob(content.to_vec()).await.expect("should store blob");

        // Create innermost tree (subdir/)
        let subdir_tree_hash = node
            .git
            .create_tree(&[crate::git::TreeEntry::file("file.txt", blob_hash)])
            .await
            .expect("should create subdir tree");

        // Create middle tree (dir/)
        let dir_tree_hash = node
            .git
            .create_tree(&[crate::git::TreeEntry::directory("subdir", subdir_tree_hash)])
            .await
            .expect("should create dir tree");

        // Create root tree
        let root_tree_hash = node
            .git
            .create_tree(&[crate::git::TreeEntry::directory("dir", dir_tree_hash)])
            .await
            .expect("should create root tree");

        // Create commit
        let commit_hash = node.git.commit(root_tree_hash, vec![], "Nested commit").await.expect("should create commit");

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        // Read deeply nested file
        let result = node
            .read_file_at_commit(&repo_id, &commit_hash, "dir/subdir/file.txt")
            .await
            .expect("should read nested file");

        assert_eq!(result, Some(content.to_vec()));
    }

    #[tokio::test]
    async fn test_read_file_at_commit_empty_tree() {
        let node = create_test_node().await;

        // Create empty tree
        let tree_hash = node.git.create_tree(&[]).await.expect("should create empty tree");

        // Create commit with empty tree
        let commit_hash = node.git.commit(tree_hash, vec![], "Empty commit").await.expect("should create commit");

        let repo_id = RepoId::from_hash(blake3::hash(b"test-repo"));

        // Try to read any file from empty tree
        let result = node
            .read_file_at_commit(&repo_id, &commit_hash, "any-file.txt")
            .await
            .expect("should return Ok(None)");

        assert_eq!(result, None);
    }

    // ========================================================================
    // merge_patch integration tests
    // ========================================================================

    /// Helper: create a repo, init with a file, create a patch with a modification.
    async fn setup_mergeable_patch(
        node: &ForgeNode<InMemoryBlobStore, DeterministicKeyValueStore>,
    ) -> (RepoIdentity, blake3::Hash) {
        let identity = node.create_repo("merge-test", vec![node.public_key()], 1).await.expect("create repo");
        let repo_id = identity.repo_id();

        // Initial commit: one file
        let h1 = node.git.store_blob(b"initial".to_vec()).await.unwrap();
        let tree1 = node.git.create_tree(&[crate::git::TreeEntry::file("readme.md", h1)]).await.unwrap();
        let c1 = node.git.commit(tree1, vec![], "Initial commit").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c1).await.unwrap();

        // Patch: modify the file
        let h2 = node.git.store_blob(b"modified".to_vec()).await.unwrap();
        let tree2 = node.git.create_tree(&[crate::git::TreeEntry::file("readme.md", h2)]).await.unwrap();
        let c2 = node.git.commit(tree2, vec![c1], "Patch commit").await.unwrap();

        let patch_id = node.cobs.create_patch(&repo_id, "My patch", "Description", c1, c2).await.unwrap();

        (identity, patch_id)
    }

    #[tokio::test]
    async fn test_merge_patch_fast_forward() {
        let node = create_test_node().await;
        let (identity, patch_id) = setup_mergeable_patch(&node).await;
        let repo_id = identity.repo_id();

        // Merge — should fast-forward since main hasn't moved
        let result = node.merge_patch(&repo_id, &patch_id, Default::default(), None).await;
        assert!(result.is_ok(), "merge should succeed: {:?}", result.err());

        let merge_hash = result.unwrap();

        // Check that main now points to the patch head (fast-forward, no merge commit)
        let head = node.refs.get(&repo_id, "heads/main").await.unwrap().unwrap();
        assert_eq!(head, merge_hash);

        // Check patch is now Merged
        let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.unwrap();
        assert!(patch.state.is_merged());
    }

    #[tokio::test]
    async fn test_merge_patch_three_way() {
        let node = create_test_node().await;
        let identity = node.create_repo("3way-test", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit: two files
        let ha = node.git.store_blob(b"file_a".to_vec()).await.unwrap();
        let hb = node.git.store_blob(b"file_b".to_vec()).await.unwrap();
        let tree0 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Push a change to main (modifies a.rs) — this makes it non-fast-forward
        let ha2 = node.git.store_blob(b"a_main".to_vec()).await.unwrap();
        let tree_main = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha2),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c_main = node.git.commit(tree_main, vec![c0], "main change").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c_main).await.unwrap();

        // Create patch based on c0 (modifies b.rs)
        let hb2 = node.git.store_blob(b"b_patch".to_vec()).await.unwrap();
        let tree_patch = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb2),
            ])
            .await
            .unwrap();
        let c_patch = node.git.commit(tree_patch, vec![c0], "patch change").await.unwrap();

        let patch_id = node.cobs.create_patch(&repo_id, "Modify b.rs", "Change b", c0, c_patch).await.unwrap();

        // Merge — should do full three-way merge
        let merge_hash = node
            .merge_patch(&repo_id, &patch_id, Default::default(), Some("Custom merge msg".to_string()))
            .await
            .expect("merge should succeed");

        // Verify the merge commit
        let merge_commit = node.git.get_commit(&merge_hash).await.unwrap();
        assert_eq!(merge_commit.parents.len(), 2, "merge commit should have two parents");
        assert_eq!(merge_commit.message, "Custom merge msg");

        // Verify the merged tree has both changes
        let merged_tree = node.git.get_tree(&merge_commit.tree()).await.unwrap();
        let a_entry = merged_tree.entries.iter().find(|e| e.name == "a.rs").unwrap();
        let b_entry = merged_tree.entries.iter().find(|e| e.name == "b.rs").unwrap();
        assert_eq!(a_entry.hash, *ha2.as_bytes(), "a.rs should be main's version");
        assert_eq!(b_entry.hash, *hb2.as_bytes(), "b.rs should be patch's version");

        // Verify main points to merge commit
        let head = node.refs.get(&repo_id, "heads/main").await.unwrap().unwrap();
        assert_eq!(head, merge_hash);
    }

    #[tokio::test]
    async fn test_merge_patch_not_open() {
        let node = create_test_node().await;
        let (identity, patch_id) = setup_mergeable_patch(&node).await;
        let repo_id = identity.repo_id();

        // Close the patch first
        node.cobs.close_patch(&repo_id, &patch_id, Some("not needed".to_string())).await.unwrap();

        // Try to merge a closed patch
        let result = node.merge_patch(&repo_id, &patch_id, Default::default(), None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ForgeError::PatchNotMergeable { .. }));
    }

    #[tokio::test]
    async fn test_merge_patch_conflict() {
        let node = create_test_node().await;
        let identity = node.create_repo("conflict-test", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit
        let h0 = node.git.store_blob(b"base".to_vec()).await.unwrap();
        let tree0 = node.git.create_tree(&[crate::git::TreeEntry::file("config.toml", h0)]).await.unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Main changes config.toml
        let h_main = node.git.store_blob(b"main_config".to_vec()).await.unwrap();
        let tree_main = node.git.create_tree(&[crate::git::TreeEntry::file("config.toml", h_main)]).await.unwrap();
        let c_main = node.git.commit(tree_main, vec![c0], "main cfg").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c_main).await.unwrap();

        // Patch also changes config.toml (differently)
        let h_patch = node.git.store_blob(b"patch_config".to_vec()).await.unwrap();
        let tree_patch = node.git.create_tree(&[crate::git::TreeEntry::file("config.toml", h_patch)]).await.unwrap();
        let c_patch = node.git.commit(tree_patch, vec![c0], "patch cfg").await.unwrap();

        let patch_id = node.cobs.create_patch(&repo_id, "Conflicting", "Desc", c0, c_patch).await.unwrap();

        // Merge should fail with conflicts
        let result = node.merge_patch(&repo_id, &patch_id, Default::default(), None).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ForgeError::MergeConflicts { count, conflicts } => {
                assert_eq!(count, 1);
                assert_eq!(conflicts[0].path, "config.toml");
            }
            e => panic!("expected MergeConflicts, got: {:?}", e),
        }
    }

    // ========================================================================
    // check_merge tests
    // ========================================================================

    #[tokio::test]
    async fn test_check_merge_mergeable() {
        let node = create_test_node().await;
        let (identity, patch_id) = setup_mergeable_patch(&node).await;
        let repo_id = identity.repo_id();

        let result = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(result.mergeable);
        assert!(result.conflicts.is_empty());
        assert!(result.protection_satisfied);
        assert!(result.available_strategies.contains(&"merge".to_string()));
        assert!(result.available_strategies.contains(&"squash".to_string()));
        assert!(result.available_strategies.contains(&"fast-forward".to_string()));
    }

    #[tokio::test]
    async fn test_check_merge_closed_patch() {
        let node = create_test_node().await;
        let (identity, patch_id) = setup_mergeable_patch(&node).await;
        let repo_id = identity.repo_id();

        node.cobs.close_patch(&repo_id, &patch_id, None).await.unwrap();

        let result = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(!result.mergeable);
        assert!(result.available_strategies.is_empty());
    }

    #[tokio::test]
    async fn test_check_merge_with_conflicts() {
        let node = create_test_node().await;
        let identity = node.create_repo("check-conflict", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit
        let h0 = node.git.store_blob(b"base".to_vec()).await.unwrap();
        let tree0 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h0)]).await.unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Main modifies f.txt
        let h1 = node.git.store_blob(b"main_v".to_vec()).await.unwrap();
        let tree1 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h1)]).await.unwrap();
        let c1 = node.git.commit(tree1, vec![c0], "main").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c1).await.unwrap();

        // Patch also modifies f.txt differently
        let h2 = node.git.store_blob(b"patch_v".to_vec()).await.unwrap();
        let tree2 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h2)]).await.unwrap();
        let c2 = node.git.commit(tree2, vec![c0], "patch").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "Conflict", "Desc", c0, c2).await.unwrap();

        let result = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(!result.mergeable);
        assert_eq!(result.conflicts, vec!["f.txt"]);
        assert!(result.protection_satisfied); // no protection rules set
        // fast-forward not available (diverged)
        assert!(!result.available_strategies.contains(&"fast-forward".to_string()));
    }

    #[tokio::test]
    async fn test_check_merge_diverged_no_ff() {
        let node = create_test_node().await;
        let identity = node.create_repo("check-div", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit with two files
        let ha = node.git.store_blob(b"a".to_vec()).await.unwrap();
        let hb = node.git.store_blob(b"b".to_vec()).await.unwrap();
        let tree0 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Main modifies a.rs
        let ha2 = node.git.store_blob(b"a2".to_vec()).await.unwrap();
        let tree1 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha2),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c1 = node.git.commit(tree1, vec![c0], "main").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c1).await.unwrap();

        // Patch modifies b.rs (clean merge possible, but no ff)
        let hb2 = node.git.store_blob(b"b2".to_vec()).await.unwrap();
        let tree2 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb2),
            ])
            .await
            .unwrap();
        let c2 = node.git.commit(tree2, vec![c0], "patch").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "Clean div", "Desc", c0, c2).await.unwrap();

        let result = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(result.mergeable);
        assert!(result.conflicts.is_empty());
        assert!(!result.available_strategies.contains(&"fast-forward".to_string()));
        assert!(result.available_strategies.contains(&"merge".to_string()));
        assert!(result.available_strategies.contains(&"squash".to_string()));
    }

    #[tokio::test]
    async fn test_diff_commits_end_to_end() {
        let node = create_test_node().await;

        // Create two commits with different files
        let h1 = node.git.store_blob(b"v1".to_vec()).await.unwrap();
        let h2 = node.git.store_blob(b"v2".to_vec()).await.unwrap();
        let h_new = node.git.store_blob(b"new".to_vec()).await.unwrap();

        let tree1 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h1)]).await.unwrap();
        let tree2 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("f.txt", h2),
                crate::git::TreeEntry::file("g.txt", h_new),
            ])
            .await
            .unwrap();

        let c1 = node.git.commit(tree1, vec![], "c1").await.unwrap();
        let c2 = node.git.commit(tree2, vec![c1], "c2").await.unwrap();

        let result = node.diff_commits(&c1, &c2, false).await.unwrap();
        assert_eq!(result.entries.len(), 2);

        let f_entry = result.entries.iter().find(|e| e.path == "f.txt").unwrap();
        assert_eq!(f_entry.kind, crate::git::DiffKind::Modified);

        let g_entry = result.entries.iter().find(|e| e.path == "g.txt").unwrap();
        assert_eq!(g_entry.kind, crate::git::DiffKind::Added);
    }

    // ========================================================================
    // merge strategy tests
    // ========================================================================

    #[tokio::test]
    async fn test_merge_patch_squash_no_divergence() {
        let node = create_test_node().await;
        let (identity, patch_id) = setup_mergeable_patch(&node).await;
        let repo_id = identity.repo_id();

        // Squash merge — target hasn't diverged, but squash always creates a new commit
        let result = node.merge_patch(&repo_id, &patch_id, crate::git::GitMergeStrategy::Squash, None).await;
        assert!(result.is_ok(), "squash merge should succeed: {:?}", result.err());

        let squash_hash = result.unwrap();

        // Squash commit should have a single parent (the old main head = patch base)
        let squash_commit = node.git.get_commit(&squash_hash).await.unwrap();
        assert_eq!(squash_commit.parents.len(), 1, "squash commit has single parent");

        // Default message should mention "Squash merge"
        assert!(
            squash_commit.message.contains("Squash merge"),
            "message should contain 'Squash merge': {}",
            squash_commit.message
        );

        // main should point to the squash commit
        let head = node.refs.get(&repo_id, "heads/main").await.unwrap().unwrap();
        assert_eq!(head, squash_hash);

        // Patch should be merged
        let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.unwrap();
        assert!(patch.state.is_merged());
    }

    #[tokio::test]
    async fn test_merge_patch_squash_diverged() {
        let node = create_test_node().await;
        let identity = node.create_repo("squash-div", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit
        let ha = node.git.store_blob(b"a_base".to_vec()).await.unwrap();
        let hb = node.git.store_blob(b"b_base".to_vec()).await.unwrap();
        let tree0 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Main diverges (modifies a.rs)
        let ha2 = node.git.store_blob(b"a_main".to_vec()).await.unwrap();
        let tree_main = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha2),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c_main = node.git.commit(tree_main, vec![c0], "main").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c_main).await.unwrap();

        // Patch modifies b.rs (non-overlapping)
        let hb2 = node.git.store_blob(b"b_patch".to_vec()).await.unwrap();
        let tree_patch = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb2),
            ])
            .await
            .unwrap();
        let c_patch = node.git.commit(tree_patch, vec![c0], "patch").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "Squash diverged", "Desc", c0, c_patch).await.unwrap();

        let squash_hash = node
            .merge_patch(&repo_id, &patch_id, crate::git::GitMergeStrategy::Squash, None)
            .await
            .expect("squash merge should succeed");

        let squash_commit = node.git.get_commit(&squash_hash).await.unwrap();
        assert_eq!(squash_commit.parents.len(), 1, "squash has single parent");
        // Parent should be c_main (the target head)
        assert_eq!(blake3::Hash::from_bytes(squash_commit.parents[0]), c_main, "squash parent should be target head");

        // Merged tree should have both changes
        let merged_tree = node.git.get_tree(&squash_commit.tree()).await.unwrap();
        let a_entry = merged_tree.entries.iter().find(|e| e.name == "a.rs").unwrap();
        let b_entry = merged_tree.entries.iter().find(|e| e.name == "b.rs").unwrap();
        assert_eq!(a_entry.hash, *ha2.as_bytes(), "a.rs from main");
        assert_eq!(b_entry.hash, *hb2.as_bytes(), "b.rs from patch");
    }

    #[tokio::test]
    async fn test_merge_patch_ff_only_succeeds() {
        let node = create_test_node().await;
        let (identity, patch_id) = setup_mergeable_patch(&node).await;
        let repo_id = identity.repo_id();

        // FF-only should succeed when target hasn't diverged
        let result = node.merge_patch(&repo_id, &patch_id, crate::git::GitMergeStrategy::FastForwardOnly, None).await;
        assert!(result.is_ok(), "ff-only should succeed: {:?}", result.err());

        // Should have advanced the ref directly (no merge commit)
        let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.unwrap();
        assert!(patch.state.is_merged());
    }

    #[tokio::test]
    async fn test_merge_patch_ff_only_rejects_diverged() {
        let node = create_test_node().await;
        let identity = node.create_repo("ff-div", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit
        let h0 = node.git.store_blob(b"base".to_vec()).await.unwrap();
        let tree0 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h0)]).await.unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Main diverges
        let h1 = node.git.store_blob(b"main_v".to_vec()).await.unwrap();
        let tree1 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h1)]).await.unwrap();
        let c1 = node.git.commit(tree1, vec![c0], "main").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c1).await.unwrap();

        // Patch from c0
        let h2 = node.git.store_blob(b"patch_v".to_vec()).await.unwrap();
        let tree2 = node.git.create_tree(&[crate::git::TreeEntry::file("g.txt", h2)]).await.unwrap();
        let c2 = node.git.commit(tree2, vec![c0], "patch").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "FF fail", "Desc", c0, c2).await.unwrap();

        let result = node.merge_patch(&repo_id, &patch_id, crate::git::GitMergeStrategy::FastForwardOnly, None).await;
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ForgeError::FastForwardNotPossible),
            "should get FastForwardNotPossible"
        );

        // Patch should still be open
        let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.unwrap();
        assert!(patch.state.is_open());
    }

    #[tokio::test]
    async fn test_merge_patch_squash_custom_message() {
        let node = create_test_node().await;
        let (identity, patch_id) = setup_mergeable_patch(&node).await;
        let repo_id = identity.repo_id();

        let squash_hash = node
            .merge_patch(&repo_id, &patch_id, crate::git::GitMergeStrategy::Squash, Some("feat: add auth".to_string()))
            .await
            .expect("squash merge should succeed");

        let commit = node.git.get_commit(&squash_hash).await.unwrap();
        assert_eq!(commit.message, "feat: add auth");
    }

    // ========================================================================
    // End-to-end integration tests
    // ========================================================================

    #[tokio::test]
    async fn test_e2e_create_patch_approve_merge() {
        let node = create_test_node().await;
        let identity = node.create_repo("e2e-test", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit
        let h0 = node.git.store_blob(b"initial".to_vec()).await.unwrap();
        let tree0 = node.git.create_tree(&[crate::git::TreeEntry::file("readme.md", h0)]).await.unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Patch: add a new file
        let h1 = node.git.store_blob(b"new file".to_vec()).await.unwrap();
        let tree1 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("readme.md", h0),
                crate::git::TreeEntry::file("new.txt", h1),
            ])
            .await
            .unwrap();
        let c1 = node.git.commit(tree1, vec![c0], "add new.txt").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "Add file", "Adding new.txt", c0, c1).await.unwrap();

        // Verify patch is open
        let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.unwrap();
        assert!(patch.state.is_open());

        // Approve
        node.cobs.approve_patch(&repo_id, &patch_id, c1, None).await.unwrap();

        // Check merge
        let check = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(check.mergeable);

        // Merge (default strategy)
        let merge_hash =
            node.merge_patch(&repo_id, &patch_id, Default::default(), None).await.expect("merge should succeed");

        // Verify ref advanced
        let head = node.refs.get(&repo_id, "heads/main").await.unwrap().unwrap();
        assert_eq!(head, merge_hash);

        // Verify patch is merged
        let patch = node.cobs.resolve_patch(&repo_id, &patch_id).await.unwrap();
        assert!(patch.state.is_merged());
    }

    #[tokio::test]
    async fn test_e2e_squash_merge_single_parent() {
        let node = create_test_node().await;
        let identity = node.create_repo("e2e-squash", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Initial commit with two files
        let ha = node.git.store_blob(b"a".to_vec()).await.unwrap();
        let hb = node.git.store_blob(b"b".to_vec()).await.unwrap();
        let tree0 = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        // Main modifies a.rs
        let ha2 = node.git.store_blob(b"a2".to_vec()).await.unwrap();
        let tree_main = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha2),
                crate::git::TreeEntry::file("b.rs", hb),
            ])
            .await
            .unwrap();
        let c_main = node.git.commit(tree_main, vec![c0], "main").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c_main).await.unwrap();

        // Patch modifies b.rs
        let hb2 = node.git.store_blob(b"b2".to_vec()).await.unwrap();
        let tree_patch = node
            .git
            .create_tree(&[
                crate::git::TreeEntry::file("a.rs", ha),
                crate::git::TreeEntry::file("b.rs", hb2),
            ])
            .await
            .unwrap();
        let c_patch = node.git.commit(tree_patch, vec![c0], "patch").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "Squash test", "Test", c0, c_patch).await.unwrap();

        // Squash merge
        let squash = node
            .merge_patch(&repo_id, &patch_id, crate::git::GitMergeStrategy::Squash, None)
            .await
            .expect("squash should succeed");

        // Verify single parent
        let commit = node.git.get_commit(&squash).await.unwrap();
        assert_eq!(commit.parents.len(), 1);
        assert_eq!(blake3::Hash::from_bytes(commit.parents[0]), c_main);

        // Verify merged tree has both changes
        let tree = node.git.get_tree(&commit.tree()).await.unwrap();
        let a_entry = tree.entries.iter().find(|e| e.name == "a.rs").unwrap();
        let b_entry = tree.entries.iter().find(|e| e.name == "b.rs").unwrap();
        assert_eq!(a_entry.hash, *ha2.as_bytes());
        assert_eq!(b_entry.hash, *hb2.as_bytes());
    }

    #[tokio::test]
    async fn test_e2e_ff_only_rejects_diverged() {
        let node = create_test_node().await;
        let identity = node.create_repo("e2e-ffonly", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Setup: diverged branches
        let h0 = node.git.store_blob(b"base".to_vec()).await.unwrap();
        let tree0 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h0)]).await.unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        let h1 = node.git.store_blob(b"main".to_vec()).await.unwrap();
        let tree1 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h1)]).await.unwrap();
        let c1 = node.git.commit(tree1, vec![c0], "main").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c1).await.unwrap();

        let h2 = node.git.store_blob(b"patch".to_vec()).await.unwrap();
        let tree2 = node.git.create_tree(&[crate::git::TreeEntry::file("g.txt", h2)]).await.unwrap();
        let c2 = node.git.commit(tree2, vec![c0], "patch").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "FF test", "Test", c0, c2).await.unwrap();

        // FF-only should fail
        let result = node.merge_patch(&repo_id, &patch_id, crate::git::GitMergeStrategy::FastForwardOnly, None).await;
        assert!(matches!(result, Err(ForgeError::FastForwardNotPossible)));

        // Check merge should report ff not available
        let check = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(!check.available_strategies.contains(&"fast-forward".to_string()));
    }

    #[tokio::test]
    async fn test_e2e_check_merge_with_protection() {
        let node = create_test_node().await;
        let identity = node.create_repo("e2e-prot", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // Setup protected branch
        let protection = crate::protection::BranchProtection {
            ref_pattern: "heads/main".to_string(),
            required_approvals: 1,
            required_ci_contexts: vec!["ci/pipeline".to_string()],
            dismiss_stale_approvals: false,
        };
        let protection_store = crate::protection::ProtectionStore::new(node.kv.clone());
        protection_store.set_rule(&repo_id, &protection).await.unwrap();

        // Initial commit + patch
        let h0 = node.git.store_blob(b"base".to_vec()).await.unwrap();
        let tree0 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h0)]).await.unwrap();
        let c0 = node.git.commit(tree0, vec![], "init").await.unwrap();
        node.refs.set(&repo_id, "heads/main", c0).await.unwrap();

        let h1 = node.git.store_blob(b"patch".to_vec()).await.unwrap();
        let tree1 = node.git.create_tree(&[crate::git::TreeEntry::file("f.txt", h1)]).await.unwrap();
        let c1 = node.git.commit(tree1, vec![c0], "patch").await.unwrap();
        let patch_id = node.cobs.create_patch(&repo_id, "Protected", "Test", c0, c1).await.unwrap();

        // Check — should be blocked (no approval, no CI)
        let check = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(!check.mergeable);
        assert!(!check.protection_satisfied);
        assert!(check.protection_reason.is_some());

        // Add CI status
        let status_store = crate::status::StatusStore::new(node.kv.clone());
        let ci_status = crate::status::CommitStatus {
            repo_id: repo_id.clone(),
            commit: *c1.as_bytes(),
            context: "ci/pipeline".to_string(),
            state: crate::status::CommitCheckState::Success,
            description: "Build passed".to_string(),
            pipeline_run_id: None,
            created_at_ms: 1000,
        };
        status_store.set_status(&ci_status).await.unwrap();

        // Still blocked (no approval)
        let check = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(!check.mergeable);

        // Add approval
        node.cobs.approve_patch(&repo_id, &patch_id, c1, None).await.unwrap();

        // Now should be mergeable
        let check = node.check_merge(&repo_id, &patch_id).await.unwrap();
        assert!(check.mergeable);
        assert!(check.protection_satisfied);
    }

    // ========================================================================
    // Fork tests
    // ========================================================================

    #[tokio::test]
    async fn test_fork_repo_basic() {
        let node = create_test_node().await;

        // Create upstream with a commit
        let upstream = node.create_repo("upstream", vec![node.public_key()], 1).await.unwrap();
        let upstream_id = upstream.repo_id();
        let commit = node.init_repo(&upstream_id, "Initial commit").await.unwrap();

        // Fork it
        let fork = node.fork_repo(&upstream_id, "my-fork", vec![node.public_key()], 1).await.unwrap();
        let fork_id = fork.repo_id();

        // Fork should have ForkInfo
        assert!(fork.is_fork());
        assert_eq!(fork.fork_info.as_ref().unwrap().upstream_repo_id, upstream_id);
        assert!(fork.fork_info.as_ref().unwrap().upstream_cluster.is_none());

        // Fork should have same head
        let fork_head = node.get_head(&fork_id).await.unwrap();
        assert_eq!(fork_head, Some(commit));
    }

    #[tokio::test]
    async fn test_fork_of_fork_lineage() {
        let node = create_test_node().await;

        let upstream = node.create_repo("upstream", vec![node.public_key()], 1).await.unwrap();
        let upstream_id = upstream.repo_id();
        node.init_repo(&upstream_id, "init").await.unwrap();

        let fork1 = node.fork_repo(&upstream_id, "fork1", vec![node.public_key()], 1).await.unwrap();
        let fork1_id = fork1.repo_id();

        let fork2 = node.fork_repo(&fork1_id, "fork2", vec![node.public_key()], 1).await.unwrap();

        // fork1 points to upstream
        assert_eq!(fork1.fork_info.as_ref().unwrap().upstream_repo_id, upstream_id);
        // fork2 points to fork1
        assert_eq!(fork2.fork_info.as_ref().unwrap().upstream_repo_id, fork1_id);
    }

    #[tokio::test]
    async fn test_fork_nonexistent_repo_error() {
        let node = create_test_node().await;

        let fake_id = RepoId::from_hash(blake3::hash(b"nonexistent"));
        let result = node.fork_repo(&fake_id, "bad-fork", vec![node.public_key()], 1).await;
        assert!(matches!(result, Err(ForgeError::RepoNotFound { .. })));
    }

    // ========================================================================
    // Mirror tests
    // ========================================================================

    #[tokio::test]
    async fn test_mirror_config_persistence() {
        let node = create_test_node().await;
        let identity = node.create_repo("mirrortest", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // No mirror initially
        let config = node.get_mirror_config(&repo_id).await.unwrap();
        assert!(config.is_none());

        // Set mirror config
        let upstream_id = RepoId::from_hash(blake3::hash(b"upstream"));
        let config = crate::mirror::MirrorConfig::new(upstream_id, 300);
        node.set_mirror_config(&repo_id, &config).await.unwrap();

        // Retrieve it
        let retrieved = node.get_mirror_config(&repo_id).await.unwrap().unwrap();
        assert_eq!(retrieved.upstream_repo_id, upstream_id);
        assert_eq!(retrieved.interval_secs, 300);
        assert!(retrieved.enabled);

        // Delete it
        node.delete_mirror_config(&repo_id).await.unwrap();
        let config = node.get_mirror_config(&repo_id).await.unwrap();
        assert!(config.is_none());
    }

    #[tokio::test]
    async fn test_mirror_interval_clamping() {
        // Below minimum
        let config = crate::mirror::MirrorConfig::new(RepoId([0; 32]), 5);
        assert_eq!(config.interval_secs, crate::constants::MIN_MIRROR_INTERVAL_SECS);

        // Above maximum
        let config = crate::mirror::MirrorConfig::new(RepoId([0; 32]), 99999);
        assert_eq!(config.interval_secs, crate::constants::MAX_MIRROR_INTERVAL_SECS);
    }

    #[tokio::test]
    async fn test_mirror_status() {
        let node = create_test_node().await;
        let identity = node.create_repo("statustest", vec![node.public_key()], 1).await.unwrap();
        let repo_id = identity.repo_id();

        // No status without config
        let status = node.get_mirror_status(&repo_id).await.unwrap();
        assert!(status.is_none());

        // Set config, check status
        let upstream_id = RepoId::from_hash(blake3::hash(b"upstream"));
        let config = crate::mirror::MirrorConfig::new(upstream_id, 60);
        node.set_mirror_config(&repo_id, &config).await.unwrap();

        let status = node.get_mirror_status(&repo_id).await.unwrap().unwrap();
        assert!(status.is_due); // Never synced, so always due
        assert_eq!(status.config.interval_secs, 60);
    }
}
