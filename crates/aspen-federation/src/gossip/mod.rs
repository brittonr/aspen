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
//! 1. **DHT-discovered peers** from `FederationDiscoveryService` (see [`crate::discovery`])
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

mod announcer;
mod events;
mod messages;
mod rate_limiter;
mod receiver;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use anyhow::Result;
use aspen_core::hlc::HLC;
pub use events::FederationEvent;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh_gossip::net::Gossip;
use iroh_gossip::proto::TopicId;
pub use messages::FederationGossipMessage;
pub use messages::SignedFederationMessage;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::info;
use tracing::warn;

use crate::app_registry::SharedAppRegistry;
use crate::identity::ClusterIdentity;

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
// Federation Gossip Service
// ============================================================================

/// Service for federation gossip.
pub struct FederationGossipService {
    /// Our cluster identity.
    pub(super) cluster_identity: ClusterIdentity,

    /// The iroh endpoint.
    pub(super) endpoint: Arc<Endpoint>,

    /// The gossip instance.
    #[allow(dead_code)]
    gossip: Arc<Gossip>,

    /// The gossip sender (for broadcasting).
    pub(super) sender: RwLock<Option<iroh_gossip::api::GossipSender>>,

    /// Channel to receive federation events.
    event_rx: RwLock<Option<mpsc::Receiver<FederationEvent>>>,

    /// Cancellation token.
    cancel: CancellationToken,

    /// Background task tracker.
    task_tracker: TaskTracker,

    /// HLC for timestamping.
    pub(super) hlc: Arc<HLC>,

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
            task_tracker: TaskTracker::new(),
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
            task_tracker: TaskTracker::new(),
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
        self.task_tracker.spawn(Self::receiver_loop(receiver, event_tx.clone(), receiver_cancel));

        // Spawn announcer task
        let announcer_cancel = self.cancel.child_token();
        let announcer_identity = self.cluster_identity.clone();
        let announcer_sender = sender;
        let announcer_endpoint = self.endpoint.clone();
        let announcer_hlc = self.hlc.clone();
        let announcer_registry = self.app_registry.clone();
        self.task_tracker.spawn(Self::announcer_loop(
            announcer_identity,
            announcer_sender,
            announcer_endpoint,
            announcer_hlc,
            announcer_registry,
            announcer_cancel,
        ));

        info!(
            topic = %hex::encode(topic_id.as_bytes()),
            cluster = %self.cluster_identity.name(),
            "federation gossip started"
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
        self.task_tracker.close();

        if tokio::time::timeout(SHUTDOWN_TIMEOUT, self.task_tracker.wait()).await.is_err() {
            warn!("federation gossip tasks did not complete within timeout");
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

    fn test_apps() -> Vec<crate::app_registry::AppManifest> {
        vec![crate::app_registry::AppManifest::new("forge", "1.0.0").with_capabilities(vec!["git", "issues"])]
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
            hlc_timestamp: aspen_core::hlc::SerializableTimestamp::from(hlc.new_timestamp()),
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
        let fed_id = crate::types::FederatedId::new(origin, [0xab; 32]);
        let hlc = aspen_core::hlc::create_hlc("test-node");

        let message = FederationGossipMessage::ResourceSeeding {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *identity.public_key().as_bytes(),
            node_keys: vec![*test_node_key().as_bytes()],
            ref_heads: vec![("heads/main".to_string(), [0xcd; 32])],
            hlc_timestamp: aspen_core::hlc::SerializableTimestamp::from(hlc.new_timestamp()),
        };

        let signed = SignedFederationMessage::sign(message, &identity).unwrap();
        assert!(signed.verify().is_some());
    }

    #[test]
    fn test_resource_update_message_roundtrip() {
        let identity = test_identity();
        let origin = test_node_key();
        let fed_id = crate::types::FederatedId::new(origin, [0xef; 32]);
        let hlc = aspen_core::hlc::create_hlc("test-node");

        let message = FederationGossipMessage::ResourceUpdate {
            version: FEDERATION_GOSSIP_VERSION,
            fed_id_origin: *fed_id.origin().as_bytes(),
            fed_id_local: *fed_id.local_id(),
            cluster_key: *identity.public_key().as_bytes(),
            update_type: "ref_update".to_string(),
            ref_heads: vec![("heads/main".to_string(), [0x12; 32])],
            hlc_timestamp: aspen_core::hlc::SerializableTimestamp::from(hlc.new_timestamp()),
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
            hlc_timestamp: aspen_core::hlc::SerializableTimestamp::from(hlc.new_timestamp()),
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
        let mut limiter = rate_limiter::FederationRateLimiter::new();
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
        let mut limiter = rate_limiter::FederationRateLimiter::new();
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
