//! Gossip-based peer discovery for Aspen clusters.
//!
//! This module re-exports the gossip peer discovery functionality from the
//! aspen-gossip crate and provides integration with the Aspen cluster layer.

use std::sync::Arc;

use anyhow::{Context, Result};
use iroh_gossip::proto::TopicId;

use super::IrohEndpointManager;
// Use the type alias from cluster mod.rs which provides the concrete type
use super::IrpcRaftNetworkFactory;
use aspen_core::{DiscoveredPeer, PeerDiscoveredCallback, PeerDiscovery};
use aspen_raft::types::NodeId;

// Re-export the main gossip discovery types
pub use aspen_gossip::{broadcast_blob_announcement, BlobAnnouncementParams, BlobAnnouncement, GossipPeerDiscovery};

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
    let gossip = iroh_manager
        .gossip()
        .context("gossip not enabled on IrohEndpointManager")?;

    let discovery = GossipPeerDiscovery::new(
        topic_id,
        node_id,
        Arc::clone(gossip),
        iroh_manager.node_addr().clone(),
        iroh_manager.secret_key().clone(),
    );

    // Convert network factory to callback if provided
    let callback = network_factory.map(|factory| {
        let callback: PeerDiscoveredCallback<iroh::EndpointAddr> = Box::new(move |peer: DiscoveredPeer<iroh::EndpointAddr>| {
            let factory = Arc::clone(&factory);
            Box::pin(async move {
                factory.add_peer(peer.node_id, peer.address).await;
                tracing::info!(
                    "added peer to network factory via callback: node_id={}",
                    peer.node_id
                );
            })
        });
        callback
    });

    // Start the discovery tasks using the PeerDiscovery trait
    let _handle = discovery.start(callback).await?;

    Ok(discovery)
}