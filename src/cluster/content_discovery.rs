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
//! - Uses `announce_peer` for lightweight peer announcement (IP:port only)
//! - Uses `get_peers` to discover nodes that have announced a specific infohash
//! - Rate-limited publishing to avoid DHT spam (1 announce per hash per 5 min)
//! - Background republishing every 30 minutes for active content
//!
//! # Security
//!
//! - All local announces are Ed25519 signed for local verification
//! - DHT operations use standard BitTorrent peer announcement protocol
//! - Provider verification can be done by attempting blob download via Iroh
//!
//! # References
//!
//! - [Iroh Content Discovery Blog](https://www.iroh.computer/blog/iroh-content-discovery)
//! - [mainline crate](https://crates.io/crates/mainline)
//! - [BEP-0005: DHT Protocol](http://bittorrent.org/beps/bep_0005.html)
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
            dht.announce_peer(infohash).await?;
            info!(
                hash = %hash.fmt_short(),
                size,
                format = ?format,
                infohash = hex::encode(infohash),
                "announced to DHT"
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

        // Query DHT if client is available
        let peers = if let Some(dht) = dht_client {
            dht.get_peers(infohash).await
        } else {
            debug!(
                hash = %hash.fmt_short(),
                infohash = hex::encode(infohash),
                "DHT query skipped (no client)"
            );
            Vec::new()
        };

        if peers.is_empty() {
            debug!(
                hash = %hash.fmt_short(),
                format = ?format,
                infohash = hex::encode(infohash),
                "no providers found in DHT"
            );
            return Ok(Vec::new());
        }

        // Convert socket addresses to ProviderInfo
        // Note: DHT only returns IP:port, not full provider metadata.
        // The caller will need to connect to verify the provider has the content.
        let now_micros = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as u64)
            .unwrap_or(0);

        let providers: Vec<ProviderInfo> = peers
            .into_iter()
            .take(MAX_PROVIDERS)
            .map(|_addr| {
                // We only have IP:port from DHT, not the Iroh PublicKey
                // Create a placeholder PublicKey (all zeros) to indicate unknown
                // The caller should use Iroh discovery to resolve the actual node
                ProviderInfo {
                    node_id: PublicKey::from_bytes(&[0u8; 32]).unwrap_or_else(|_| {
                        // Fallback: generate a random public key as placeholder
                        iroh::SecretKey::generate(&mut rand::rng()).public()
                    }),
                    blob_size: 0, // Unknown from DHT
                    blob_format: format,
                    discovered_at: now_micros,
                    verified: false,
                }
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
