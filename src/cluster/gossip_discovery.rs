//! Gossip-based peer discovery for Aspen clusters.
//!
//! This module provides automatic peer discovery using iroh-gossip, enabling
//! nodes to announce their presence and discover peers without manual configuration.
//!
//! # Architecture
//!
//! Each node:
//! 1. Subscribes to a cluster-wide gossip topic (derived from cluster cookie)
//! 2. Periodically broadcasts its EndpointAddr (every 10 seconds)
//! 3. Listens for peer announcements and adds them to the Iroh endpoint
//!
//! # Security
//!
//! Messages are signed with the node's SecretKey and verified on receipt.
//! Invalid signatures are rejected (fail-fast).
//!
//! # Test Coverage
//!
//! TODO: Add unit tests for GossipPeerDiscovery:
//!       - Message serialization/deserialization roundtrip
//!       - Peer announcement broadcast timing (10s interval)
//!       - Duplicate peer detection and deduplication
//!       - Shutdown cancellation token behavior
//!       Coverage: 19.55% line coverage - tested via integration tests only
//!
//! TODO: Add tests for edge cases:
//!       - Malformed gossip message handling
//!       - Network partition recovery
//!       - High churn rate peer discovery
//!
//! # Example
//!
//! ```no_run
//! use aspen::cluster::gossip_discovery::GossipPeerDiscovery;
//! use iroh_gossip::proto::TopicId;
//!
//! # async fn example(
//! #     node_id: u64,
//! #     iroh_manager: &aspen::cluster::IrohEndpointManager,
//! #     network_factory: Option<std::sync::Arc<aspen::raft::network::IrpcRaftNetworkFactory>>,
//! # ) -> anyhow::Result<()> {
//! let topic_id = TopicId::from_bytes([1u8; 32]);
//! let discovery = GossipPeerDiscovery::spawn(
//!     topic_id,
//!     node_id,
//!     iroh_manager,
//!     network_factory,
//! ).await?;
//!
//! // Discovery runs in background, automatically connecting to discovered peers...
//!
//! discovery.shutdown().await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use futures::StreamExt;
use iroh::EndpointAddr;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use iroh_gossip::api::Event;
use iroh_gossip::proto::TopicId;
use serde::Deserialize;
use serde::Serialize;
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use super::IrohEndpointManager;
use crate::raft::constants::GOSSIP_ANNOUNCE_FAILURE_THRESHOLD;
use crate::raft::constants::GOSSIP_GLOBAL_BURST;
use crate::raft::constants::GOSSIP_GLOBAL_RATE_PER_MINUTE;
use crate::raft::constants::GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS;
use crate::raft::constants::GOSSIP_MAX_STREAM_RETRIES;
use crate::raft::constants::GOSSIP_MAX_TRACKED_PEERS;
use crate::raft::constants::GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
use crate::raft::constants::GOSSIP_PER_PEER_BURST;
use crate::raft::constants::GOSSIP_PER_PEER_RATE_PER_MINUTE;
use crate::raft::constants::GOSSIP_STREAM_BACKOFF_SECS;
// Use the type alias from cluster mod.rs which provides the concrete type
use super::IrpcRaftNetworkFactory;
use crate::raft::pure::calculate_backoff_duration;
use crate::raft::types::NodeId;
use crate::sharding::topology::TopologyAnnouncement;

/// Current gossip message protocol version.
///
/// Version history:
/// - v1: Initial version (unsigned)
/// - v2: Added Ed25519 signatures for message authentication
///
/// Note: This is a breaking change - old nodes will not parse new messages.
const GOSSIP_MESSAGE_VERSION: u8 = 2;

/// Announcement message broadcast to the gossip topic.
///
/// Contains node's ID, EndpointAddr, and a timestamp for freshness tracking.
///
/// Tiger Style: Fixed-size payload, explicit timestamp in microseconds, versioned for forward
/// compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PeerAnnouncement {
    /// Protocol version for forward compatibility checking.
    version: u8,
    /// Node ID of the announcing node.
    node_id: NodeId,
    /// The endpoint address of the announcing node.
    endpoint_addr: EndpointAddr,
    /// Timestamp when this announcement was created (microseconds since UNIX epoch).
    timestamp_micros: u64,
}

impl PeerAnnouncement {
    /// Create a new announcement with the current timestamp.
    fn new(node_id: NodeId, endpoint_addr: EndpointAddr) -> Result<Self> {
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: GOSSIP_MESSAGE_VERSION,
            node_id,
            endpoint_addr,
            timestamp_micros,
        })
    }

    /// Serialize to bytes using postcard.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize peer announcement")
    }
}

/// Signed peer announcement for cryptographic verification.
///
/// Wraps a `PeerAnnouncement` with an Ed25519 signature from the sender's
/// Iroh secret key. Recipients verify using the public key embedded in
/// the announcement's `endpoint_addr.id`.
///
/// Tiger Style: Fixed 64-byte signature, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedPeerAnnouncement {
    /// The announcement payload (node_id, endpoint_addr, timestamp).
    announcement: PeerAnnouncement,
    /// Ed25519 signature over the serialized announcement (64 bytes).
    signature: Signature,
}

impl SignedPeerAnnouncement {
    /// Create a signed announcement.
    ///
    /// Signs the serialized `PeerAnnouncement` bytes with the provided secret key.
    fn sign(announcement: PeerAnnouncement, secret_key: &SecretKey) -> Result<Self> {
        let announcement_bytes = announcement.to_bytes()?;
        let signature = secret_key.sign(&announcement_bytes);

        Ok(Self {
            announcement,
            signature,
        })
    }

    /// Verify the signature and return the inner announcement if valid.
    ///
    /// Extracts the public key from `announcement.endpoint_addr.id` and verifies
    /// that the signature was created by the corresponding secret key.
    ///
    /// Returns `None` if:
    /// - Signature verification fails (tampered or wrong key)
    /// - Announcement deserialization fails
    fn verify(&self) -> Option<&PeerAnnouncement> {
        // Re-serialize announcement to get canonical bytes for verification
        let announcement_bytes = self.announcement.to_bytes().ok()?;

        // The endpoint_addr.id IS the PublicKey
        let public_key = self.announcement.endpoint_addr.id;

        // Verify signature
        match public_key.verify(&announcement_bytes, &self.signature) {
            Ok(()) => Some(&self.announcement),
            Err(_) => None,
        }
    }

    /// Serialize to bytes using postcard.
    #[allow(dead_code)]
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize signed peer announcement")
    }

    /// Deserialize from bytes using postcard.
    ///
    /// Returns None for unknown future versions to allow graceful handling.
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let signed: Self = postcard::from_bytes(bytes).ok()?;

        // Reject unknown future versions
        if signed.announcement.version > GOSSIP_MESSAGE_VERSION {
            return None;
        }

        Some(signed)
    }
}

