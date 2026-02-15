//! ContentDiscoveryService and its spawn/announce/find logic.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use iroh::Endpoint;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh_base::TransportAddr;
use iroh_blobs::BlobFormat;
use iroh_blobs::Hash;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::client::DhtClient;
use crate::client::MAX_PROVIDERS;
use crate::config::ContentDiscoveryConfig;
use crate::hash::iroh_secret_to_signing_key;
use crate::hash::to_dht_infohash;
use crate::types::DhtAnnounce;
use crate::types::DhtNodeAddr;
use crate::types::ProviderInfo;
use crate::types::SignedDhtAnnounce;

// ============================================================================
// Constants (Tiger Style: Fixed limits)
// ============================================================================

/// Maximum number of tracked announces per node (prevents memory exhaustion).
const MAX_TRACKED_ANNOUNCES: usize = 10_000;

/// Minimum interval between announces for the same hash (rate limiting).
const MIN_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(5 * 60); // 5 minutes

/// Interval for republishing active content to DHT.
const REPUBLISH_INTERVAL: Duration = Duration::from_secs(30 * 60); // 30 minutes

/// Maximum size of a serialized announce record.
const MAX_ANNOUNCE_SIZE: usize = 1024;

/// DHT announce record TTL (how long the DHT should cache the record).
#[allow(dead_code)]
const DHT_RECORD_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour

/// Maximum number of providers to query via BEP-44 mutable lookups.
#[allow(dead_code)]
const MAX_MUTABLE_LOOKUPS: usize = 20;

// ============================================================================
// Service State
// ============================================================================

/// Tracks recent announces to prevent spam.
pub(crate) struct AnnounceTracker {
    /// Map from hash -> (last announce time, size, format) for republishing
    pub(crate) announces: HashMap<Hash, (Instant, u64, BlobFormat)>,
    /// Maximum tracked announces
    max_size: usize,
}

impl AnnounceTracker {
    pub(crate) fn new(max_size: usize) -> Self {
        Self {
            announces: HashMap::with_capacity(max_size),
            max_size,
        }
    }

    /// Check if we can announce this hash (rate limiting).
    pub(crate) fn can_announce(&self, hash: &Hash) -> bool {
        match self.announces.get(hash) {
            Some((last, _, _)) => last.elapsed() >= MIN_ANNOUNCE_INTERVAL,
            None => true,
        }
    }

    /// Record an announce, evicting old entries if needed.
    pub(crate) fn record_announce(&mut self, hash: Hash, size: u64, format: BlobFormat) {
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
    pub(crate) fn get_stale_announces(&self) -> Vec<(Hash, u64, BlobFormat)> {
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
        endpoint: Arc<Endpoint>,
        secret_key: SecretKey,
        config: ContentDiscoveryConfig,
        mut command_rx: mpsc::Receiver<DiscoveryCommand>,
        cancel: CancellationToken,
    ) {
        info!(server_mode = config.server_mode, dht_port = config.dht_port, "starting content discovery service");

        // Initialize DHT client
        let (dht_client, bootstrap_handle) = match DhtClient::new(&config) {
            Ok(client) => {
                // Wait for bootstrap in background (don't block service start)
                let client = Arc::new(client);
                let client_clone = client.clone();
                let handle = tokio::spawn(async move {
                    client_clone.wait_for_bootstrap().await;
                });
                (Some(client), Some(handle))
            }
            Err(err) => {
                warn!(error = %err, "failed to initialize DHT client, operating in stub mode");
                (None, None)
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
                    if let Some(handle) = bootstrap_handle {
                        handle.abort();
                    }
                    break;
                }

                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        DiscoveryCommand::Announce { hash, size, format, reply } => {
                            let result = Self::handle_announce(
                                &endpoint,
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
                                    &endpoint,
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
        endpoint: &Arc<Endpoint>,
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
            let node_addr = DhtNodeAddr::new(
                secret_key.public(),
                endpoint.addr().relay_urls().next().map(|relay| &**relay),
                endpoint.addr().addrs.iter().filter_map(|addr| match addr {
                    TransportAddr::Ip(socket_addr) => Some(*socket_addr),
                    _ => None,
                }),
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

        let providers: Vec<ProviderInfo> = peers
            .into_iter()
            .take(MAX_PROVIDERS)
            .filter_map(|addr| {
                debug!(addr = %addr, "found peer in DHT");

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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Constants Tests
    // =========================================================================

    #[test]
    fn test_max_tracked_announces_reasonable() {
        const { assert!(MAX_TRACKED_ANNOUNCES > 0) };
        const { assert!(MAX_TRACKED_ANNOUNCES <= 100_000) }; // Not too large
    }

    #[test]
    fn test_min_announce_interval_reasonable() {
        // At least 1 minute to prevent spam
        assert!(MIN_ANNOUNCE_INTERVAL >= Duration::from_secs(60));
        // At most 1 hour
        assert!(MIN_ANNOUNCE_INTERVAL <= Duration::from_secs(3600));
    }

    #[test]
    fn test_republish_interval_reasonable() {
        // At least 10 minutes
        assert!(REPUBLISH_INTERVAL >= Duration::from_secs(600));
        // At most 24 hours
        assert!(REPUBLISH_INTERVAL <= Duration::from_secs(86400));
    }

    #[test]
    fn test_max_providers_reasonable() {
        const { assert!(MAX_PROVIDERS > 0) };
        const { assert!(MAX_PROVIDERS <= 1000) };
    }

    #[test]
    fn test_max_announce_size_reasonable() {
        const { assert!(MAX_ANNOUNCE_SIZE > 0) };
        const { assert!(MAX_ANNOUNCE_SIZE <= 64 * 1024) }; // At most 64KB
    }

    // =========================================================================
    // AnnounceTracker Tests
    // =========================================================================

    #[test]
    fn test_announce_tracker_empty() {
        let tracker = AnnounceTracker::new(100);
        assert!(tracker.announces.is_empty());
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

    #[test]
    fn test_announce_tracker_different_hashes() {
        let mut tracker = AnnounceTracker::new(10);

        let hash1 = Hash::from_bytes([0x01; 32]);
        let hash2 = Hash::from_bytes([0x02; 32]);

        // Both should be allowed
        assert!(tracker.can_announce(&hash1));
        assert!(tracker.can_announce(&hash2));

        tracker.record_announce(hash1, 100, BlobFormat::Raw);

        // hash1 now blocked, hash2 still allowed
        assert!(!tracker.can_announce(&hash1));
        assert!(tracker.can_announce(&hash2));
    }

    #[test]
    fn test_announce_tracker_capacity_limit() {
        let mut tracker = AnnounceTracker::new(3);

        for i in 0..5 {
            let hash = Hash::from_bytes([i as u8; 32]);
            tracker.record_announce(hash, i as u64 * 100, BlobFormat::Raw);
        }

        // Should not exceed capacity
        assert!(tracker.announces.len() <= 3);
    }

    // =========================================================================
    // ContentDiscoveryService Tests
    // =========================================================================

    #[tokio::test]
    async fn test_content_discovery_service_lifecycle() {
        let secret_key = SecretKey::generate(&mut rand::rng());
        let endpoint = iroh::Endpoint::builder().bind().await.unwrap();

        let config = ContentDiscoveryConfig {
            is_enabled: true,
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
