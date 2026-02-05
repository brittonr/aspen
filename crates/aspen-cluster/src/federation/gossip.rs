//! Federation gossip for cross-cluster announcements.
//!
//! This module provides real-time gossip-based communication between federated
//! clusters, enabling instant announcements of cluster availability and resource
//! updates across the federation.
//!
//! # Overview
//!
//! While DHT discovery provides eventual discovery of clusters, gossip provides
//! **real-time** notification of:
//!
//! - Cluster availability changes (online/offline)
//! - New federated resources being seeded
//! - Updates to existing federated resources
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │                      FederationGossipService                            │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────────┐                   ┌──────────────────┐           │
//! │  │ ClusterIdentity  │                   │   AppRegistry    │           │
//! │  │   (for signing)  │                   │ (capabilities)   │           │
//! │  └────────┬─────────┘                   └────────┬─────────┘           │
//! │           │                                      │                      │
//! │           ▼                                      ▼                      │
//! │  ┌────────────────────────────────────────────────────────────┐        │
//! │  │              SignedFederationMessage                       │        │
//! │  │  ┌──────────────────────────────────────────────────────┐ │        │
//! │  │  │ FederationGossipMessage                              │ │        │
//! │  │  │  • ClusterOnline { key, name, nodes, apps, hlc }     │ │        │
//! │  │  │  • ResourceSeeding { fed_id, cluster, refs }         │ │        │
//! │  │  │  • ResourceUpdate { fed_id, type, refs }             │ │        │
//! │  │  └──────────────────────────────────────────────────────┘ │        │
//! │  │  + Ed25519 Signature (cluster key)                        │        │
//! │  └────────────────────────────────────────────────────────────┘        │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌────────────────────────────────────────────────────────────┐        │
//! │  │              iroh-gossip (global topic)                    │        │
//! │  │                 Topic: blake3(FEDERATION_TOPIC_PREFIX)     │        │
//! │  └────────────────────────────────────────────────────────────┘        │
//! │                              │                                          │
//! │           ┌──────────────────┼──────────────────┐                       │
//! │           ▼                  ▼                  ▼                       │
//! │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐                   │
//! │  │  Cluster A  │   │  Cluster B  │   │  Cluster C  │                   │
//! │  └─────────────┘   └─────────────┘   └─────────────┘                   │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Federation gossip uses a **global topic** (not cluster-scoped) to broadcast:
//!
//! 1. **Cluster Online**: Announce cluster availability and endpoints
//! 2. **Resource Seeding**: Announce that a cluster seeds a federated resource
//! 3. **Resource Update**: Announce updates to a federated resource
//!
//! # Usage
//!
//! ## Starting the Gossip Service
//!
//! ```ignore
//! use aspen_cluster::federation::{FederationGossipService, ClusterIdentity};
//!
//! let gossip_service = FederationGossipService::with_app_registry(
//!     identity,
//!     endpoint,
//!     gossip,
//!     cancel_token,
//!     "node-1",
//!     app_registry,
//! ).await?;
//!
//! // Start with bootstrap peers (from DHT discovery or configuration)
//! gossip_service.start(bootstrap_peers).await?;
//! ```
//!
//! ## Processing Events
//!
//! ```ignore
//! // Take the event receiver (can only be taken once)
//! let mut events = gossip_service.take_event_receiver().unwrap();
//!
//! while let Some(event) = events.recv().await {
//!     match event {
//!         FederationEvent::ClusterOnline(cluster) => {
//!             println!("Cluster {} is online with {} apps",
//!                 cluster.name, cluster.apps.len());
//!         }
//!         FederationEvent::ResourceSeeding { fed_id, cluster_key, .. } => {
//!             println!("Cluster {} is seeding {}", cluster_key, fed_id.short());
//!         }
//!         FederationEvent::ResourceUpdate { fed_id, update_type, .. } => {
//!             println!("Resource {} updated: {}", fed_id.short(), update_type);
//!         }
//!     }
//! }
//! ```
//!
//! ## Announcing Resources
//!
//! ```ignore
//! // Announce that we're seeding a resource
//! gossip_service.announce_resource_seeding(&fed_id, ref_heads).await?;
//!
//! // Announce a resource update
//! gossip_service.announce_resource_update(&fed_id, "ref_update", ref_heads).await?;
//! ```
//!
//! # Security
//!
//! All messages are cryptographically authenticated:
//!
//! - **Cluster-level signing**: Messages are signed with the cluster's Ed25519 key (not node key),
//!   ensuring cluster-wide identity even as individual nodes change
//! - **Signature verification**: All incoming messages are verified before processing
//! - **Rate limiting**: Per-cluster and global limits prevent spam attacks
//! - **No sensitive data**: Gossip only carries metadata, not actual content
//!
//! ## Signature Verification Flow
//!
//! ```text
//! Incoming Message
//!       │
//!       ▼
//! ┌───────────────┐
//! │ Parse message │──── Fail ──► Drop (logged)
//! └───────┬───────┘
//!         │
//!         ▼
//! ┌───────────────┐
//! │ Rate limit    │──── Exceeded ──► Drop (logged)
//! │ check         │
//! └───────┬───────┘
//!         │
//!         ▼
//! ┌───────────────┐
//! │ Verify        │──── Invalid ──► Drop (warned)
//! │ signature     │
//! └───────┬───────┘
//!         │
//!         ▼
//! ┌───────────────┐
//! │ Process event │
//! └───────────────┘
//! ```
//!
//! # Rate Limiting
//!
//! Federation gossip implements **two-level rate limiting** to prevent abuse:
//!
//! ## Per-Cluster Limits
//!
//! Each cluster is limited to:
//! - **Rate**: 12 messages per minute ([`CLUSTER_RATE_PER_MINUTE`])
//! - **Burst**: 5 messages ([`CLUSTER_RATE_BURST`])
//! - **Tracking**: LRU cache of up to 512 clusters ([`MAX_TRACKED_CLUSTERS`])
//!
//! The token bucket refills at ~0.2 tokens/second, allowing sustained messaging
//! while preventing burst floods.
//!
//! ## Global Limits
//!
//! Across all clusters combined:
//! - **Rate**: 600 messages per minute ([`GLOBAL_RATE_PER_MINUTE`])
//! - **Burst**: 100 messages ([`GLOBAL_RATE_BURST`])
//!
//! When the global limit is exceeded, all incoming messages are dropped until
//! tokens refill.
//!
//! ## Token Bucket Algorithm
//!
//! ```text
//! Tokens replenish:  rate_per_minute / 60 = tokens/second
//!
//! Per-cluster:  12/60 = 0.2 tokens/sec, max 5 tokens
//! Global:      600/60 = 10 tokens/sec, max 100 tokens
//!
//! Example: Cluster sends 5 messages instantly (burst), then must wait
//!          ~5 seconds per message (0.2 tokens/sec refill)
//! ```
//!
//! ## LRU Eviction
//!
//! When the per-cluster tracker reaches [`MAX_TRACKED_CLUSTERS`] (512), the least
//! recently seen cluster is evicted. This bounds memory usage while allowing
//! new clusters to participate in federation.
//!
//! # Message Types
//!
//! | Message | Purpose | Frequency |
//! |---------|---------|-----------|
//! | `ClusterOnline` | Announce availability and apps | Every 60 seconds |
//! | `ResourceSeeding` | Announce resource hosting | On resource creation |
//! | `ResourceUpdate` | Notify of changes | On each update |
//!
//! # Bootstrap
//!
//! Federation gossip bootstraps via:
//! 1. **DHT-discovered peers** from `FederationDiscoveryService` (see [`super::discovery`])
//! 2. **Trusted clusters** from configuration
//! 3. **Gossip-discovered peers** (recursive discovery via `ClusterOnline` messages)
//!
//! # Event Channel
//!
//! The gossip service uses a bounded channel (capacity: 256) for events:
//!
//! - If the channel fills up, new events are dropped (logged as warning)
//! - Consumers should process events promptly to avoid drops
//! - Use `take_event_receiver()` to get ownership of the receiver
//!
//! # Tiger Style Compliance
//!
//! | Resource | Limit | Constant |
//! |----------|-------|----------|
//! | Tracked clusters | 512 | [`MAX_TRACKED_CLUSTERS`] |
//! | Per-cluster rate | 12/min | [`CLUSTER_RATE_PER_MINUTE`] |
//! | Per-cluster burst | 5 | [`CLUSTER_RATE_BURST`] |
//! | Global rate | 600/min | [`GLOBAL_RATE_PER_MINUTE`] |
//! | Global burst | 100 | [`GLOBAL_RATE_BURST`] |
//! | Message size | 4 KB | [`MAX_MESSAGE_SIZE`] |
//! | Event channel | 256 | (hardcoded) |
//!
//! # Shutdown
//!
//! Graceful shutdown is handled via cancellation tokens:
//!
//! ```ignore
//! // Shutdown with 10-second timeout for background tasks
//! gossip_service.shutdown().await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use aspen_core::Signature;
use aspen_core::hlc::HLC;
use aspen_core::hlc::SerializableTimestamp;
use futures::StreamExt;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh_gossip::api::Event;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::trace;
use tracing::warn;

