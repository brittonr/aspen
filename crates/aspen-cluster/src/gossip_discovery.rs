//! Gossip-based peer discovery for Aspen clusters.
//!
//! This module provides integration between the gossip peer discovery
//! functionality and the Aspen cluster layer.

use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use aspen_core::DiscoveredPeer;
use aspen_core::PeerDiscoveredCallback;
use aspen_core::PeerDiscovery;
use aspen_raft::types::NodeId;
use iroh_gossip::proto::TopicId;

use super::IrohEndpointManager;
// Use the type alias from cluster mod.rs which provides the concrete type
use super::IrpcRaftNetworkFactory;
// Re-export the main gossip discovery types from the internal gossip module
pub use crate::gossip::{BlobAnnouncement, BlobAnnouncementParams, GossipPeerDiscovery, broadcast_blob_announcement};

/// Compatibility wrapper: spawn gossip peer discovery tasks using IrohEndpointManager.
///
/// Subscribes to the gossip topic and starts background tasks for
/// announcing this node's ID/address and receiving peer announcements.
///
/// If `network_factory` is provided, discovered peers are automatically
/// added to it for Raft networking.
///
/// This is a compatibility function that adapts the new standalone GossipPeerDiscovery
/// to work with the existing IrohEndpointManager interface.
pub async fn spawn_gossip_peer_discovery(
    topic_id: TopicId,
    node_id: NodeId,
    iroh_manager: &IrohEndpointManager,
    network_factory: Option<Arc<IrpcRaftNetworkFactory>>,
) -> Result<GossipPeerDiscovery> {
    let gossip = iroh_manager.gossip().context("gossip not enabled on IrohEndpointManager")?;

    let discovery = GossipPeerDiscovery::new(
        topic_id,
        node_id,
        Arc::clone(gossip),
        iroh_manager.node_addr().clone(),
        iroh_manager.secret_key().clone(),
    );

    // Convert network factory to callback if provided
    let callback = network_factory.map(|factory| {
        let callback: PeerDiscoveredCallback<iroh::EndpointAddr> =
            Box::new(move |peer: DiscoveredPeer<iroh::EndpointAddr>| {
                let factory = Arc::clone(&factory);
                Box::pin(async move {
                    factory.add_peer(peer.node_id, peer.address).await;
                    tracing::info!("added peer to network factory via callback: node_id={}", peer.node_id);
                })
            });
        callback
    });

    // Start the discovery tasks using the PeerDiscovery trait
    let _handle = discovery.start(callback).await?;

    Ok(discovery)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use iroh::SecretKey;

    use super::*;

    /// Create a deterministic secret key from a seed for reproducible tests.
    fn secret_key_from_seed(seed: u64) -> SecretKey {
        let mut key_bytes = [0u8; 32];
        key_bytes[0..8].copy_from_slice(&seed.to_le_bytes());
        SecretKey::from_bytes(&key_bytes)
    }

    /// Create a mock EndpointAddr from a secret key.
    fn endpoint_addr_from_secret_key(secret_key: &SecretKey) -> iroh::EndpointAddr {
        iroh::EndpointAddr::new(secret_key.public())
    }

    // =========================================================================
    // Re-export Tests
    // =========================================================================

    #[test]
    fn test_reexports_available() {
        // Verify re-exports from the gossip module are accessible
        // This is a compile-time test - if it compiles, the re-exports work
        let _: fn(
            NodeId,
            iroh::EndpointAddr,
            iroh_blobs::Hash,
            u64,
            iroh_blobs::BlobFormat,
            Option<String>,
        ) -> anyhow::Result<BlobAnnouncement> = BlobAnnouncement::new;

        // BlobAnnouncementParams struct should be accessible
        let secret_key = secret_key_from_seed(1);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let _params = BlobAnnouncementParams {
            node_id: NodeId::from(1u64),
            endpoint_addr,
            blob_hash: iroh_blobs::Hash::from_bytes([0u8; 32]),
            blob_size: 0,
            blob_format: iroh_blobs::BlobFormat::Raw,
            tag: None,
        };
    }

    // =========================================================================
    // TopicId Tests
    // =========================================================================

    #[test]
    fn test_topic_id_deterministic_from_bytes() {
        let bytes = [0x42u8; 32];
        let topic1 = TopicId::from_bytes(bytes);
        let topic2 = TopicId::from_bytes(bytes);

        assert_eq!(topic1, topic2);
    }

    #[test]
    fn test_topic_id_different_bytes_different_ids() {
        let topic1 = TopicId::from_bytes([0x01u8; 32]);
        let topic2 = TopicId::from_bytes([0x02u8; 32]);

        assert_ne!(topic1, topic2);
    }

    // =========================================================================
    // Callback Construction Tests
    // =========================================================================

    #[tokio::test]
    async fn test_peer_discovered_callback_type() {
        // Verify the callback type signature matches what's expected
        let invoked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let invoked_clone = invoked.clone();

        let callback: PeerDiscoveredCallback<iroh::EndpointAddr> =
            Box::new(move |peer: DiscoveredPeer<iroh::EndpointAddr>| {
                let invoked = invoked_clone.clone();
                Box::pin(async move {
                    invoked.store(true, std::sync::atomic::Ordering::SeqCst);
                    // Verify peer fields are accessible
                    let _ = peer.node_id;
                    let _ = peer.address;
                    let _ = peer.timestamp_micros;
                })
            });

        // Create a mock peer
        let secret_key = secret_key_from_seed(10);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let peer = DiscoveredPeer {
            node_id: NodeId::from(100u64),
            address: endpoint_addr,
            timestamp_micros: 1234567890,
        };

        // Invoke the callback
        callback(peer).await;

        assert!(invoked.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_discovered_peer_fields() {
        let secret_key = secret_key_from_seed(20);
        let endpoint_addr = endpoint_addr_from_secret_key(&secret_key);
        let node_id = NodeId::from(200u64);
        let timestamp = 9876543210u64;

        let peer = DiscoveredPeer {
            node_id,
            address: endpoint_addr.clone(),
            timestamp_micros: timestamp,
        };

        assert_eq!(peer.node_id, node_id);
        assert_eq!(peer.address.id, endpoint_addr.id);
        assert_eq!(peer.timestamp_micros, timestamp);
    }

    // =========================================================================
    // NodeId Tests
    // =========================================================================

    #[test]
    fn test_node_id_from_u64() {
        let id = NodeId::from(42u64);
        assert_eq!(u64::from(id), 42);
    }

    #[test]
    fn test_node_id_edge_values() {
        let min_id = NodeId::from(0u64);
        let max_id = NodeId::from(u64::MAX);

        assert_eq!(u64::from(min_id), 0);
        assert_eq!(u64::from(max_id), u64::MAX);
    }

    // =========================================================================
    // GossipPeerDiscovery Re-export Tests
    // =========================================================================

    #[test]
    fn test_gossip_peer_discovery_is_reexported() {
        // Verify GossipPeerDiscovery is accessible via the re-export
        // This is primarily a compile-time test
        use aspen_core::PeerDiscovery as _;

        // GossipPeerDiscovery implements PeerDiscovery trait
        fn assert_peer_discovery<T: PeerDiscovery>() {}
        assert_peer_discovery::<GossipPeerDiscovery>();
    }
}
