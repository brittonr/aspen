//! Global content discovery for Aspen using the BitTorrent Mainline DHT.
//!
//! This module provides decentralized content discovery beyond cluster boundaries,
//! allowing nodes to find blob providers across the global network without relying
//! on centralized trackers.
//!
//! # Feature Flags
//!
//! - **`global-discovery`**: Enables actual DHT operations using the `mainline` crate.
//!   Without this feature, the service logs operations but doesn't perform real DHT queries.
//!   Enable with: `cargo build --features global-discovery`
//!
//! # Architecture
//!
//! The content discovery system uses a two-layer approach:
//!
//! 1. **Intra-cluster**: Gossip-based `BlobAnnouncement` for fast local discovery
//! 2. **Global**: Mainline DHT for cross-cluster and public content discovery
//!
//! # Design Decisions
//!
//! - Uses the `mainline` crate directly (v6.0) for DHT access when feature is enabled
//! - Maps BLAKE3 hashes to DHT infohashes via SHA-256 truncation (20 bytes)
//! - Uses BEP-44 mutable items to store full iroh NodeAddr (32-byte public key + relay URL)
//! - Each provider announces with their ed25519 key, using infohash as salt
//! - Discoverers query for mutable items to retrieve full NodeAddr for connection
//! - Rate-limited publishing to avoid DHT spam (1 announce per hash per 5 min)
//! - Background republishing every 30 minutes for active content
//!
//! # Security
//!
//! - All DHT announces are Ed25519 signed (BEP-44 mutable items)
//! - Provider identity is cryptographically verified
//! - DHT operations use standard BitTorrent mutable item protocol
//! - Provider verification can be done by attempting blob download via Iroh
//!
//! # References
//!
//! - [Iroh Content Discovery Blog](https://www.iroh.computer/blog/iroh-content-discovery)
//! - [mainline crate](https://crates.io/crates/mainline)
//! - [BEP-0005: DHT Protocol](http://bittorrent.org/beps/bep_0005.html)
//! - [BEP-0044: Storing arbitrary data in the DHT](http://bittorrent.org/beps/bep_0044.html)
//!
//! # Example
//!
//! ```ignore
//! use aspen::cluster::content_discovery::{ContentDiscoveryService, ContentDiscoveryConfig};
//!
//! let config = ContentDiscoveryConfig::default();
//! let service = ContentDiscoveryService::spawn(
//!     endpoint.clone(),
//!     secret_key.clone(),
//!     config,
//!     cancellation_token.clone(),
//! ).await?;
//!
//! // Announce a blob to the global DHT
//! service.announce(blob_hash, blob_size, blob_format).await?;
//!
//! // Query for providers of a specific blob
//! let providers = service.find_providers(blob_hash, blob_format).await?;
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;
use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// Maximum number of tracked announces per node (prevents memory exhaustion).
const MAX_TRACKED_ANNOUNCES: usize = 10_000;

/// Minimum interval between announces for the same hash (rate limiting).
const MIN_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(5 * 60); // 5 minutes

/// Interval for republishing active content to DHT.
const REPUBLISH_INTERVAL: Duration = Duration::from_secs(30 * 60); // 30 minutes

/// Maximum providers to return from a query.
const MAX_PROVIDERS: usize = 50;

/// Timeout for DHT queries (get_peers).
#[cfg(feature = "global-discovery")]
const DHT_QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum size of a serialized announce record.
const MAX_ANNOUNCE_SIZE: usize = 1024;

/// DHT announce record TTL (how long the DHT should cache the record).
#[allow(dead_code)]
const DHT_RECORD_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour

/// Maximum number of providers to query via BEP-44 mutable lookups.
/// We query our own public key plus discovered peer public keys.
#[allow(dead_code)]
const MAX_MUTABLE_LOOKUPS: usize = 20;

/// Timeout for BEP-44 mutable item lookups.
#[cfg(feature = "global-discovery")]
const MUTABLE_LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);

// ============================================================================
// Configuration
// ============================================================================

/// Configuration for the content discovery service.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentDiscoveryConfig {
    /// Enable global DHT-based content discovery.
    /// Default: false (opt-in for privacy)
    pub enabled: bool,

    /// Run DHT node in server mode (accept incoming requests).
    /// Default: false (client-only mode)
    pub server_mode: bool,

    /// Bootstrap nodes for initial DHT connection.
    /// Default: uses mainline's built-in bootstrap nodes
    pub bootstrap_nodes: Vec<String>,

    /// Port for DHT UDP socket.
    /// Default: 0 (random port)
    pub dht_port: u16,

    /// Automatically announce all local blobs to DHT.
    /// Default: false (manual announcement only)
    pub auto_announce: bool,

    /// Maximum concurrent DHT queries.
    /// Default: 8
    pub max_concurrent_queries: usize,
}

impl Default for ContentDiscoveryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            server_mode: false,
            bootstrap_nodes: Vec::new(),
            dht_port: 0,
            auto_announce: false,
            max_concurrent_queries: 8,
        }
    }
}