use super::app_registry::AppManifest;
use super::app_registry::SharedAppRegistry;
use super::discovery::DiscoveredCluster;
use super::identity::ClusterIdentity;
use super::types::FederatedId;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// Global federation gossip topic.
pub const FEDERATION_TOPIC_PREFIX: &[u8] = b"aspen:federation:v1";

/// Protocol version for federation gossip messages.
pub const FEDERATION_GOSSIP_VERSION: u8 = 1;

/// Maximum clusters to track in rate limiter.
pub const MAX_TRACKED_CLUSTERS: usize = 512;

/// Rate limit: messages per minute per cluster.
pub const CLUSTER_RATE_PER_MINUTE: u32 = 12;

/// Rate limit: burst capacity per cluster.
pub const CLUSTER_RATE_BURST: u32 = 5;

/// Global rate limit: messages per minute.
pub const GLOBAL_RATE_PER_MINUTE: u32 = 600;

/// Global rate limit: burst capacity.
pub const GLOBAL_RATE_BURST: u32 = 100;

/// Announce interval for cluster online messages.
pub const CLUSTER_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum size of a serialized gossip message.
pub const MAX_MESSAGE_SIZE: usize = 4096;

/// Shutdown timeout for gossip tasks.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);

// ============================================================================
// Message Types
// ============================================================================