/// Signed topology announcement for cryptographic verification.
///
/// Wraps a `TopologyAnnouncement` with an Ed25519 signature from the sender's
/// Iroh secret key. Used to broadcast topology version updates so nodes can
/// detect when they have stale topology information.
///
/// Tiger Style: Fixed 64-byte signature, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedTopologyAnnouncement {
    /// The topology announcement payload.
    announcement: TopologyAnnouncement,
    /// Ed25519 signature over the serialized announcement (64 bytes).
    signature: Signature,
    /// Public key of the signing node (for verification without endpoint_addr).
    public_key: PublicKey,
}

impl SignedTopologyAnnouncement {
    /// Create a signed topology announcement.
    #[allow(dead_code)]
    fn sign(announcement: TopologyAnnouncement, secret_key: &SecretKey) -> Result<Self> {
        let announcement_bytes =
            postcard::to_stdvec(&announcement).context("failed to serialize topology announcement")?;
        let signature = secret_key.sign(&announcement_bytes);
        let public_key = secret_key.public();

        Ok(Self {
            announcement,
            signature,
            public_key,
        })
    }

    /// Verify the signature and return the inner announcement if valid.
    fn verify(&self) -> Option<&TopologyAnnouncement> {
        let announcement_bytes = postcard::to_stdvec(&self.announcement).ok()?;

        match self.public_key.verify(&announcement_bytes, &self.signature) {
            Ok(()) => Some(&self.announcement),
            Err(_) => None,
        }
    }

    /// Serialize to bytes using postcard.
    #[allow(dead_code)]
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize signed topology announcement")
    }

    /// Deserialize from bytes using postcard.
    #[allow(dead_code)]
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let signed: Self = postcard::from_bytes(bytes).ok()?;

        // Reject unknown future versions
        if signed.announcement.version > TopologyAnnouncement::PROTOCOL_VERSION {
            return None;
        }

        Some(signed)
    }
}

/// Blob announcement for P2P content seeding.
///
/// Announces that a node has a specific blob available for download.
/// Recipients can use iroh-blobs to fetch the content if needed.
///
/// Tiger Style: Fixed-size payload, explicit timestamp, versioned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobAnnouncement {
    /// Protocol version for forward compatibility checking.
    pub version: u8,
    /// Node ID of the node offering this blob.
    pub node_id: NodeId,
    /// Endpoint address for downloading the blob.
    pub endpoint_addr: EndpointAddr,
    /// BLAKE3 hash of the blob content.
    pub blob_hash: iroh_blobs::Hash,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Format of the blob (Raw or HashSeq).
    pub blob_format: iroh_blobs::BlobFormat,
    /// Timestamp when this announcement was created (microseconds since UNIX epoch).
    pub timestamp_micros: u64,
    /// Optional tag for categorization (e.g., "kv-offload", "user-upload").
    /// Max 64 bytes to limit payload size.
    pub tag: Option<String>,
}

impl BlobAnnouncement {
    /// Maximum tag length in bytes.
    const MAX_TAG_LEN: usize = 64;

    /// Create a new blob announcement with the current timestamp.
    pub fn new(
        node_id: NodeId,
        endpoint_addr: EndpointAddr,
        blob_hash: iroh_blobs::Hash,
        blob_size: u64,
        blob_format: iroh_blobs::BlobFormat,
        tag: Option<String>,
    ) -> Result<Self> {
        // Validate tag length (Tiger Style: bounded strings)
        if let Some(ref t) = tag
            && t.len() > Self::MAX_TAG_LEN
        {
            anyhow::bail!("blob announcement tag too long: {} > {}", t.len(), Self::MAX_TAG_LEN);
        }

        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: GOSSIP_MESSAGE_VERSION,
            node_id,
            endpoint_addr,
            blob_hash,
            blob_size,
            blob_format,
            timestamp_micros,
            tag,
        })
    }

    /// Serialize to bytes using postcard.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize blob announcement")
    }
}

/// Signed blob announcement for cryptographic verification.
///
/// Wraps a `BlobAnnouncement` with an Ed25519 signature from the sender's
/// Iroh secret key. Recipients verify using the public key embedded in
/// the announcement's `endpoint_addr.id`.
///
/// Tiger Style: Fixed 64-byte signature, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedBlobAnnouncement {
    /// The blob announcement payload.
    pub announcement: BlobAnnouncement,
    /// Ed25519 signature over the serialized announcement (64 bytes).
    pub signature: Signature,
}

impl SignedBlobAnnouncement {
    /// Create a signed blob announcement.
    pub fn sign(announcement: BlobAnnouncement, secret_key: &SecretKey) -> Result<Self> {
        let announcement_bytes = announcement.to_bytes()?;
        let signature = secret_key.sign(&announcement_bytes);

        Ok(Self {
            announcement,
            signature,
        })
    }

    /// Verify the signature and return the inner announcement if valid.
    pub fn verify(&self) -> Option<&BlobAnnouncement> {
        let announcement_bytes = self.announcement.to_bytes().ok()?;
        let public_key = self.announcement.endpoint_addr.id;

        match public_key.verify(&announcement_bytes, &self.signature) {
            Ok(()) => Some(&self.announcement),
            Err(_) => None,
        }
    }
}

/// Envelope for gossip messages supporting multiple message types.
///
/// This allows peer announcements, topology announcements, and blob announcements
/// to share the same gossip topic while being distinguishable by type.
///
/// Tiger Style: Versioned enum for forward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
enum GossipMessage {
    /// Peer endpoint address announcement.
    PeerAnnouncement(SignedPeerAnnouncement),
    /// Topology version announcement (for cache invalidation).
    TopologyAnnouncement(SignedTopologyAnnouncement),
    /// Blob availability announcement for P2P content seeding.
    BlobAnnouncement(SignedBlobAnnouncement),
}

impl GossipMessage {
    /// Serialize to bytes using postcard.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize gossip message")
    }

    /// Deserialize from bytes using postcard.
    ///
    /// Falls back to parsing as legacy SignedPeerAnnouncement for backwards compatibility.
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        // Try new envelope format first
        if let Ok(msg) = postcard::from_bytes::<Self>(bytes) {
            return Some(msg);
        }

        // Fall back to legacy SignedPeerAnnouncement format for backwards compat
        if let Some(signed) = SignedPeerAnnouncement::from_bytes(bytes) {
            return Some(GossipMessage::PeerAnnouncement(signed));
        }

        None
    }
}

// ============================================================================
// Rate Limiting (HIGH-6 Security Enhancement)
// ============================================================================

/// Token bucket state for rate limiting.
///
/// Tracks tokens and last update time for a single rate limit bucket.
/// Tokens are replenished over time up to the burst capacity.
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of available tokens.
    tokens: f64,
    /// Maximum tokens (burst capacity).
    capacity: f64,
    /// Tokens added per second.
    rate_per_sec: f64,
    /// Last time tokens were updated.
    last_update: Instant,
}