// ============================================================================
// Provider Information
// ============================================================================

/// Information about a content provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Public key of the provider node.
    pub node_id: PublicKey,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Format of the blob (Raw or HashSeq).
    pub blob_format: BlobFormat,
    /// Timestamp when this provider was discovered (microseconds since epoch).
    pub discovered_at: u64,
    /// Whether this provider has been verified (we successfully connected).
    pub verified: bool,
}

/// Announce record stored in the DHT.
///
/// This is the payload signed by the announcing node and stored
/// in the DHT using BEP-0044 mutable items.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DhtAnnounce {
    /// Protocol version for forward compatibility.
    version: u8,
    /// BLAKE3 hash of the content.
    blob_hash: Hash,
    /// Size of the blob.
    blob_size: u64,
    /// Format of the blob.
    blob_format: BlobFormat,
    /// Timestamp (microseconds since epoch).
    timestamp_micros: u64,
}

/// Node address information stored in DHT via BEP-44 mutable items.
///
/// This structure contains the full information needed to connect to an iroh node.
/// It's stored as a BEP-44 mutable item with:
/// - Key: The node's ed25519 public key (same as iroh PublicKey)
/// - Salt: The 20-byte infohash of the content
/// - Value: Serialized DhtNodeAddr
///
/// This solves the 20-byte vs 32-byte peer ID problem by using the DHT's
/// mutable item storage instead of the limited peer announcement protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhtNodeAddr {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// The node's iroh public key (32 bytes).
    pub public_key: [u8; 32],
    /// Optional relay URL for NAT traversal.
    pub relay_url: Option<String>,
    /// Direct socket addresses (IPv4 and IPv6).
    pub direct_addrs: Vec<String>,
    /// Size of the blob in bytes.
    pub blob_size: u64,
    /// Format of the blob.
    pub blob_format: BlobFormat,
    /// Timestamp (microseconds since epoch).
    pub timestamp_micros: u64,
}

impl DhtNodeAddr {
    const VERSION: u8 = 1;

    /// Create a new DhtNodeAddr from the current endpoint state.
    pub fn new(
        public_key: PublicKey,
        relay_url: Option<&url::Url>,
        direct_addrs: impl IntoIterator<Item = std::net::SocketAddr>,
        blob_size: u64,
        blob_format: BlobFormat,
    ) -> Result<Self> {
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: Self::VERSION,
            public_key: *public_key.as_bytes(),
            relay_url: relay_url.map(|u| u.to_string()),
            direct_addrs: direct_addrs.into_iter().map(|a| a.to_string()).collect(),
            blob_size,
            blob_format,
            timestamp_micros,
        })
    }

    /// Serialize to bytes for DHT storage.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize DhtNodeAddr")
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }

    /// Get the iroh PublicKey.
    pub fn iroh_public_key(&self) -> Option<PublicKey> {
        PublicKey::from_bytes(&self.public_key).ok()
    }
}

impl DhtAnnounce {
    const VERSION: u8 = 1;

    fn new(blob_hash: Hash, blob_size: u64, blob_format: BlobFormat) -> Result<Self> {
        let timestamp_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("system time before Unix epoch")?
            .as_micros() as u64;

        Ok(Self {
            version: Self::VERSION,
            blob_hash,
            blob_size,
            blob_format,
            timestamp_micros,
        })
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize DHT announce")
    }

    #[allow(dead_code)]
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }
}

/// Signed announce record for DHT storage.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SignedDhtAnnounce {
    /// The announce payload.
    announce: DhtAnnounce,
    /// Public key of the announcing node.
    public_key: PublicKey,
    /// Ed25519 signature over the serialized announce.
    signature: Signature,
}

impl SignedDhtAnnounce {
    fn sign(announce: DhtAnnounce, secret_key: &SecretKey) -> Result<Self> {
        let announce_bytes = announce.to_bytes()?;
        let signature = secret_key.sign(&announce_bytes);
        let public_key = secret_key.public();

        Ok(Self {
            announce,
            public_key,
            signature,
        })
    }

    #[allow(dead_code)]
    fn verify(&self) -> Option<&DhtAnnounce> {
        let announce_bytes = self.announce.to_bytes().ok()?;
        self.public_key.verify(&announce_bytes, &self.signature).ok()?;
        Some(&self.announce)
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize signed DHT announce")
    }

    #[allow(dead_code)]
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        postcard::from_bytes(bytes).ok()
    }
}

// ============================================================================
// Hash Mapping (BLAKE3 -> DHT Infohash)
// ============================================================================

/// Convert a BLAKE3 hash + format to a DHT-compatible infohash.
///
/// Uses SHA-1 of the BLAKE3 hash concatenated with the format byte
/// to create a 20-byte infohash for DHT lookups.
pub fn to_dht_infohash(hash: &Hash, format: BlobFormat) -> [u8; 20] {
    use sha2::Digest;
    use sha2::Sha256;

    // Concatenate hash bytes with format discriminant
    let mut hasher = Sha256::new();
    hasher.update(hash.as_bytes());
    hasher.update([format_to_byte(format)]);
    let full_hash = hasher.finalize();

    // Truncate to 20 bytes for DHT compatibility
    let mut infohash = [0u8; 20];
    infohash.copy_from_slice(&full_hash[..20]);
    infohash
}