/// Federation gossip message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FederationGossipMessage {
    /// Announce that a cluster is online.
    ClusterOnline {
        /// Protocol version.
        version: u8,
        /// Cluster public key.
        cluster_key: [u8; 32],
        /// Human-readable cluster name.
        cluster_name: String,
        /// Iroh node public keys for connectivity.
        node_keys: Vec<[u8; 32]>,
        /// Relay URLs for NAT traversal.
        relay_urls: Vec<String>,
        /// Applications installed on this cluster.
        apps: Vec<AppManifest>,
        /// Legacy capabilities (for backwards compatibility).
        #[serde(default)]
        capabilities: Vec<String>,
        /// HLC timestamp when announced.
        hlc_timestamp: SerializableTimestamp,
    },

    /// Announce that a cluster is seeding a resource.
    ResourceSeeding {
        /// Protocol version.
        version: u8,
        /// Federated resource ID origin cluster key.
        fed_id_origin: [u8; 32],
        /// Federated resource ID local identifier.
        fed_id_local: [u8; 32],
        /// Cluster public key.
        cluster_key: [u8; 32],
        /// Node keys for fetching.
        node_keys: Vec<[u8; 32]>,
        /// Current ref heads (for sync comparison).
        ref_heads: Vec<(String, [u8; 32])>,
        /// HLC timestamp when announced.
        hlc_timestamp: SerializableTimestamp,
    },

    /// Announce an update to a resource.
    ResourceUpdate {
        /// Protocol version.
        version: u8,
        /// Federated resource ID origin cluster key.
        fed_id_origin: [u8; 32],
        /// Federated resource ID local identifier.
        fed_id_local: [u8; 32],
        /// Cluster public key (who made the update).
        cluster_key: [u8; 32],
        /// Update type (e.g., "ref_update", "cob_change").
        update_type: String,
        /// Updated ref heads.
        ref_heads: Vec<(String, [u8; 32])>,
        /// HLC timestamp when announced.
        hlc_timestamp: SerializableTimestamp,
    },
}

impl FederationGossipMessage {
    /// Get the cluster key from any message type.
    pub fn cluster_key(&self) -> Option<PublicKey> {
        let bytes = match self {
            Self::ClusterOnline { cluster_key, .. } => cluster_key,
            Self::ResourceSeeding { cluster_key, .. } => cluster_key,
            Self::ResourceUpdate { cluster_key, .. } => cluster_key,
        };
        PublicKey::from_bytes(bytes).ok()
    }

    /// Get the HLC timestamp from any message type.
    pub fn hlc_timestamp(&self) -> &SerializableTimestamp {
        match self {
            Self::ClusterOnline { hlc_timestamp, .. } => hlc_timestamp,
            Self::ResourceSeeding { hlc_timestamp, .. } => hlc_timestamp,
            Self::ResourceUpdate { hlc_timestamp, .. } => hlc_timestamp,
        }
    }
}

/// Signed federation gossip message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedFederationMessage {
    /// The message payload.
    pub message: FederationGossipMessage,
    /// Ed25519 signature over the serialized message (cluster key).
    pub signature: Signature,
}

impl SignedFederationMessage {
    /// Sign a message with the cluster's secret key.
    pub fn sign(message: FederationGossipMessage, identity: &ClusterIdentity) -> Result<Self> {
        let message_bytes = postcard::to_allocvec(&message).context("failed to serialize message for signing")?;
        let signature = identity.sign(&message_bytes);

        Ok(Self { message, signature })
    }