impl TokenBucket {
    /// Create a new token bucket with the specified rate and burst capacity.
    fn new(rate_per_minute: u32, burst: u32) -> Self {
        let capacity = f64::from(burst);
        Self {
            tokens: capacity, // Start full
            capacity,
            rate_per_sec: f64::from(rate_per_minute) / 60.0,
            last_update: Instant::now(),
        }
    }

    /// Try to consume one token. Returns true if allowed, false if rate limited.
    ///
    /// Replenishes tokens based on elapsed time before checking.
    fn try_consume(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();

        // Replenish tokens based on elapsed time
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_update = now;

        // Try to consume one token
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Per-peer rate limit entry with LRU tracking.
#[derive(Debug)]
struct PeerRateEntry {
    bucket: TokenBucket,
    /// Last access time for LRU eviction.
    last_access: Instant,
}

/// Rate limiter for gossip messages.
///
/// Implements two-tier rate limiting:
/// 1. Per-peer: Limits messages from any single peer (prevents individual abuse)
/// 2. Global: Limits total message rate (prevents cluster-wide DoS)
///
/// Tiger Style: Fixed bounds on tracked peers (LRU eviction), explicit rate limits.
///
/// # Security Properties
///
/// - Prevents single-peer message flooding
/// - Prevents Sybil attacks with many peers each sending moderate traffic
/// - Rate limiting happens BEFORE signature verification to save CPU
/// - Bounded memory: max 256 tracked peers (~16KB overhead)
#[derive(Debug)]
struct GossipRateLimiter {
    /// Per-peer rate limit buckets, keyed by PublicKey.
    /// Tiger Style: Bounded to GOSSIP_MAX_TRACKED_PEERS with LRU eviction.
    per_peer: HashMap<PublicKey, PeerRateEntry>,
    /// Global rate limit bucket for all messages.
    global: TokenBucket,
}

impl GossipRateLimiter {
    /// Create a new rate limiter with configured limits.
    fn new() -> Self {
        Self {
            per_peer: HashMap::with_capacity(GOSSIP_MAX_TRACKED_PEERS),
            global: TokenBucket::new(GOSSIP_GLOBAL_RATE_PER_MINUTE, GOSSIP_GLOBAL_BURST),
        }
    }

    /// Check if a message from the given peer should be allowed.
    ///
    /// Returns `Ok(())` if allowed, `Err(RateLimitReason)` if rate limited.
    ///
    /// Tiger Style: Check global limit first (cheaper), then per-peer.
    fn check(&mut self, peer_id: &PublicKey) -> std::result::Result<(), RateLimitReason> {
        // Check global limit first (single bucket, cheaper)
        if !self.global.try_consume() {
            return Err(RateLimitReason::Global);
        }

        // Check per-peer limit
        let now = Instant::now();

        // Get or create per-peer entry
        if let Some(entry) = self.per_peer.get_mut(peer_id) {
            entry.last_access = now;
            if !entry.bucket.try_consume() {
                return Err(RateLimitReason::PerPeer);
            }
        } else {
            // New peer - enforce LRU eviction if at capacity
            if self.per_peer.len() >= GOSSIP_MAX_TRACKED_PEERS {
                self.evict_oldest();
            }

            // Create new entry (starts with full bucket, so first message always allowed)
            let mut bucket = TokenBucket::new(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST);
            bucket.try_consume(); // Consume token for this message
            self.per_peer.insert(
                *peer_id,
                PeerRateEntry {
                    bucket,
                    last_access: now,
                },
            );
        }

        Ok(())
    }

    /// Evict the least recently accessed peer entry.
    ///
    /// Tiger Style: O(n) scan is acceptable for bounded n=256.
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self.per_peer.iter().min_by_key(|(_, entry)| entry.last_access).map(|(key, _)| *key) {
            self.per_peer.remove(&oldest_key);
        }
    }
}

/// Reason for rate limiting a gossip message.
#[derive(Debug, Clone, Copy)]
enum RateLimitReason {
    /// Per-peer rate limit exceeded.
    PerPeer,
    /// Global rate limit exceeded.
    Global,
}

/// Manages gossip-based peer discovery lifecycle.
///
/// Spawns two background tasks:
/// 1. Announcer: Periodically broadcasts this node's ID and EndpointAddr
/// 2. Receiver: Listens for peer announcements and automatically adds them to the network factory
///
/// Tiger Style: Bounded announcement interval (10s), explicit shutdown mechanism.
///
/// ## Task Lifecycle Management
///
/// - Uses CancellationToken for clean shutdown coordination
/// - Implements Drop to abort tasks if struct dropped without shutdown()
/// - Tasks check cancellation token for responsive shutdown
/// - Bounded shutdown timeout (10s) with explicit task abortion
pub struct GossipPeerDiscovery {
    topic_id: TopicId,
    _node_id: NodeId, // Stored for debugging/logging purposes
    cancel_token: CancellationToken,
    // Tiger Style: Option allows moving out in shutdown() while still implementing Drop
    announcer_task: Option<JoinHandle<()>>,
    receiver_task: Option<JoinHandle<()>>,
}

impl GossipPeerDiscovery {
    // Note: Announcement interval is now controlled by GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS
    // and GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS from constants.rs with adaptive backoff.