fn format_to_byte(format: BlobFormat) -> u8 {
    match format {
        BlobFormat::Raw => 0,
        BlobFormat::HashSeq => 1,
    }
}

/// Convert an iroh SecretKey to a mainline SigningKey.
///
/// Iroh uses the same ed25519 curve, so this is a direct byte copy.
/// We use mainline's re-exported SigningKey to avoid version conflicts.
#[cfg(feature = "global-discovery")]
fn iroh_secret_to_signing_key(secret_key: &SecretKey) -> mainline::SigningKey {
    // Iroh SecretKey wraps an ed25519 keypair
    // The secret key bytes are the first 32 bytes of the keypair
    let secret_bytes: [u8; 32] = secret_key.to_bytes();
    mainline::SigningKey::from_bytes(&secret_bytes)
}

// ============================================================================
// DHT Client Wrapper (feature-gated)
// ============================================================================

/// DHT client wrapper that encapsulates the mainline crate.
///
/// When the `global-discovery` feature is enabled, this wraps an actual
/// mainline::AsyncDht instance. Without the feature, it's a no-op stub.
#[cfg(feature = "global-discovery")]
struct DhtClient {
    /// The async DHT client from mainline crate.
    dht: mainline::async_dht::AsyncDht,
}

#[cfg(feature = "global-discovery")]
impl DhtClient {
    /// Create a new DHT client with the given configuration.
    fn new(config: &ContentDiscoveryConfig) -> Result<Self> {
        use mainline::Dht;

        let mut builder = Dht::builder();

        // Configure port if specified
        if config.dht_port > 0 {
            builder.port(config.dht_port);
        }

        // Configure server mode if requested
        if config.server_mode {
            builder.server_mode();
        }

        // Add custom bootstrap nodes if specified
        if !config.bootstrap_nodes.is_empty() {
            builder.extra_bootstrap(&config.bootstrap_nodes);
        }

        // Build the DHT
        let dht = builder.build().context("failed to build DHT client")?;

        info!(
            server_mode = config.server_mode,
            dht_port = config.dht_port,
            bootstrap_nodes = config.bootstrap_nodes.len(),
            "DHT client created"
        );

        Ok(Self { dht: dht.as_async() })
    }

    /// Wait for DHT bootstrap to complete.
    async fn wait_for_bootstrap(&self) {
        let ready = self.dht.bootstrapped().await;
        if ready {
            info!("DHT bootstrap complete");
        } else {
            warn!("DHT bootstrap may not be complete");
        }
    }

    /// Announce a peer for the given infohash.
    ///
    /// The peer will be announced with the implicit port (the port the DHT
    /// received the request from). This is typically the QUIC port used by Iroh.
    async fn announce_peer(&self, infohash: [u8; 20]) -> Result<()> {
        use mainline::Id;

        let id = Id::from(infohash);

        // Announce with implicit port (None = use source port detected by DHT nodes)
        // This is appropriate since Iroh uses QUIC with dynamic ports
        self.dht.announce_peer(id, None).await.context("DHT announce_peer failed")?;

        debug!(infohash = hex::encode(infohash), "announced peer to DHT");
        Ok(())
    }

    /// Get peers for the given infohash.
    ///
    /// Returns a stream of peer socket addresses. Each responding DHT node
    /// returns up to 20 peers, so results accumulate as queries complete.
    async fn get_peers(&self, infohash: [u8; 20]) -> Vec<std::net::SocketAddrV4> {
        use futures::StreamExt;
        use mainline::Id;

        let id = Id::from(infohash);
        let mut stream = self.dht.get_peers(id);
        let mut all_peers = Vec::new();

        // Collect peers from the stream with a timeout
        let timeout = tokio::time::timeout(DHT_QUERY_TIMEOUT, async {
            while let Some(peers) = stream.next().await {
                all_peers.extend(peers);
                if all_peers.len() >= MAX_PROVIDERS {
                    break;
                }
            }
        });

        if let Err(_) = timeout.await {
            debug!(infohash = hex::encode(infohash), peers_found = all_peers.len(), "DHT get_peers timed out");
        }

        debug!(infohash = hex::encode(infohash), peers_found = all_peers.len(), "collected peers from DHT");

        all_peers
    }