    /// Verify the signature and return the message if valid.
    ///
    /// Tiger Style: Debug logging added for verification failures without
    /// exposing details in the API (security: don't leak verification reasons).
    pub fn verify(&self) -> Option<&FederationGossipMessage> {
        let cluster_key = self.message.cluster_key()?;

        let message_bytes = match postcard::to_allocvec(&self.message) {
            Ok(bytes) => bytes,
            Err(e) => {
                debug!(error = %e, "federation gossip: failed to serialize message for verification");
                return None;
            }
        };

        let sig_bytes: [u8; 64] = match self.signature.0.try_into() {
            Ok(bytes) => bytes,
            Err(_) => {
                debug!("federation gossip: invalid signature length");
                return None;
            }
        };
        let sig = iroh::Signature::from_bytes(&sig_bytes);

        match cluster_key.verify(&message_bytes, &sig) {
            Ok(()) => Some(&self.message),
            Err(_) => {
                debug!("federation gossip: signature verification failed");
                None
            }
        }
    }

    /// Serialize to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let bytes = postcard::to_allocvec(self).context("failed to serialize signed message")?;
        if bytes.len() > MAX_MESSAGE_SIZE {
            anyhow::bail!("message too large: {} > {}", bytes.len(), MAX_MESSAGE_SIZE);
        }
        Ok(bytes)
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_MESSAGE_SIZE {
            return None;
        }
        postcard::from_bytes(bytes).ok()
    }
}

// ============================================================================
// Rate Limiting
// ============================================================================

/// Token bucket for rate limiting.
#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    rate_per_sec: f64,
    last_update: Instant,
}