    /// Spawn gossip peer discovery tasks.
    ///
    /// Subscribes to the gossip topic and starts background tasks for
    /// announcing this node's ID/address and receiving peer announcements.
    ///
    /// If `network_factory` is provided, discovered peers are automatically
    /// added to it for Raft networking.
    ///
    /// Tiger Style: Fail fast if gossip is not enabled or subscription fails.
    pub async fn spawn(
        topic_id: TopicId,
        node_id: NodeId,
        iroh_manager: &IrohEndpointManager,
        network_factory: Option<Arc<IrpcRaftNetworkFactory>>,
    ) -> Result<Self> {
        // Get gossip instance or fail
        let gossip = iroh_manager.gossip().context("gossip not enabled on IrohEndpointManager")?;

        // Subscribe to the topic with timeout
        // Tiger Style: Explicit timeout prevents indefinite blocking during subscription.
        // If gossip is unavailable, the node should continue without it (non-fatal).
        let gossip_topic =
            tokio::time::timeout(crate::raft::constants::GOSSIP_SUBSCRIBE_TIMEOUT, gossip.subscribe(topic_id, vec![]))
                .await
                .context("timeout subscribing to gossip topic")?
                .context("failed to subscribe to gossip topic")?;

        // Split into sender and receiver
        let (gossip_sender, mut gossip_receiver) = gossip_topic.split();

        let cancel_token = CancellationToken::new();
        let endpoint_addr = iroh_manager.node_addr().clone();
        let secret_key = iroh_manager.secret_key().clone();

        // Spawn announcer task
        let announcer_cancel = cancel_token.child_token();
        let announcer_sender = gossip_sender.clone();
        let announcer_node_id = node_id;
        let announcer_task = tokio::spawn(async move {
            // Adaptive interval tracking for sender-side rate limiting
            let mut consecutive_failures: u32 = 0;
            let mut current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
            let mut ticker = interval(Duration::from_secs(current_interval_secs));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = announcer_cancel.cancelled() => {
                        tracing::debug!("gossip announcer shutting down");
                        break;
                    }
                    _ = ticker.tick() => {
                        let announcement =
                            match PeerAnnouncement::new(announcer_node_id, endpoint_addr.clone()) {
                                Ok(ann) => ann,
                                Err(e) => {
                                    tracing::error!("failed to create peer announcement: {}", e);
                                    continue;
                                }
                            };

                        // Sign the announcement with our secret key
                        let signed = match SignedPeerAnnouncement::sign(announcement, &secret_key) {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::error!("failed to sign peer announcement: {}", e);
                                continue;
                            }
                        };

                        // Wrap in GossipMessage envelope for versioned message handling
                        let gossip_msg = GossipMessage::PeerAnnouncement(signed);
                        match gossip_msg.to_bytes() {
                            Ok(bytes) => {
                                match announcer_sender.broadcast(bytes.into()).await {
                                    Ok(()) => {
                                        tracing::trace!(
                                            "broadcast signed peer announcement for node_id={}",
                                            announcer_node_id
                                        );
                                        // Reset on success if we were in backoff mode
                                        if consecutive_failures > 0 {
                                            consecutive_failures = 0;
                                            if current_interval_secs != GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS {
                                                current_interval_secs = GOSSIP_MIN_ANNOUNCE_INTERVAL_SECS;
                                                ticker = interval(Duration::from_secs(current_interval_secs));
                                                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                                                tracing::debug!(
                                                    "announcement succeeded, resetting interval to {}s",
                                                    current_interval_secs
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        consecutive_failures += 1;
                                        tracing::warn!(
                                            "failed to broadcast peer announcement (failure {}/{}): {}",
                                            consecutive_failures,
                                            GOSSIP_ANNOUNCE_FAILURE_THRESHOLD,
                                            e
                                        );

                                        // Increase interval after threshold failures
                                        if consecutive_failures >= GOSSIP_ANNOUNCE_FAILURE_THRESHOLD {
                                            let new_interval = (current_interval_secs * 2)
                                                .min(GOSSIP_MAX_ANNOUNCE_INTERVAL_SECS);
                                            if new_interval != current_interval_secs {
                                                current_interval_secs = new_interval;
                                                ticker = interval(Duration::from_secs(current_interval_secs));
                                                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                                                tracing::info!(
                                                    "increasing announcement interval to {}s due to failures",
                                                    current_interval_secs
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("failed to serialize signed peer announcement: {}", e);
                            }
                        }
                    }
                }
            }
        });

        // Spawn receiver task
        let receiver_cancel = cancel_token.child_token();
        let receiver_node_id = node_id;
        let receiver_network_factory = network_factory.clone();
        let receiver_task = tokio::spawn(async move {
            // Rate limiter for incoming gossip messages (HIGH-6 security)
            let mut rate_limiter = GossipRateLimiter::new();

            // Error recovery state for transient stream errors
            let mut consecutive_errors: u32 = 0;
            let backoff_durations: Vec<Duration> =
                GOSSIP_STREAM_BACKOFF_SECS.iter().map(|s| Duration::from_secs(*s)).collect();

            loop {
                tokio::select! {
                    _ = receiver_cancel.cancelled() => {
                        tracing::debug!("gossip receiver shutting down");
                        break;
                    }
                    event = gossip_receiver.next() => match event {
                    Some(Ok(Event::Received(msg))) => {
                        // Reset error count on successful message
                        consecutive_errors = 0;
                        // Rate limit check BEFORE parsing/signature verification (save CPU)
                        // Use delivered_from as the peer identifier
                        if let Err(reason) = rate_limiter.check(&msg.delivered_from) {
                            match reason {
                                RateLimitReason::PerPeer => {
                                    // Per-peer limit: drop silently (avoid log spam)
                                    tracing::trace!(
                                        "rate limited gossip message from peer={:?}",
                                        msg.delivered_from
                                    );
                                }
                                RateLimitReason::Global => {
                                    // Global limit: log at warn level (indicates attack)
                                    tracing::warn!(
                                        "global gossip rate limit exceeded, dropping message from peer={:?}",
                                        msg.delivered_from
                                    );
                                }
                            }
                            continue;
                        }

                        // Parse gossip message (supports both peer and topology announcements)
                        let gossip_msg = match GossipMessage::from_bytes(&msg.content) {
                            Some(m) => m,
                            None => {
                                tracing::warn!("failed to parse gossip message");
                                continue;
                            }
                        };

                        match gossip_msg {
                            GossipMessage::PeerAnnouncement(signed) => {
                                // Verify signature - reject if invalid
                                let announcement = match signed.verify() {
                                    Some(ann) => ann,
                                    None => {
                                        tracing::warn!(
                                            "rejected peer announcement with invalid signature from endpoint_id={:?}",
                                            signed.announcement.endpoint_addr.id
                                        );
                                        continue;
                                    }
                                };

                                // Filter out our own announcements
                                if announcement.node_id == receiver_node_id {
                                    tracing::trace!("ignoring self-announcement");
                                    continue;
                                }

                                tracing::debug!(
                                    "received verified peer announcement from node_id={}, endpoint_id={:?}",
                                    announcement.node_id,
                                    announcement.endpoint_addr.id
                                );

                                // Add peer to network factory's fallback cache.
                                //
                                // ARCHITECTURAL NOTE: Gossip discovery intentionally does NOT
                                // automatically trigger add_learner() to add discovered peers to
                                // Raft membership. This separation is by design:
                                //
                                // 1. Transport vs Application Layer: Gossip provides transport-layer
                                //    connectivity (who can I talk to?), while Raft membership is an
                                //    application-layer concern (who is part of the cluster?).
                                //
                                // 2. Security: Automatic promotion would allow any gossiping node to
                                //    join the cluster, creating a Sybil attack vector.
                                //
                                // 3. Bounded Growth: Manual add_learner() calls ensure cluster
                                //    membership is explicitly controlled by operators.
                                //
                                // 4. Address Persistence: Addresses ARE persisted! When a peer is
                                //    added via add_learner(), their RaftMemberInfo (with iroh_addr) is
                                //    stored in Raft membership and persisted to the state machine.
                                //    On restart, these addresses are recovered automatically.
                                //
                                // The network factory's peer_addrs map is just a fallback cache for
                                // addresses not yet in Raft membership. Once a peer is promoted to
                                // learner/voter, their address comes from the Raft membership state.
                                if let Some(ref factory) = receiver_network_factory {
                                    factory
                                        .add_peer(
                                            announcement.node_id,
                                            announcement.endpoint_addr.clone(),
                                        )
                                        .await;

                                    tracing::info!(
                                        "added peer to network factory: node_id={}, endpoint_id={:?}",
                                        announcement.node_id,
                                        announcement.endpoint_addr.id
                                    );
                                }

                                // Log the discovery details
                                let relay_urls: Vec<_> =
                                    announcement.endpoint_addr.relay_urls().collect();
                                tracing::info!(
                                    "discovered verified peer: node_id={}, endpoint_id={:?}, relay={:?}, direct_addresses={}",
                                    announcement.node_id,
                                    announcement.endpoint_addr.id,
                                    relay_urls,
                                    announcement.endpoint_addr.addrs.len()
                                );
                            }
                            GossipMessage::TopologyAnnouncement(signed) => {
                                // Verify signature - reject if invalid
                                let announcement = match signed.verify() {
                                    Some(ann) => ann,
                                    None => {
                                        tracing::warn!(
                                            "rejected topology announcement with invalid signature from node_id={}",
                                            signed.announcement.node_id
                                        );
                                        continue;
                                    }
                                };

                                // Filter out our own announcements
                                if announcement.node_id == u64::from(receiver_node_id) {
                                    tracing::trace!("ignoring self-topology-announcement");
                                    continue;
                                }

                                tracing::debug!(
                                    "received verified topology announcement from node_id={}, version={}, hash={}",
                                    announcement.node_id,
                                    announcement.topology_version,
                                    announcement.topology_hash
                                );

                                // TODO: Compare with local topology version and trigger sync if stale.
                                // This will be implemented when we add the topology sync RPC.
                                // For now, just log the announcement so nodes can observe topology changes.
                                tracing::info!(
                                    "topology announcement: node_id={}, version={}, term={}",
                                    announcement.node_id,
                                    announcement.topology_version,
                                    announcement.term
                                );
                            }
                            GossipMessage::BlobAnnouncement(signed) => {
                                // Verify signature - reject if invalid
                                let announcement = match signed.verify() {
                                    Some(ann) => ann,
                                    None => {
                                        tracing::warn!(
                                            "rejected blob announcement with invalid signature from node_id={}",
                                            u64::from(signed.announcement.node_id)
                                        );
                                        continue;
                                    }
                                };

                                // Filter out our own announcements
                                if u64::from(announcement.node_id) == u64::from(receiver_node_id) {
                                    tracing::trace!("ignoring self-blob-announcement");
                                    continue;
                                }

                                // Log the blob availability announcement
                                // In the future, this could trigger background blob fetching
                                // for redundancy or prefetching based on access patterns.
                                tracing::info!(
                                    hash = %announcement.blob_hash.fmt_short(),
                                    size = announcement.blob_size,
                                    format = ?announcement.blob_format,
                                    node_id = u64::from(announcement.node_id),
                                    tag = ?announcement.tag,
                                    "discovered blob available from peer"
                                );

                                // TODO: Optionally trigger background blob download for redundancy
                                // if let Some(blob_store) = &blob_store_opt {
                                //     let provider = announcement.endpoint_addr.id;
                                //     if !blob_store.has(&announcement.blob_hash).await.unwrap_or(false) {
                                //         // Download in background for redundancy
                                //         blob_store.download_from_peer(&announcement.blob_hash, provider).await;
                                //     }
                                // }
                            }
                        }
                    }
                    Some(Ok(Event::NeighborUp(neighbor_id))) => {
                        tracing::debug!("neighbor up: {:?}", neighbor_id);
                    }
                    Some(Ok(Event::NeighborDown(neighbor_id))) => {
                        tracing::debug!("neighbor down: {:?}", neighbor_id);
                    }
                    Some(Ok(Event::Lagged)) => {
                        tracing::warn!("gossip receiver lagged, messages may be lost");
                    }
                    Some(Err(e)) => {
                        consecutive_errors += 1;

                        // Check if we've exceeded max retries
                        if consecutive_errors > GOSSIP_MAX_STREAM_RETRIES {
                            tracing::error!(
                                "gossip receiver exceeded max retries ({}), giving up: {}",
                                GOSSIP_MAX_STREAM_RETRIES,
                                e
                            );
                            break;
                        }

                        // Calculate backoff duration using existing pure function
                        let backoff = calculate_backoff_duration(
                            (consecutive_errors as usize).saturating_sub(1),
                            &backoff_durations
                        );

                        tracing::warn!(
                            "gossip receiver error (retry {}/{}), backing off for {:?}: {}",
                            consecutive_errors,
                            GOSSIP_MAX_STREAM_RETRIES,
                            backoff,
                            e
                        );

                        // Wait before retrying - stream may recover
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                    None => {
                        tracing::info!("gossip receiver stream ended");
                        break;
                    }
                    } // end of match event
                } // end of tokio::select!
            }
        });

        Ok(Self {
            topic_id,
            _node_id: node_id,
            cancel_token,
            announcer_task: Some(announcer_task),
            receiver_task: Some(receiver_task),
        })
    }

    /// Get the topic ID for this discovery instance.
    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    /// Shutdown the discovery tasks and wait for completion.
    ///
    /// Tiger Style: Bounded wait time (10 seconds max), explicit task abortion on timeout.
    pub async fn shutdown(mut self) -> Result<()> {
        tracing::info!("shutting down gossip peer discovery");

        // Signal cancellation to both tasks
        self.cancel_token.cancel();

        // Wait for tasks with timeout
        let timeout = Duration::from_secs(10);

        // Take tasks out of Option (they will be None after this, preventing Drop from aborting)
        let mut announcer_task =
            self.announcer_task.take().context("announcer task not initialized or already consumed")?;
        let mut receiver_task =
            self.receiver_task.take().context("receiver task not initialized or already consumed")?;

        tokio::select! {
            result = &mut announcer_task => {
                match result {
                    Ok(()) => tracing::debug!("announcer task completed"),
                    Err(e) => tracing::error!("announcer task panicked: {}", e),
                }
            }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("announcer task did not complete within timeout, aborting");
                // Tiger Style: Explicit task abortion to prevent leak
                announcer_task.abort();
            }
        }

        tokio::select! {
            result = &mut receiver_task => {
                match result {
                    Ok(()) => tracing::debug!("receiver task completed"),
                    Err(e) => tracing::error!("receiver task panicked: {}", e),
                }
            }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("receiver task did not complete within timeout, aborting");
                // Tiger Style: Explicit task abortion to prevent leak
                receiver_task.abort();
            }
        }

        Ok(())
    }
}

/// Broadcast a blob announcement to the gossip network.
///
/// This announces that the local node has a blob available for P2P download.
/// Other nodes can use this information to fetch the blob for redundancy
/// or when they need the content.
///
/// # Arguments
/// * `iroh_manager` - The Iroh endpoint manager (for gossip access)
/// * `topic_id` - The gossip topic to broadcast on
/// * `node_id` - Our node ID
/// * `blob_hash` - BLAKE3 hash of the blob
/// * `blob_size` - Size of the blob in bytes
/// * `blob_format` - Format of the blob (Raw or HashSeq)
/// * `tag` - Optional categorization tag
///
/// # Returns
/// Ok(()) on success, Err if broadcast fails
pub async fn broadcast_blob_announcement(
    iroh_manager: &super::IrohEndpointManager,
    topic_id: TopicId,
    node_id: NodeId,
    blob_hash: iroh_blobs::Hash,
    blob_size: u64,
    blob_format: iroh_blobs::BlobFormat,
    tag: Option<String>,
) -> Result<()> {
    let gossip = iroh_manager.gossip().context("gossip not enabled")?;
    let endpoint_addr = iroh_manager.node_addr().clone();
    let secret_key = iroh_manager.secret_key();

    // Create and sign the announcement
    let announcement = BlobAnnouncement::new(node_id, endpoint_addr, blob_hash, blob_size, blob_format, tag)?;
    let signed = SignedBlobAnnouncement::sign(announcement, secret_key)?;
    let message = GossipMessage::BlobAnnouncement(signed);
    let bytes = message.to_bytes()?;

    // Subscribe briefly to get a sender, then broadcast
    // Note: This creates a new subscription which may not be ideal for frequent announcements.
    // For high-frequency blob announcements, consider caching the topic/sender.
    let mut topic = gossip.subscribe(topic_id, vec![]).await.context("failed to subscribe to gossip topic")?;
    topic.broadcast(bytes.into()).await.context("failed to broadcast blob announcement")?;

    tracing::debug!(
        hash = %blob_hash.fmt_short(),
        size = blob_size,
        format = ?blob_format,
        "broadcast blob announcement"
    );

    Ok(())
}

/// Tiger Style: Abort tasks if dropped without explicit shutdown().
///
/// This prevents task leaks when GossipPeerDiscovery is dropped (e.g., on panic
/// or if shutdown() is not called). The tasks will be aborted immediately.
///
/// If shutdown() was called, tasks will already be None and this is a no-op.
impl Drop for GossipPeerDiscovery {
    fn drop(&mut self) {
        if self.announcer_task.is_some() || self.receiver_task.is_some() {
            tracing::warn!("GossipPeerDiscovery dropped without shutdown(), aborting tasks");
            if let Some(task) = self.announcer_task.take() {
                task.abort();
            }
            if let Some(task) = self.receiver_task.take() {
                task.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_announcement_serialize_deserialize() {
        let node_id = 123u64;
        let addr = EndpointAddr::new(iroh::SecretKey::from([1u8; 32]).public());
        let announcement = PeerAnnouncement::new(node_id.into(), addr).unwrap();

        let bytes = announcement.to_bytes().unwrap();
        let deserialized: PeerAnnouncement = postcard::from_bytes(&bytes).unwrap();

        assert_eq!(announcement.node_id, deserialized.node_id);
        assert_eq!(announcement.endpoint_addr, deserialized.endpoint_addr);
        assert_eq!(announcement.timestamp_micros, deserialized.timestamp_micros);
    }

    #[test]
    fn test_peer_announcement_timestamp() {
        let node_id = 456u64;
        let addr = EndpointAddr::new(iroh::SecretKey::from([1u8; 32]).public());
        let announcement1 = PeerAnnouncement::new(node_id.into(), addr.clone()).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let announcement2 = PeerAnnouncement::new(node_id.into(), addr).unwrap();

        // Second announcement should have a later timestamp
        assert!(announcement2.timestamp_micros > announcement1.timestamp_micros);
    }

    #[test]
    fn test_signed_peer_announcement_sign_and_verify() {
        let secret_key = SecretKey::from([42u8; 32]);
        let node_id = 789u64;
        let addr = EndpointAddr::new(secret_key.public());

        let announcement = PeerAnnouncement::new(node_id.into(), addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement.clone(), &secret_key).unwrap();

        // Verify should succeed with correct key
        let verified = signed.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap().node_id, announcement.node_id);
    }

    #[test]
    fn test_signed_peer_announcement_roundtrip() {
        let secret_key = SecretKey::from([99u8; 32]);
        let node_id = 111u64;
        let addr = EndpointAddr::new(secret_key.public());

        let announcement = PeerAnnouncement::new(node_id.into(), addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        // Serialize and deserialize
        let bytes = signed.to_bytes().unwrap();
        let deserialized = SignedPeerAnnouncement::from_bytes(&bytes).unwrap();

        // Verify still works after roundtrip
        assert!(deserialized.verify().is_some());
    }

    #[test]
    fn test_signed_peer_announcement_wrong_key_rejected() {
        let real_key = SecretKey::from([11u8; 32]);
        let fake_key = SecretKey::from([22u8; 32]);
        let node_id = 222u64;

        // Announcement claims to be from real_key's identity
        let addr = EndpointAddr::new(real_key.public());
        let announcement = PeerAnnouncement::new(node_id.into(), addr).unwrap();

        // But signed with fake_key (attacker trying to impersonate)
        let signed = SignedPeerAnnouncement {
            announcement,
            signature: fake_key.sign(b"some message"),
        };

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_peer_announcement_tampered_rejected() {
        let secret_key = SecretKey::from([33u8; 32]);
        let node_id = 333u64;
        let addr = EndpointAddr::new(secret_key.public());

        let announcement = PeerAnnouncement::new(node_id.into(), addr.clone()).unwrap();
        let mut signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        // Tamper with the announcement after signing
        signed.announcement.node_id = 999u64.into();

        // Verification should fail (signature doesn't match modified content)
        assert!(signed.verify().is_none());
    }

    // ========================================================================
    // Rate Limiter Tests (HIGH-6)
    // ========================================================================

    #[test]
    fn test_token_bucket_allows_burst() {
        let mut bucket = TokenBucket::new(12, 3); // 12/min, burst 3

        // Should allow burst capacity
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());
        assert!(bucket.try_consume());

        // Fourth should fail (burst exhausted)
        assert!(!bucket.try_consume());
    }

    #[test]
    fn test_token_bucket_replenishes() {
        let mut bucket = TokenBucket::new(60, 1); // 1 per second, burst 1

        // Exhaust the bucket
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());

        // Wait for replenishment (1+ second)
        std::thread::sleep(std::time::Duration::from_millis(1100));

        // Should have replenished
        assert!(bucket.try_consume());
    }

    #[test]
    fn test_rate_limiter_allows_first_messages() {
        let mut limiter = GossipRateLimiter::new();
        let peer1 = SecretKey::from([1u8; 32]).public();
        let peer2 = SecretKey::from([2u8; 32]).public();

        // First message from each peer should be allowed
        assert!(limiter.check(&peer1).is_ok());
        assert!(limiter.check(&peer2).is_ok());
    }

    #[test]
    fn test_rate_limiter_per_peer_limit() {
        let mut limiter = GossipRateLimiter::new();
        let peer = SecretKey::from([3u8; 32]).public();

        // Exhaust per-peer burst (3 messages)
        assert!(limiter.check(&peer).is_ok());
        assert!(limiter.check(&peer).is_ok());
        assert!(limiter.check(&peer).is_ok());

        // Fourth should be rate limited
        let result = limiter.check(&peer);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RateLimitReason::PerPeer));
    }

    #[test]
    fn test_rate_limiter_per_peer_independent() {
        let mut limiter = GossipRateLimiter::new();
        let peer1 = SecretKey::from([4u8; 32]).public();
        let peer2 = SecretKey::from([5u8; 32]).public();

        // Exhaust peer1's burst
        for _ in 0..3 {
            assert!(limiter.check(&peer1).is_ok());
        }
        assert!(limiter.check(&peer1).is_err());

        // peer2 should still have full burst
        assert!(limiter.check(&peer2).is_ok());
        assert!(limiter.check(&peer2).is_ok());
        assert!(limiter.check(&peer2).is_ok());
    }

    #[test]
    fn test_rate_limiter_lru_eviction() {
        // Test eviction logic directly without rate limiting interference
        let mut limiter = GossipRateLimiter::new();

        // Manually insert entries up to capacity
        let now = Instant::now();
        for i in 0..GOSSIP_MAX_TRACKED_PEERS {
            let mut key_bytes = [0u8; 32];
            key_bytes[0] = (i & 0xFF) as u8;
            key_bytes[1] = ((i >> 8) & 0xFF) as u8;
            let peer = SecretKey::from(key_bytes).public();

            // Directly insert entries to avoid rate limiting
            limiter.per_peer.insert(
                peer,
                PeerRateEntry {
                    bucket: TokenBucket::new(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST),
                    last_access: now,
                },
            );
        }

        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);

        // Trigger eviction by adding one more
        limiter.evict_oldest();
        let new_peer = SecretKey::from([0xFF; 32]).public();
        limiter.per_peer.insert(
            new_peer,
            PeerRateEntry {
                bucket: TokenBucket::new(GOSSIP_PER_PEER_RATE_PER_MINUTE, GOSSIP_PER_PEER_BURST),
                last_access: now,
            },
        );

        // Should still be at capacity (not over)
        assert_eq!(limiter.per_peer.len(), GOSSIP_MAX_TRACKED_PEERS);
    }

    #[test]
    fn test_rate_limiter_global_limit() {
        // Test with a custom bucket that has zero replenishment rate
        // to avoid time-based replenishment affecting the test
        let mut limiter = GossipRateLimiter {
            per_peer: HashMap::with_capacity(GOSSIP_MAX_TRACKED_PEERS),
            // Use 0 rate per minute so no replenishment during test
            global: TokenBucket::new(0, GOSSIP_GLOBAL_BURST),
        };

        // Exhaust global burst (100 messages from different peers)
        for i in 0..GOSSIP_GLOBAL_BURST {
            let mut key_bytes = [0u8; 32];
            key_bytes[0] = (i & 0xFF) as u8;
            key_bytes[1] = ((i >> 8) & 0xFF) as u8;
            let peer = SecretKey::from(key_bytes).public();
            assert!(limiter.check(&peer).is_ok(), "message {} should be allowed", i);
        }

        // Next message should hit global limit
        let extra_peer = SecretKey::from([0xFE; 32]).public();
        let result = limiter.check(&extra_peer);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RateLimitReason::Global));
    }

    #[test]
    fn test_evict_oldest_removes_least_recent() {
        let mut limiter = GossipRateLimiter::new();
        let now = Instant::now();

        // Insert peer A with older timestamp
        let peer_a = SecretKey::from([0xAA; 32]).public();
        limiter.per_peer.insert(
            peer_a,
            PeerRateEntry {
                bucket: TokenBucket::new(12, 3),
                last_access: now - std::time::Duration::from_secs(100),
            },
        );

        // Insert peer B with newer timestamp
        let peer_b = SecretKey::from([0xBB; 32]).public();
        limiter.per_peer.insert(
            peer_b,
            PeerRateEntry {
                bucket: TokenBucket::new(12, 3),
                last_access: now,
            },
        );

        // Evict oldest
        limiter.evict_oldest();

        // peer_a (older) should be evicted, peer_b should remain
        assert!(!limiter.per_peer.contains_key(&peer_a));
        assert!(limiter.per_peer.contains_key(&peer_b));
    }

    // ========================================================================
    // Topology Announcement Tests
    // ========================================================================

    #[test]
    fn test_signed_topology_announcement_sign_and_verify() {
        let secret_key = SecretKey::from([42u8; 32]);
        let announcement = TopologyAnnouncement::new(123, 1, 0xDEADBEEF, 5, 1000000);
        let signed = SignedTopologyAnnouncement::sign(announcement.clone(), &secret_key).unwrap();

        // Verify should succeed with correct key
        let verified = signed.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap().node_id, announcement.node_id);
        assert_eq!(verified.unwrap().topology_version, announcement.topology_version);
    }