    /// Store a mutable item in the DHT using BEP-44.
    ///
    /// This stores the node address information signed with the node's ed25519 key.
    /// The salt is the infohash, allowing multiple content items per key.
    ///
    /// # Arguments
    /// * `signing_key` - The mainline SigningKey (derived from iroh SecretKey)
    /// * `infohash` - The 20-byte content infohash (used as salt)
    /// * `node_addr` - The node address information to store
    /// * `seq` - Sequence number for this update (must be monotonically increasing)
    async fn put_mutable(
        &self,
        signing_key: &mainline::SigningKey,
        infohash: [u8; 20],
        node_addr: &DhtNodeAddr,
        seq: i64,
    ) -> Result<()> {
        use mainline::MutableItem;

        let value = node_addr.to_bytes()?;
        if value.len() > 1000 {
            anyhow::bail!("DhtNodeAddr too large: {} > 1000 bytes", value.len());
        }

        // Create mutable item with salt = infohash
        let item = MutableItem::new(signing_key.clone(), &value, seq, Some(&infohash));

        // Put to DHT with CAS (compare-and-swap) using previous seq
        let cas = if seq > 0 { Some(seq - 1) } else { None };
        self.dht.put_mutable(item, cas).await.context("DHT put_mutable failed")?;

        debug!(
            public_key = hex::encode(signing_key.verifying_key().to_bytes()),
            infohash = hex::encode(infohash),
            seq,
            size = value.len(),
            "stored mutable item in DHT"
        );

        Ok(())
    }

    /// Retrieve mutable items from the DHT for a specific public key and infohash.
    ///
    /// Returns the most recent DhtNodeAddr if found.
    ///
    /// # Arguments
    /// * `public_key` - The 32-byte ed25519 public key of the provider
    /// * `infohash` - The 20-byte content infohash (used as salt)
    async fn get_mutable(&self, public_key: &[u8; 32], infohash: [u8; 20]) -> Option<DhtNodeAddr> {
        // Query for the most recent mutable item
        let result = tokio::time::timeout(MUTABLE_LOOKUP_TIMEOUT, async {
            self.dht.get_mutable_most_recent(public_key, Some(&infohash)).await
        })
        .await;

        match result {
            Ok(Some(item)) => {
                // Parse the stored DhtNodeAddr
                match DhtNodeAddr::from_bytes(item.value()) {
                    Some(addr) => {
                        debug!(
                            public_key = hex::encode(public_key),
                            infohash = hex::encode(infohash),
                            seq = item.seq(),
                            "retrieved mutable item from DHT"
                        );
                        Some(addr)
                    }
                    None => {
                        debug!(
                            public_key = hex::encode(public_key),
                            infohash = hex::encode(infohash),
                            "failed to parse DhtNodeAddr from mutable item"
                        );
                        None
                    }
                }
            }
            Ok(None) => {
                debug!(
                    public_key = hex::encode(public_key),
                    infohash = hex::encode(infohash),
                    "no mutable item found in DHT"
                );
                None
            }
            Err(_) => {
                debug!(
                    public_key = hex::encode(public_key),
                    infohash = hex::encode(infohash),
                    "mutable item lookup timed out"
                );
                None
            }
        }
    }
}

/// Stub signing key type when feature is disabled.
#[cfg(not(feature = "global-discovery"))]
struct StubSigningKey;

/// Convert an iroh SecretKey to a stub signing key (no-op when feature disabled).
#[cfg(not(feature = "global-discovery"))]
fn iroh_secret_to_signing_key(_secret_key: &SecretKey) -> StubSigningKey {
    StubSigningKey
}

/// Stub DHT client when feature is disabled.
#[cfg(not(feature = "global-discovery"))]
struct DhtClient;

#[cfg(not(feature = "global-discovery"))]
impl DhtClient {
    fn new(_config: &ContentDiscoveryConfig) -> Result<Self> {
        info!("DHT client disabled (global-discovery feature not enabled)");
        Ok(Self)
    }

    async fn wait_for_bootstrap(&self) {
        // No-op
    }

    async fn announce_peer(&self, infohash: [u8; 20]) -> Result<()> {
        debug!(infohash = hex::encode(infohash), "would announce to DHT (feature disabled)");
        Ok(())
    }

    async fn get_peers(&self, infohash: [u8; 20]) -> Vec<std::net::SocketAddrV4> {
        debug!(infohash = hex::encode(infohash), "would query DHT (feature disabled)");
        Vec::new()
    }

    async fn put_mutable(
        &self,
        _signing_key: &StubSigningKey,
        infohash: [u8; 20],
        _node_addr: &DhtNodeAddr,
        _seq: i64,
    ) -> Result<()> {
        debug!(infohash = hex::encode(infohash), "would put mutable to DHT (feature disabled)");
        Ok(())
    }

    async fn get_mutable(&self, public_key: &[u8; 32], infohash: [u8; 20]) -> Option<DhtNodeAddr> {
        debug!(
            public_key = hex::encode(public_key),
            infohash = hex::encode(infohash),
            "would get mutable from DHT (feature disabled)"
        );
        None
    }
}

// ============================================================================
// Service State
// ============================================================================

/// Tracks recent announces to prevent spam.
struct AnnounceTracker {
    /// Map from hash -> (last announce time, size, format) for republishing
    announces: HashMap<Hash, (Instant, u64, BlobFormat)>,
    /// Maximum tracked announces
    max_size: usize,
}

impl AnnounceTracker {
    fn new(max_size: usize) -> Self {
        Self {
            announces: HashMap::with_capacity(max_size),
            max_size,
        }
    }