impl TokenBucket {
    fn new(rate_per_minute: u32, burst: u32) -> Self {
        let capacity = f64::from(burst);
        Self {
            tokens: capacity,
            capacity,
            rate_per_sec: f64::from(rate_per_minute) / 60.0,
            last_update: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_update = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Per-cluster rate limit entry.
#[derive(Debug)]
struct ClusterRateEntry {
    bucket: TokenBucket,
    last_access: Instant,
}

/// Rate limiter for federation gossip.
#[derive(Debug)]
struct FederationRateLimiter {
    per_cluster: HashMap<PublicKey, ClusterRateEntry>,
    global: TokenBucket,
}

impl FederationRateLimiter {
    fn new() -> Self {
        Self {
            per_cluster: HashMap::with_capacity(MAX_TRACKED_CLUSTERS),
            global: TokenBucket::new(GLOBAL_RATE_PER_MINUTE, GLOBAL_RATE_BURST),
        }
    }

    fn check(&mut self, cluster_key: &PublicKey) -> bool {
        // Check global limit first
        if !self.global.try_consume() {
            return false;
        }

        let now = Instant::now();

        // Check per-cluster limit
        if let Some(entry) = self.per_cluster.get_mut(cluster_key) {
            entry.last_access = now;
            if !entry.bucket.try_consume() {
                return false;
            }
        } else {
            // New cluster - enforce LRU eviction
            if self.per_cluster.len() >= MAX_TRACKED_CLUSTERS {
                self.evict_oldest();
            }

            let mut bucket = TokenBucket::new(CLUSTER_RATE_PER_MINUTE, CLUSTER_RATE_BURST);
            bucket.try_consume();
            self.per_cluster.insert(*cluster_key, ClusterRateEntry {
                bucket,
                last_access: now,
            });
        }

        true
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest_key) =
            self.per_cluster.iter().min_by_key(|(_, entry)| entry.last_access).map(|(key, _)| *key)
        {
            self.per_cluster.remove(&oldest_key);
        }
    }
}

// ============================================================================
// Federation Event
// ============================================================================

/// Event from the federation gossip service.
#[derive(Debug, Clone)]
pub enum FederationEvent {
    /// A cluster came online.
    ClusterOnline(DiscoveredCluster),
    /// A cluster is seeding a resource.
    ResourceSeeding {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// The cluster seeding this resource.
        cluster_key: PublicKey,
        /// Node keys available for fetching.
        node_keys: Vec<PublicKey>,
        /// Current ref heads for sync comparison.
        ref_heads: HashMap<String, [u8; 32]>,
    },
    /// A resource was updated.
    ResourceUpdate {
        /// The federated resource ID.
        fed_id: FederatedId,
        /// The cluster that made the update.
        cluster_key: PublicKey,
        /// Type of update (e.g., "ref_update", "cob_change").
        update_type: String,
        /// Updated ref heads.
        ref_heads: HashMap<String, [u8; 32]>,
    },
}

// ============================================================================
// Federation Gossip Service
// ============================================================================

/// Service for federation gossip.
pub struct FederationGossipService {
    /// Our cluster identity.
    cluster_identity: ClusterIdentity,

    /// The iroh endpoint.
    endpoint: Arc<Endpoint>,

    /// The gossip instance.
    #[allow(dead_code)]
    gossip: Arc<Gossip>,

    /// The gossip sender (for broadcasting).
    sender: RwLock<Option<iroh_gossip::api::GossipSender>>,

    /// Channel to receive federation events.
    event_rx: RwLock<Option<mpsc::Receiver<FederationEvent>>>,

    /// Cancellation token.
    cancel: CancellationToken,

    /// Background task handles.
    tasks: RwLock<Vec<JoinHandle<()>>>,

    /// HLC for timestamping.
    hlc: Arc<HLC>,

    /// Shared app registry for capability announcements.
    app_registry: Option<SharedAppRegistry>,
}

impl FederationGossipService {
    /// Create a new federation gossip service.
    pub async fn new(
        cluster_identity: ClusterIdentity,
        endpoint: Arc<Endpoint>,
        gossip: Arc<Gossip>,
        cancel: CancellationToken,
        node_id: &str,
    ) -> Result<Self> {
        Ok(Self {
            cluster_identity,
            endpoint,
            gossip,
            sender: RwLock::new(None),
            event_rx: RwLock::new(None),
            cancel,
            tasks: RwLock::new(Vec::new()),
            hlc: Arc::new(aspen_core::hlc::create_hlc(node_id)),
            app_registry: None,
        })
    }

    /// Create a new federation gossip service with an app registry.
    ///
    /// The app registry is used to dynamically announce installed applications
    /// to other clusters in the federation.
    pub async fn with_app_registry(
        cluster_identity: ClusterIdentity,
        endpoint: Arc<Endpoint>,
        gossip: Arc<Gossip>,
        cancel: CancellationToken,
        node_id: &str,
        app_registry: SharedAppRegistry,
    ) -> Result<Self> {
        Ok(Self {
            cluster_identity,
            endpoint,
            gossip,
            sender: RwLock::new(None),
            event_rx: RwLock::new(None),
            cancel,
            tasks: RwLock::new(Vec::new()),
            hlc: Arc::new(aspen_core::hlc::create_hlc(node_id)),
            app_registry: Some(app_registry),
        })
    }

    /// Set the app registry after construction.
    ///
    /// This allows setting the registry if it wasn't available at construction time.
    pub fn set_app_registry(&mut self, registry: SharedAppRegistry) {
        self.app_registry = Some(registry);
    }

    /// Compute the federation topic ID.
    pub fn topic_id() -> TopicId {
        let hash = blake3::hash(FEDERATION_TOPIC_PREFIX);
        TopicId::from_bytes(*hash.as_bytes())
    }

    /// Start the gossip service.
    ///
    /// Subscribes to the federation topic and spawns background tasks.
    pub async fn start(&self, bootstrap_peers: Vec<PublicKey>) -> Result<()> {
        let topic_id = Self::topic_id();

        // Subscribe to federation topic
        let topic = self
            .gossip
            .subscribe(topic_id, bootstrap_peers)
            .await
            .context("failed to subscribe to federation topic")?;

        let (sender, receiver) = topic.split();
        let (event_tx, event_rx) = mpsc::channel(256);

        // Store sender and event receiver
        *self.sender.write() = Some(sender.clone());
        *self.event_rx.write() = Some(event_rx);

        // Spawn receiver task
        let receiver_cancel = self.cancel.child_token();
        let receiver_task = tokio::spawn(Self::receiver_loop(receiver, event_tx.clone(), receiver_cancel));

        // Spawn announcer task
        let announcer_cancel = self.cancel.child_token();
        let announcer_identity = self.cluster_identity.clone();
        let announcer_sender = sender;
        let announcer_endpoint = self.endpoint.clone();
        let announcer_hlc = self.hlc.clone();
        let announcer_registry = self.app_registry.clone();
        let announcer_task = tokio::spawn(Self::announcer_loop(
            announcer_identity,
            announcer_sender,
            announcer_endpoint,
            announcer_hlc,
            announcer_registry,
            announcer_cancel,
        ));

        self.tasks.write().push(receiver_task);
        self.tasks.write().push(announcer_task);

        info!(
            topic = %hex::encode(topic_id.as_bytes()),
            cluster = %self.cluster_identity.name(),
            "federation gossip started"
        );

        Ok(())
    }

    /// Receiver loop - process incoming gossip messages.
    async fn receiver_loop(
        mut receiver: iroh_gossip::api::GossipReceiver,
        event_tx: mpsc::Sender<FederationEvent>,
        cancel: CancellationToken,
    ) {
        let mut rate_limiter = FederationRateLimiter::new();

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("federation gossip receiver shutting down");
                    break;
                }
                event = receiver.next() => {
                    match event {
                        Some(Ok(Event::Received(msg))) => {
                            // Parse and verify message
                            let signed = match SignedFederationMessage::from_bytes(&msg.content) {
                                Some(s) => s,
                                None => {
                                    trace!("failed to parse federation gossip message");
                                    continue;
                                }
                            };

                            // Get cluster key and rate limit check
                            let cluster_key = match signed.message.cluster_key() {
                                Some(k) => k,
                                None => continue,
                            };

                            if !rate_limiter.check(&cluster_key) {
                                trace!(
                                    cluster = %cluster_key,
                                    "rate limited federation message"
                                );
                                continue;
                            }

                            // Verify signature
                            let message = match signed.verify() {
                                Some(m) => m,
                                None => {
                                    warn!(
                                        cluster = %cluster_key,
                                        "rejected federation message with invalid signature"
                                    );
                                    continue;
                                }
                            };

                            // Process message
                            let event = match message {
                                FederationGossipMessage::ClusterOnline {
                                    cluster_key,
                                    cluster_name,
                                    node_keys,
                                    relay_urls,
                                    apps,
                                    capabilities,
                                    hlc_timestamp,
                                    ..
                                } => {
                                    let pk = match PublicKey::from_bytes(cluster_key) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let node_keys: Vec<PublicKey> = node_keys
                                        .iter()
                                        .filter_map(|k| PublicKey::from_bytes(k).ok())
                                        .collect();

                                    info!(
                                        cluster_name = %cluster_name,
                                        cluster_key = %pk,
                                        nodes = node_keys.len(),
                                        apps = apps.len(),
                                        "discovered federated cluster via gossip"
                                    );

                                    FederationEvent::ClusterOnline(DiscoveredCluster {
                                        cluster_key: pk,
                                        name: cluster_name.clone(),
                                        node_keys,
                                        relay_urls: relay_urls.clone(),
                                        apps: apps.clone(),
                                        capabilities: capabilities.clone(),
                                        discovered_at: Instant::now(),
                                        announced_at_hlc: hlc_timestamp.clone(),
                                    })
                                }

                                FederationGossipMessage::ResourceSeeding {
                                    fed_id_origin,
                                    fed_id_local,
                                    cluster_key,
                                    node_keys,
                                    ref_heads,
                                    ..
                                } => {
                                    let origin = match PublicKey::from_bytes(fed_id_origin) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let fed_id = FederatedId::new(origin, *fed_id_local);
                                    let ck = match PublicKey::from_bytes(cluster_key) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let node_keys: Vec<PublicKey> = node_keys
                                        .iter()
                                        .filter_map(|k| PublicKey::from_bytes(k).ok())
                                        .collect();
                                    let ref_map: HashMap<String, [u8; 32]> =
                                        ref_heads.iter().cloned().collect();

                                    debug!(
                                        fed_id = %fed_id.short(),
                                        cluster = %ck,
                                        "resource seeding announcement"
                                    );

                                    FederationEvent::ResourceSeeding {
                                        fed_id,
                                        cluster_key: ck,
                                        node_keys,
                                        ref_heads: ref_map,
                                    }
                                }

                                FederationGossipMessage::ResourceUpdate {
                                    fed_id_origin,
                                    fed_id_local,
                                    cluster_key,
                                    update_type,
                                    ref_heads,
                                    ..
                                } => {
                                    let origin = match PublicKey::from_bytes(fed_id_origin) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let fed_id = FederatedId::new(origin, *fed_id_local);
                                    let ck = match PublicKey::from_bytes(cluster_key) {
                                        Ok(k) => k,
                                        Err(_) => continue,
                                    };
                                    let ref_map: HashMap<String, [u8; 32]> =
                                        ref_heads.iter().cloned().collect();

                                    debug!(
                                        fed_id = %fed_id.short(),
                                        cluster = %ck,
                                        update_type = %update_type,
                                        "resource update announcement"
                                    );

                                    FederationEvent::ResourceUpdate {
                                        fed_id,
                                        cluster_key: ck,
                                        update_type: update_type.clone(),
                                        ref_heads: ref_map,
                                    }
                                }
                            };

                            // Send event (non-blocking)
                            if event_tx.try_send(event).is_err() {
                                warn!("federation event channel full, dropping event");
                            }
                        }
                        Some(Ok(Event::NeighborUp(peer))) => {
                            debug!(peer = %peer, "federation gossip neighbor up");
                        }
                        Some(Ok(Event::NeighborDown(peer))) => {
                            debug!(peer = %peer, "federation gossip neighbor down");
                        }
                        Some(Ok(Event::Lagged)) => {
                            warn!("federation gossip lagged, messages may be lost");
                        }
                        Some(Err(e)) => {
                            warn!(error = %e, "federation gossip receiver error");
                        }
                        None => {
                            info!("federation gossip stream ended");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Announcer loop - periodically announce cluster presence.
    async fn announcer_loop(
        identity: ClusterIdentity,
        sender: iroh_gossip::api::GossipSender,
        endpoint: Arc<Endpoint>,
        hlc: Arc<HLC>,
        app_registry: Option<SharedAppRegistry>,
        cancel: CancellationToken,
    ) {
        let mut interval = tokio::time::interval(CLUSTER_ANNOUNCE_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("federation gossip announcer shutting down");
                    break;
                }
                _ = interval.tick() => {
                    let node_keys = vec![*endpoint.id().as_bytes()];
                    let relay_urls: Vec<String> = endpoint
                        .addr()
                        .relay_urls()
                        .map(|u| u.to_string())
                        .collect();

                    // Get apps from registry or use default Forge app for backwards compatibility
                    let apps = match &app_registry {
                        Some(registry) => {
                            let registered = registry.to_announcement_list();
                            if registered.is_empty() {
                                // No apps registered - use default Forge
                                vec![Self::default_forge_app()]
                            } else {
                                registered
                            }
                        }
                        None => {
                            // No registry provided - use default Forge
                            vec![Self::default_forge_app()]
                        }
                    };

                    // Compute legacy capabilities from apps
                    let capabilities: Vec<String> = apps
                        .iter()
                        .flat_map(|app| app.capabilities.iter().cloned())
                        .collect();

                    let message = FederationGossipMessage::ClusterOnline {
                        version: FEDERATION_GOSSIP_VERSION,
                        cluster_key: *identity.public_key().as_bytes(),
                        cluster_name: identity.name().to_string(),
                        node_keys,
                        relay_urls,
                        apps,
                        capabilities,
                        hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
                    };

                    match SignedFederationMessage::sign(message, &identity) {
                        Ok(signed) => match signed.to_bytes() {
                            Ok(bytes) => {
                                if let Err(e) = sender.broadcast(bytes.into()).await {
                                    warn!(error = %e, "failed to broadcast cluster online");
                                } else {
                                    trace!(
                                        cluster = %identity.name(),
                                        "broadcast cluster online announcement"
                                    );
                                }
                            }
                            Err(e) => {
                                warn!(error = %e, "failed to serialize cluster announcement");
                            }
                        },
                        Err(e) => {
                            warn!(error = %e, "failed to sign cluster announcement");
                        }
                    }
                }
            }
        }
    }

    /// Returns the default Forge app manifest for backwards compatibility.
    ///
    /// Used when no app registry is provided or when no apps are registered.
    fn default_forge_app() -> AppManifest {
        AppManifest::new("forge", "1.0.0")
            .with_name("Aspen Forge")
            .with_capabilities(vec!["git", "issues", "patches"])
    }

    /// Announce that we're seeding a federated resource.
    pub async fn announce_resource_seeding(
        &self,
        fed_id: &FederatedId,
        ref_heads: Vec<(String, [u8; 32])>,
    ) -> Result<()> {
        let sender = self.sender.read().clone().context("gossip sender not initialized")?;

        let node_keys = vec![*self.endpoint.id().as_bytes()];

        let message = FederationGossipMessage::ResourceSeeding {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *self.cluster_identity.public_key().as_bytes(),
            node_keys,
            ref_heads,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &self.cluster_identity)?;
        let bytes = signed.to_bytes()?;

        sender.broadcast(bytes.into()).await.context("failed to broadcast resource seeding")?;

        debug!(
            fed_id = %fed_id.short(),
            "announced resource seeding"
        );

        Ok(())
    }

    /// Announce an update to a federated resource.
    pub async fn announce_resource_update(
        &self,
        fed_id: &FederatedId,
        update_type: &str,
        ref_heads: Vec<(String, [u8; 32])>,
    ) -> Result<()> {
        let sender = self.sender.read().clone().context("gossip sender not initialized")?;

        let message = FederationGossipMessage::ResourceUpdate {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *self.cluster_identity.public_key().as_bytes(),
            update_type: update_type.to_string(),
            ref_heads,
            hlc_timestamp: SerializableTimestamp::from(self.hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &self.cluster_identity)?;
        let bytes = signed.to_bytes()?;

        sender.broadcast(bytes.into()).await.context("failed to broadcast resource update")?;

        debug!(
            fed_id = %fed_id.short(),
            update_type = %update_type,
            "announced resource update"
        );

        Ok(())
    }

    /// Get the event receiver.
    ///
    /// Returns None if already taken.
    pub fn take_event_receiver(&self) -> Option<mpsc::Receiver<FederationEvent>> {
        self.event_rx.write().take()
    }

    /// Shutdown the gossip service.
    pub async fn shutdown(&self) -> Result<()> {
        info!("shutting down federation gossip");

        self.cancel.cancel();

        let mut tasks = self.tasks.write();
        for task in tasks.drain(..) {
            tokio::select! {
                result = task => {
                    if let Err(e) = result {
                        warn!(error = %e, "federation gossip task panicked");
                    }
                }
                _ = tokio::time::sleep(SHUTDOWN_TIMEOUT) => {
                    warn!("federation gossip task did not complete within timeout");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity() -> ClusterIdentity {
        ClusterIdentity::generate("test-cluster".to_string())
    }

    fn test_node_key() -> PublicKey {
        iroh::SecretKey::generate(&mut rand::rng()).public()
    }

    fn test_apps() -> Vec<AppManifest> {
        vec![AppManifest::new("forge", "1.0.0").with_capabilities(vec!["git", "issues"])]
    }

    #[test]
    fn test_cluster_online_message_roundtrip() {
        let identity = test_identity();
        let node_keys = vec![*test_node_key().as_bytes()];
        let apps = test_apps();
        let hlc = aspen_core::hlc::create_hlc("test-node");

        let message = FederationGossipMessage::ClusterOnline {
            version: FEDERATION_GOSSIP_VERSION,
            cluster_key: *identity.public_key().as_bytes(),
            cluster_name: identity.name().to_string(),
            node_keys,
            relay_urls: vec!["https://relay.example.com".to_string()],
            apps: apps.clone(),
            capabilities: vec!["git".to_string(), "issues".to_string()],
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &identity).unwrap();
        let bytes = signed.to_bytes().unwrap();

        let parsed = SignedFederationMessage::from_bytes(&bytes).unwrap();
        let verified = parsed.verify().expect("signature should be valid");

        match verified {
            FederationGossipMessage::ClusterOnline { cluster_name, apps, .. } => {
                assert_eq!(cluster_name, identity.name());
                assert_eq!(apps.len(), 1);
                assert_eq!(apps[0].app_id, "forge");
            }
            _ => panic!("expected ClusterOnline"),
        }
    }

    #[test]
    fn test_resource_seeding_message_roundtrip() {
        let identity = test_identity();
        let origin = test_node_key();
        let fed_id = FederatedId::new(origin, [0xab; 32]);
        let hlc = aspen_core::hlc::create_hlc("test-node");

        let message = FederationGossipMessage::ResourceSeeding {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *identity.public_key().as_bytes(),
            node_keys: vec![*test_node_key().as_bytes()],
            ref_heads: vec![("heads/main".to_string(), [0xcd; 32])],
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &identity).unwrap();
        assert!(signed.verify().is_some());
    }

    #[test]
    fn test_resource_update_message_roundtrip() {
        let identity = test_identity();
        let origin = test_node_key();
        let fed_id = FederatedId::new(origin, [0xef; 32]);
        let hlc = aspen_core::hlc::create_hlc("test-node");

        let message = FederationGossipMessage::ResourceUpdate {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *identity.public_key().as_bytes(),
            update_type: "ref_update".to_string(),
            ref_heads: vec![("heads/main".to_string(), [0x12; 32])],
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &identity).unwrap();
        assert!(signed.verify().is_some());
    }

    #[test]
    fn test_signed_message_tamper_detection() {
        let identity = test_identity();
        let hlc = aspen_core::hlc::create_hlc("test-node");

        let mut message = FederationGossipMessage::ClusterOnline {
            version: FEDERATION_GOSSIP_VERSION,
            cluster_key: *identity.public_key().as_bytes(),
            cluster_name: identity.name().to_string(),
            node_keys: vec![],
            relay_urls: vec![],
            apps: vec![],
            capabilities: vec![],
            hlc_timestamp: SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message.clone(), &identity).unwrap();

        // Tamper with the message
        if let FederationGossipMessage::ClusterOnline {
            ref mut cluster_name, ..
        } = message
        {
            *cluster_name = "tampered".to_string();
        }

        let tampered = SignedFederationMessage {
            message,
            signature: signed.signature.clone(),
        };

        assert!(tampered.verify().is_none());
    }

    #[test]
    fn test_rate_limiter_allows_burst() {
        let mut limiter = FederationRateLimiter::new();
        let cluster = test_node_key();

        // Should allow burst capacity
        for _ in 0..CLUSTER_RATE_BURST {
            assert!(limiter.check(&cluster));
        }

        // Should be rate limited
        assert!(!limiter.check(&cluster));
    }

    #[test]
    fn test_rate_limiter_independent_clusters() {
        let mut limiter = FederationRateLimiter::new();
        let cluster1 = test_node_key();
        let cluster2 = test_node_key();

        // Exhaust cluster1's burst
        for _ in 0..CLUSTER_RATE_BURST {
            assert!(limiter.check(&cluster1));
        }
        assert!(!limiter.check(&cluster1));

        // cluster2 should still have full burst
        for _ in 0..CLUSTER_RATE_BURST {
            assert!(limiter.check(&cluster2));
        }
    }

    #[test]
    fn test_topic_id_is_deterministic() {
        let topic1 = FederationGossipService::topic_id();
        let topic2 = FederationGossipService::topic_id();
        assert_eq!(topic1, topic2);
    }
}
