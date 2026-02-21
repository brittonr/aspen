//! DHT client wrapper that encapsulates the mainline crate.

use anyhow::Context;
use anyhow::Result;
use tracing::debug;
use tracing::info;
use tracing::warn;

use crate::config::ContentDiscoveryConfig;
use crate::types::DhtNodeAddr;

/// Maximum providers to return from a query.
pub(crate) const MAX_PROVIDERS: usize = 50;

/// Timeout for DHT queries (get_peers).
#[cfg(feature = "global-discovery")]
const DHT_QUERY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Timeout for BEP-44 mutable item lookups.
#[cfg(feature = "global-discovery")]
const MUTABLE_LOOKUP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

// ============================================================================
// Feature-enabled DHT client
// ============================================================================

/// DHT client wrapper that encapsulates the mainline crate.
///
/// When the `global-discovery` feature is enabled, this wraps an actual
/// mainline::AsyncDht instance. Without the feature, it's a no-op stub.
#[cfg(feature = "global-discovery")]
pub(crate) struct DhtClient {
    /// The async DHT client from mainline crate.
    dht: mainline::async_dht::AsyncDht,
}

#[cfg(feature = "global-discovery")]
impl DhtClient {
    /// Create a new DHT client with the given configuration.
    pub(crate) fn new(config: &ContentDiscoveryConfig) -> Result<Self> {
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
    pub(crate) async fn wait_for_bootstrap(&self) {
        let ready = self.dht.bootstrapped().await;
        if ready {
            info!("DHT bootstrap complete");
        } else {
            warn!("DHT bootstrap may not be complete");
        }
    }

    /// Announce a peer for the given infohash.
    pub(crate) async fn announce_peer(&self, infohash: [u8; 20]) -> Result<()> {
        use mainline::Id;

        let id = Id::from(infohash);

        self.dht.announce_peer(id, None).await.context("DHT announce_peer failed")?;

        debug!(infohash = hex::encode(infohash), "announced peer to DHT");
        Ok(())
    }

    /// Get peers for the given infohash.
    pub(crate) async fn get_peers(&self, infohash: [u8; 20]) -> Vec<std::net::SocketAddrV4> {
        use mainline::Id;
        use n0_future::StreamExt;

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
    pub(crate) async fn put_mutable(
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
    pub(crate) async fn get_mutable(&self, public_key: &[u8; 32], infohash: [u8; 20]) -> Option<DhtNodeAddr> {
        let result = tokio::time::timeout(MUTABLE_LOOKUP_TIMEOUT, async {
            self.dht.get_mutable_most_recent(public_key, Some(&infohash)).await
        })
        .await;

        match result {
            Ok(Some(item)) => match DhtNodeAddr::from_bytes(item.value()) {
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
            },
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

// ============================================================================
// Feature-disabled stub DHT client
// ============================================================================

/// Stub DHT client when feature is disabled.
#[cfg(not(feature = "global-discovery"))]
pub(crate) struct DhtClient;

#[cfg(not(feature = "global-discovery"))]
impl DhtClient {
    pub(crate) fn new(_config: &ContentDiscoveryConfig) -> Result<Self> {
        info!("DHT client disabled (global-discovery feature not enabled)");
        Ok(Self)
    }

    pub(crate) async fn wait_for_bootstrap(&self) {
        // No-op
    }

    pub(crate) async fn announce_peer(&self, infohash: [u8; 20]) -> Result<()> {
        debug!(infohash = hex::encode(infohash), "would announce to DHT (feature disabled)");
        Ok(())
    }

    pub(crate) async fn get_peers(&self, infohash: [u8; 20]) -> Vec<std::net::SocketAddrV4> {
        debug!(infohash = hex::encode(infohash), "would query DHT (feature disabled)");
        Vec::new()
    }

    #[cfg(not(feature = "global-discovery"))]
    pub(crate) async fn put_mutable(
        &self,
        _signing_key: &crate::hash::StubSigningKey,
        infohash: [u8; 20],
        _node_addr: &DhtNodeAddr,
        _seq: i64,
    ) -> Result<()> {
        debug!(infohash = hex::encode(infohash), "would put mutable to DHT (feature disabled)");
        Ok(())
    }

    pub(crate) async fn get_mutable(&self, public_key: &[u8; 32], infohash: [u8; 20]) -> Option<DhtNodeAddr> {
        debug!(
            public_key = hex::encode(public_key),
            infohash = hex::encode(infohash),
            "would get mutable from DHT (feature disabled)"
        );
        None
    }
}