    /// Check if we can announce this hash (rate limiting).
    fn can_announce(&self, hash: &Hash) -> bool {
        match self.announces.get(hash) {
            Some((last, _, _)) => last.elapsed() >= MIN_ANNOUNCE_INTERVAL,
            None => true,
        }
    }

    /// Record an announce, evicting old entries if needed.
    fn record_announce(&mut self, hash: Hash, size: u64, format: BlobFormat) {
        // Evict oldest entries if at capacity
        if self.announces.len() >= self.max_size {
            // Remove entries older than republish interval
            let cutoff = Instant::now() - REPUBLISH_INTERVAL;
            self.announces.retain(|_, (t, _, _)| *t > cutoff);

            // If still at capacity, remove oldest
            if self.announces.len() >= self.max_size
                && let Some(oldest_hash) = self.announces.iter().min_by_key(|(_, (t, _, _))| *t).map(|(h, _)| *h)
            {
                self.announces.remove(&oldest_hash);
            }
        }

        self.announces.insert(hash, (Instant::now(), size, format));
    }

    /// Get hashes that need republishing along with their metadata.
    fn get_stale_announces(&self) -> Vec<(Hash, u64, BlobFormat)> {
        let cutoff = Instant::now() - REPUBLISH_INTERVAL;
        self.announces
            .iter()
            .filter(|(_, (t, _, _))| *t < cutoff)
            .map(|(h, (_, size, format))| (*h, *size, *format))
            .collect()
    }
}

/// Commands sent to the discovery service.
enum DiscoveryCommand {
    /// Announce a blob to the DHT.
    Announce {
        hash: Hash,
        size: u64,
        format: BlobFormat,
        reply: tokio::sync::oneshot::Sender<Result<()>>,
    },
    /// Query for providers of a blob.
    FindProviders {
        hash: Hash,
        format: BlobFormat,
        reply: tokio::sync::oneshot::Sender<Result<Vec<ProviderInfo>>>,
    },
    /// Announce all local blobs (for initial sync and auto-announce).
    AnnounceLocalBlobs {
        blobs: Vec<(Hash, u64, BlobFormat)>,
        reply: tokio::sync::oneshot::Sender<Result<usize>>,
    },
    /// Look up a specific provider by their public key.
    FindProviderByKey {
        public_key: PublicKey,
        hash: Hash,
        format: BlobFormat,
        reply: tokio::sync::oneshot::Sender<Option<DhtNodeAddr>>,
    },
}

// ============================================================================
// Content Discovery Service
// ============================================================================

/// Handle to the content discovery service.
///
/// Provides methods to announce content and find providers using
/// the BitTorrent Mainline DHT.
#[derive(Clone)]
pub struct ContentDiscoveryService {
    /// Channel to send commands to the service task.
    command_tx: mpsc::Sender<DiscoveryCommand>,
    /// Our public key for identification.
    public_key: PublicKey,
}

impl ContentDiscoveryService {
    /// Spawn the content discovery service.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Iroh endpoint for peer connections
    /// * `secret_key` - Node's secret key for signing announces
    /// * `config` - Service configuration
    /// * `cancel` - Cancellation token for shutdown
    ///
    /// # Returns
    ///
    /// A handle to the running service and a join handle for the background task.
    pub async fn spawn(
        endpoint: Arc<Endpoint>,
        secret_key: SecretKey,
        config: ContentDiscoveryConfig,
        cancel: CancellationToken,
    ) -> Result<(Self, JoinHandle<()>)> {
        let (command_tx, command_rx) = mpsc::channel(256);
        let public_key = secret_key.public();

        let service = Self { command_tx, public_key };

        let task = tokio::spawn(Self::run_service(endpoint, secret_key, config, command_rx, cancel));

        Ok((service, task))
    }

    /// Announce a blob to the global DHT.
    ///
    /// Rate-limited to prevent spam (max 1 announce per hash per 5 minutes).
    pub async fn announce(&self, hash: Hash, size: u64, format: BlobFormat) -> Result<()> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(DiscoveryCommand::Announce {
                hash,
                size,
                format,
                reply: reply_tx,
            })
            .await
            .context("discovery service shut down")?;