    #[test]
    fn test_signed_topology_announcement_roundtrip() {
        let secret_key = SecretKey::from([99u8; 32]);
        let announcement = TopologyAnnouncement::new(456, 2, 0xCAFEBABE, 10, 2000000);
        let signed = SignedTopologyAnnouncement::sign(announcement, &secret_key).unwrap();

        // Serialize and deserialize
        let bytes = signed.to_bytes().unwrap();
        let deserialized = SignedTopologyAnnouncement::from_bytes(&bytes).unwrap();

        // Verify still works after roundtrip
        assert!(deserialized.verify().is_some());
        assert_eq!(deserialized.verify().unwrap().topology_version, signed.verify().unwrap().topology_version);
    }

    #[test]
    fn test_signed_topology_announcement_wrong_key_rejected() {
        let real_key = SecretKey::from([11u8; 32]);
        let fake_key = SecretKey::from([22u8; 32]);
        let announcement = TopologyAnnouncement::new(789, 3, 0x12345678, 15, 3000000);

        // Sign with real key, but replace the public_key with a different one
        let mut signed = SignedTopologyAnnouncement::sign(announcement, &real_key).unwrap();
        signed.public_key = fake_key.public();

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    // ========================================================================
    // GossipMessage Envelope Tests
    // ========================================================================

    #[test]
    fn test_gossip_message_peer_announcement_roundtrip() {
        let secret_key = SecretKey::from([50u8; 32]);
        let node_id = 100u64;
        let addr = EndpointAddr::new(secret_key.public());

        let announcement = PeerAnnouncement::new(node_id.into(), addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();
        let msg = GossipMessage::PeerAnnouncement(signed);

        let bytes = msg.to_bytes().unwrap();
        let deserialized = GossipMessage::from_bytes(&bytes).unwrap();

        match deserialized {
            GossipMessage::PeerAnnouncement(s) => {
                assert!(s.verify().is_some());
            }
            _ => panic!("expected PeerAnnouncement"),
        }
    }

    #[test]
    fn test_gossip_message_topology_announcement_roundtrip() {
        let secret_key = SecretKey::from([60u8; 32]);
        let announcement = TopologyAnnouncement::new(200, 5, 0xAABBCCDD, 20, 4000000);
        let signed = SignedTopologyAnnouncement::sign(announcement, &secret_key).unwrap();
        let msg = GossipMessage::TopologyAnnouncement(signed);

        let bytes = msg.to_bytes().unwrap();
        let deserialized = GossipMessage::from_bytes(&bytes).unwrap();

        match deserialized {
            GossipMessage::TopologyAnnouncement(s) => {
                assert!(s.verify().is_some());
                assert_eq!(s.verify().unwrap().topology_version, 5);
            }
            _ => panic!("expected TopologyAnnouncement"),
        }
    }

    #[test]
    fn test_gossip_message_backwards_compat_legacy_peer_announcement() {
        let secret_key = SecretKey::from([70u8; 32]);
        let node_id = 300u64;
        let addr = EndpointAddr::new(secret_key.public());

        let announcement = PeerAnnouncement::new(node_id.into(), addr).unwrap();
        let signed = SignedPeerAnnouncement::sign(announcement, &secret_key).unwrap();

        // Serialize using the OLD format (just SignedPeerAnnouncement, not GossipMessage)
        let legacy_bytes = signed.to_bytes().unwrap();

        // Should still parse as GossipMessage via backwards compat fallback
        let deserialized = GossipMessage::from_bytes(&legacy_bytes).unwrap();

        match deserialized {
            GossipMessage::PeerAnnouncement(s) => {
                assert!(s.verify().is_some());
            }
            _ => panic!("expected PeerAnnouncement from legacy format"),
        }
    }

    // ========================================================================
    // Blob Announcement Tests
    // ========================================================================

    #[test]
    fn test_blob_announcement_sign_and_verify() {
        let secret_key = SecretKey::from([42u8; 32]);
        let node_id = 123u64;
        let addr = EndpointAddr::new(secret_key.public());
        let blob_hash = iroh_blobs::Hash::from_bytes([1u8; 32]);

        let announcement = BlobAnnouncement::new(
            node_id.into(),
            addr,
            blob_hash,
            1024,
            iroh_blobs::BlobFormat::Raw,
            Some("kv-offload".to_string()),
        )
        .unwrap();

        let signed = SignedBlobAnnouncement::sign(announcement.clone(), &secret_key).unwrap();

        // Verify should succeed with correct key
        let verified = signed.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap().node_id, announcement.node_id);
        assert_eq!(verified.unwrap().blob_hash, blob_hash);
        assert_eq!(verified.unwrap().blob_size, 1024);
        assert_eq!(verified.unwrap().tag, Some("kv-offload".to_string()));
    }

    #[test]
    fn test_blob_announcement_roundtrip() {
        let secret_key = SecretKey::from([99u8; 32]);
        let node_id = 456u64;
        let addr = EndpointAddr::new(secret_key.public());
        let blob_hash = iroh_blobs::Hash::from_bytes([2u8; 32]);

        let announcement =
            BlobAnnouncement::new(node_id.into(), addr, blob_hash, 2048, iroh_blobs::BlobFormat::HashSeq, None)
                .unwrap();

        let signed = SignedBlobAnnouncement::sign(announcement, &secret_key).unwrap();
        let msg = GossipMessage::BlobAnnouncement(signed);

        // Serialize and deserialize
        let bytes = msg.to_bytes().unwrap();
        let deserialized = GossipMessage::from_bytes(&bytes).unwrap();

        match deserialized {
            GossipMessage::BlobAnnouncement(s) => {
                assert!(s.verify().is_some());
                assert_eq!(s.verify().unwrap().blob_size, 2048);
                assert_eq!(s.verify().unwrap().blob_format, iroh_blobs::BlobFormat::HashSeq);
            }
            _ => panic!("expected BlobAnnouncement"),
        }
    }

    #[test]
    fn test_blob_announcement_wrong_key_rejected() {
        let real_key = SecretKey::from([11u8; 32]);
        let fake_key = SecretKey::from([22u8; 32]);
        let node_id = 789u64;

        // Announcement claims to be from real_key's identity
        let addr = EndpointAddr::new(real_key.public());
        let blob_hash = iroh_blobs::Hash::from_bytes([3u8; 32]);
        let announcement =
            BlobAnnouncement::new(node_id.into(), addr, blob_hash, 512, iroh_blobs::BlobFormat::Raw, None).unwrap();

        // But signed with fake_key (attacker trying to impersonate)
        let signed = SignedBlobAnnouncement {
            announcement,
            signature: fake_key.sign(b"some message"),
        };

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_blob_announcement_tampered_rejected() {
        let secret_key = SecretKey::from([33u8; 32]);
        let node_id = 333u64;
        let addr = EndpointAddr::new(secret_key.public());
        let blob_hash = iroh_blobs::Hash::from_bytes([4u8; 32]);

        let announcement =
            BlobAnnouncement::new(node_id.into(), addr, blob_hash, 4096, iroh_blobs::BlobFormat::Raw, None).unwrap();

        let mut signed = SignedBlobAnnouncement::sign(announcement, &secret_key).unwrap();

        // Tamper with the announcement after signing
        signed.announcement.blob_size = 9999;

        // Verification should fail (signature doesn't match modified content)
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_blob_announcement_tag_too_long() {
        let secret_key = SecretKey::from([44u8; 32]);
        let addr = EndpointAddr::new(secret_key.public());
        let blob_hash = iroh_blobs::Hash::from_bytes([5u8; 32]);

        // Tag too long (>64 bytes)
        let long_tag = "a".repeat(65);
        let result =
            BlobAnnouncement::new(1u64.into(), addr, blob_hash, 1024, iroh_blobs::BlobFormat::Raw, Some(long_tag));

        assert!(result.is_err());
    }

    #[test]
    fn test_blob_announcement_max_tag_length_ok() {
        let secret_key = SecretKey::from([55u8; 32]);
        let addr = EndpointAddr::new(secret_key.public());
        let blob_hash = iroh_blobs::Hash::from_bytes([6u8; 32]);

        // Tag at max length (exactly 64 bytes)
        let max_tag = "a".repeat(64);
        let result = BlobAnnouncement::new(
            1u64.into(),
            addr,
            blob_hash,
            1024,
            iroh_blobs::BlobFormat::Raw,
            Some(max_tag.clone()),
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap().tag, Some(max_tag));
    }
}