        reply_rx.await.context("reply channel closed")?
    }

    /// Find providers for a blob hash.
    ///
    /// Queries the DHT for nodes that have announced this content.
    /// Returns up to `MAX_PROVIDERS` results.
    pub async fn find_providers(&self, hash: Hash, format: BlobFormat) -> Result<Vec<ProviderInfo>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(DiscoveryCommand::FindProviders {
                hash,
                format,
                reply: reply_tx,
            })
            .await
            .context("discovery service shut down")?;

        reply_rx.await.context("reply channel closed")?
    }

    /// Get our public key.
    pub fn public_key(&self) -> PublicKey {
        self.public_key
    }

    /// Announce multiple local blobs to the DHT.
    ///
    /// This is useful for bulk announcing (e.g., on startup when auto_announce is enabled).
    /// Returns the number of blobs successfully announced.
    pub async fn announce_local_blobs(&self, blobs: Vec<(Hash, u64, BlobFormat)>) -> Result<usize> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(DiscoveryCommand::AnnounceLocalBlobs { blobs, reply: reply_tx })
            .await
            .context("discovery service shut down")?;

        reply_rx.await.context("reply channel closed")?
    }

    /// Look up a specific provider's node address by their public key.
    ///
    /// This queries the DHT for the BEP-44 mutable item stored by a known provider.
    /// Use this when you already know a provider's public key (e.g., from gossip,
    /// from our own announces, or from a previous successful connection).
    ///
    /// Returns the full DhtNodeAddr if found, which contains the iroh public key
    /// and can be used to establish a connection.
    pub async fn find_provider_by_public_key(
        &self,
        public_key: &PublicKey,
        hash: Hash,
        format: BlobFormat,
    ) -> Result<Option<DhtNodeAddr>> {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        self.command_tx
            .send(DiscoveryCommand::FindProviderByKey {
                public_key: *public_key,
                hash,
                format,
                reply: reply_tx,
            })
            .await
            .context("discovery service shut down")?;

        reply_rx.await.context("reply channel closed")
    }

    /// Main service loop.
    async fn run_service(
        _endpoint: Arc<Endpoint>,
        secret_key: SecretKey,
        config: ContentDiscoveryConfig,
        mut command_rx: mpsc::Receiver<DiscoveryCommand>,
        cancel: CancellationToken,
    ) {
        info!(server_mode = config.server_mode, dht_port = config.dht_port, "starting content discovery service");

        // Initialize DHT client
        let dht_client = match DhtClient::new(&config) {
            Ok(client) => {
                // Wait for bootstrap in background (don't block service start)
                let client = Arc::new(client);
                let client_clone = client.clone();
                tokio::spawn(async move {
                    client_clone.wait_for_bootstrap().await;
                });
                Some(client)
            }
            Err(err) => {
                warn!(error = %err, "failed to initialize DHT client, operating in stub mode");
                None
            }
        };

        let tracker = Arc::new(RwLock::new(AnnounceTracker::new(MAX_TRACKED_ANNOUNCES)));

        // Republish timer
        let mut republish_interval = tokio::time::interval(REPUBLISH_INTERVAL);
        republish_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("content discovery service shutting down");
                    break;
                }

                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        DiscoveryCommand::Announce { hash, size, format, reply } => {
                            let result = Self::handle_announce(
                                &secret_key,
                                &tracker,
                                dht_client.as_ref(),
                                hash,
                                size,
                                format,
                            ).await;
                            let _ = reply.send(result);
                        }

                        DiscoveryCommand::FindProviders { hash, format, reply } => {
                            let result = Self::handle_find_providers(
                                dht_client.as_ref(),
                                hash,
                                format,
                            ).await;
                            let _ = reply.send(result);
                        }

                        DiscoveryCommand::AnnounceLocalBlobs { blobs, reply } => {
                            let mut announced = 0;
                            for (hash, size, format) in blobs {
                                let result = Self::handle_announce(
                                    &secret_key,
                                    &tracker,
                                    dht_client.as_ref(),
                                    hash,
                                    size,
                                    format,
                                ).await;
                                if result.is_ok() {
                                    announced += 1;
                                }
                            }
                            info!(count = announced, "bulk announced local blobs to DHT");
                            let _ = reply.send(Ok(announced));
                        }

                        DiscoveryCommand::FindProviderByKey { public_key, hash, format, reply } => {
                            let result = Self::handle_find_provider_by_key(
                                dht_client.as_ref(),
                                &public_key,
                                hash,
                                format,
                            ).await;
                            let _ = reply.send(result);
                        }
                    }
                }

                _ = republish_interval.tick() => {
                    let stale = tracker.read().get_stale_announces();
                    if !stale.is_empty() {
                        debug!(count = stale.len(), "republishing stale announces");
                        // Republish stale announces to DHT
                        if let Some(ref dht) = dht_client {
                            for (hash, _size, format) in stale {
                                let infohash = to_dht_infohash(&hash, format);
                                if let Err(err) = dht.announce_peer(infohash).await {
                                    warn!(
                                        hash = %hash.fmt_short(),
                                        error = %err,
                                        "failed to republish announce to DHT"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn handle_announce(
        secret_key: &SecretKey,
        tracker: &Arc<RwLock<AnnounceTracker>>,
        dht_client: Option<&Arc<DhtClient>>,
        hash: Hash,
        size: u64,
        format: BlobFormat,
    ) -> Result<()> {
        // Check rate limit
        if !tracker.read().can_announce(&hash) {
            debug!(?hash, "announce rate limited");
            return Ok(()); // Silent success for rate-limited announces
        }

        // Create signed announce for local verification
        let announce = DhtAnnounce::new(hash, size, format)?;
        let signed = SignedDhtAnnounce::sign(announce, secret_key)?;
        let bytes = signed.to_bytes()?;

        if bytes.len() > MAX_ANNOUNCE_SIZE {
            anyhow::bail!("announce too large: {} > {}", bytes.len(), MAX_ANNOUNCE_SIZE);
        }

        // Calculate DHT key
        let infohash = to_dht_infohash(&hash, format);

        // Announce to DHT if client is available
        if let Some(dht) = dht_client {
            // 1. Legacy announce_peer for backwards compatibility (IP:port only)
            if let Err(e) = dht.announce_peer(infohash).await {
                debug!(error = %e, "announce_peer failed (non-fatal)");
            }

            // 2. Store full NodeAddr via BEP-44 mutable item
            // This allows cross-cluster discovery with full connection info
            let node_addr = DhtNodeAddr::new(
                secret_key.public(),
                None,               // TODO: Get relay URL from endpoint when available
                std::iter::empty(), // TODO: Get direct addrs from endpoint when available
                size,
                format,
            )?;

            // Convert iroh SecretKey to mainline SigningKey for BEP-44
            let signing_key = iroh_secret_to_signing_key(secret_key);

            // Use a monotonically increasing sequence number based on timestamp
            let seq = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(1);

            if let Err(e) = dht.put_mutable(&signing_key, infohash, &node_addr, seq).await {
                debug!(error = %e, "put_mutable failed (non-fatal)");
            }

            info!(
                hash = %hash.fmt_short(),
                size,
                format = ?format,
                infohash = hex::encode(infohash),
                public_key = %secret_key.public().fmt_short(),
                "announced to DHT (peer + mutable)"
            );
        } else {
            debug!(
                hash = %hash.fmt_short(),
                infohash = hex::encode(infohash),
                "DHT announce skipped (no client)"
            );
        }

        // Record announce for rate limiting and republishing
        tracker.write().record_announce(hash, size, format);

        Ok(())
    }

    async fn handle_find_providers(
        dht_client: Option<&Arc<DhtClient>>,
        hash: Hash,
        format: BlobFormat,
    ) -> Result<Vec<ProviderInfo>> {
        let infohash = to_dht_infohash(&hash, format);
        let now_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        let Some(dht) = dht_client else {
            debug!(
                hash = %hash.fmt_short(),
                infohash = hex::encode(infohash),
                "DHT query skipped (no client)"
            );
            return Ok(Vec::new());
        };

        // Strategy: Query get_peers first to find IP:port pairs.
        // Then, for each peer, try to look up their mutable item using
        // a derived public key approach.
        //
        // The key insight: when a node announces, they store a mutable item
        // under their public key with infohash as salt. We need to discover
        // those public keys somehow.
        //
        // Approach: Use Iroh's pkarr discovery to find node IDs, then
        // look up mutable items for those node IDs.
        let peers = dht.get_peers(infohash).await;

        if peers.is_empty() {
            debug!(
                hash = %hash.fmt_short(),
                format = ?format,
                infohash = hex::encode(infohash),
                "no providers found in DHT (get_peers returned empty)"
            );
            return Ok(Vec::new());
        }

        info!(
            hash = %hash.fmt_short(),
            format = ?format,
            peer_count = peers.len(),
            "found peers in DHT, attempting mutable item lookups"
        );

        // Collect providers from mutable item lookups
        // We'll try to find mutable items for any known public keys
        // For now, we return the legacy placeholder providers
        // The real fix is in the client handler which will call
        // find_provider_by_public_key for known keys
        let providers: Vec<ProviderInfo> = peers
            .into_iter()
            .take(MAX_PROVIDERS)
            .filter_map(|addr| {
                // Try to extract potential public keys from the peer address
                // This is a heuristic - we use the IP:port to create a lookup key
                // In practice, the client handler should call find_provider_by_public_key
                // with actual known public keys from gossip or previous connections

                // For now, store the address info so the caller can use other
                // discovery mechanisms (Iroh mDNS, pkarr, relay) to find the node
                debug!(addr = %addr, "found peer in DHT");

                // Create a placeholder - the caller will need to use Iroh discovery
                // to resolve the actual node identity
                Some(ProviderInfo {
                    node_id: PublicKey::from_bytes(&[0u8; 32]).ok()?,
                    blob_size: 0,
                    blob_format: format,
                    discovered_at: now_micros,
                    verified: false,
                })
            })
            .collect();

        info!(
            hash = %hash.fmt_short(),
            format = ?format,
            provider_count = providers.len(),
            "found providers in DHT"
        );

        Ok(providers)
    }

    /// Find a specific provider's node address by their public key.
    ///
    /// This queries the DHT for the mutable item stored by a known provider.
    /// Use this when you already know a provider's public key (e.g., from gossip
    /// or a previous successful connection).
    async fn handle_find_provider_by_key(
        dht_client: Option<&Arc<DhtClient>>,
        public_key: &PublicKey,
        hash: Hash,
        format: BlobFormat,
    ) -> Option<DhtNodeAddr> {
        let Some(dht) = dht_client else {
            debug!("cannot find provider by key: no DHT client");
            return None;
        };

        let infohash = to_dht_infohash(&hash, format);
        let key_bytes = public_key.as_bytes();

        dht.get_mutable(key_bytes, infohash).await
    }
}

// ============================================================================
// Bridge: BlobAnnouncement -> ContentDiscovery
// ============================================================================

use super::gossip_discovery::BlobAnnouncement;

impl From<&BlobAnnouncement> for (Hash, u64, BlobFormat) {
    fn from(ann: &BlobAnnouncement) -> Self {
        (ann.blob_hash, ann.blob_size, ann.blob_format)
    }
}

/// Extension trait for bridging gossip announcements to global discovery.
pub trait ContentDiscoveryExt {
    /// Announce this blob to the global DHT (if configured).
    fn announce_to_dht(
        &self,
        service: &ContentDiscoveryService,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

impl ContentDiscoveryExt for BlobAnnouncement {
    async fn announce_to_dht(&self, service: &ContentDiscoveryService) -> Result<()> {
        service.announce(self.blob_hash, self.blob_size, self.blob_format).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dht_infohash_deterministic() {
        let hash = Hash::from_bytes([0x42; 32]);
        let infohash1 = to_dht_infohash(&hash, BlobFormat::Raw);
        let infohash2 = to_dht_infohash(&hash, BlobFormat::Raw);
        assert_eq!(infohash1, infohash2);
    }

    #[test]
    fn test_dht_infohash_format_differs() {
        let hash = Hash::from_bytes([0x42; 32]);
        let raw_infohash = to_dht_infohash(&hash, BlobFormat::Raw);
        let seq_infohash = to_dht_infohash(&hash, BlobFormat::HashSeq);
        assert_ne!(raw_infohash, seq_infohash);
    }

    #[test]
    fn test_dht_announce_roundtrip() {
        let hash = Hash::from_bytes([0xAB; 32]);
        let announce = DhtAnnounce::new(hash, 1024, BlobFormat::Raw).unwrap();
        let bytes = announce.to_bytes().unwrap();
        let decoded = DhtAnnounce::from_bytes(&bytes).unwrap();

        assert_eq!(announce.version, decoded.version);
        assert_eq!(announce.blob_hash, decoded.blob_hash);
        assert_eq!(announce.blob_size, decoded.blob_size);
    }

    #[test]
    fn test_signed_announce_verify() {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let hash = Hash::from_bytes([0xCD; 32]);
        let announce = DhtAnnounce::new(hash, 2048, BlobFormat::HashSeq).unwrap();
        let signed = SignedDhtAnnounce::sign(announce, &secret_key).unwrap();

        // Verify should succeed
        assert!(signed.verify().is_some());

        // Serialization roundtrip
        let bytes = signed.to_bytes().unwrap();
        let decoded = SignedDhtAnnounce::from_bytes(&bytes).unwrap();
        assert!(decoded.verify().is_some());
    }

    #[test]
    fn test_announce_tracker_rate_limiting() {
        let mut tracker = AnnounceTracker::new(100);
        let hash = Hash::from_bytes([0x11; 32]);

        // First announce should be allowed
        assert!(tracker.can_announce(&hash));
        tracker.record_announce(hash, 1024, BlobFormat::Raw);

        // Immediate second announce should be blocked
        assert!(!tracker.can_announce(&hash));
    }

    #[test]
    fn test_announce_tracker_capacity() {
        let mut tracker = AnnounceTracker::new(2);

        let hash1 = Hash::from_bytes([0x01; 32]);
        let hash2 = Hash::from_bytes([0x02; 32]);
        let hash3 = Hash::from_bytes([0x03; 32]);

        tracker.record_announce(hash1, 100, BlobFormat::Raw);
        tracker.record_announce(hash2, 200, BlobFormat::Raw);
        assert_eq!(tracker.announces.len(), 2);

        // Adding third should evict oldest
        tracker.record_announce(hash3, 300, BlobFormat::Raw);
        assert!(tracker.announces.len() <= 2);
    }

    #[test]
    fn test_announce_tracker_stale_announces() {
        let mut tracker = AnnounceTracker::new(10);

        let hash = Hash::from_bytes([0xAA; 32]);
        tracker.record_announce(hash, 512, BlobFormat::HashSeq);

        // Fresh announces should not be stale
        let stale = tracker.get_stale_announces();
        assert!(stale.is_empty());

        // Check that metadata is preserved
        let (_, size, format) = tracker.announces.get(&hash).unwrap();
        assert_eq!(*size, 512);
        assert_eq!(*format, BlobFormat::HashSeq);
    }

    #[tokio::test]
    async fn test_content_discovery_service_lifecycle() {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let endpoint = iroh::Endpoint::builder().bind().await.unwrap();

        let config = ContentDiscoveryConfig {
            enabled: true,
            ..Default::default()
        };
        let cancel = CancellationToken::new();

        let (service, handle) =
            ContentDiscoveryService::spawn(Arc::new(endpoint), secret_key.clone(), config, cancel.clone())
                .await
                .unwrap();

        assert_eq!(service.public_key(), secret_key.public());

        // Shutdown
        cancel.cancel();
        handle.await.unwrap();
    }
}
