//! Discovery initialization for cluster nodes.
//!
//! This module handles gossip-based peer discovery and content discovery via DHT.

use std::sync::Arc;

use anyhow::Result;
use iroh_gossip::proto::TopicId;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing::warn;

use super::IrpcRaftNetworkFactory;
use crate::IrohEndpointManager;
use crate::config::NodeConfig;
use crate::gossip_discovery::GossipPeerDiscovery;
use crate::gossip_discovery::spawn_gossip_peer_discovery;
use crate::ticket::AspenClusterTicket;

/// Derive a gossip topic ID from the cluster cookie.
///
/// Uses blake3 hash of the cookie string to create a deterministic
/// 32-byte topic ID. All nodes with the same cookie will join the
/// same gossip topic.
///
/// Tiger Style: Fixed-size output (32 bytes), deterministic.
pub fn derive_topic_id_from_cookie(cookie: &str) -> TopicId {
    let hash = blake3::hash(cookie.as_bytes());
    TopicId::from_bytes(*hash.as_bytes())
}

/// Derive gossip topic ID from cluster ticket or cookie.
///
/// If a gossip ticket is configured, uses the topic ID from the ticket.
/// Otherwise, derives a deterministic topic ID from the cluster cookie.
pub(super) fn derive_gossip_topic_from_config(config: &NodeConfig) -> TopicId {
    if let Some(ref ticket_str) = config.iroh.gossip_ticket {
        match AspenClusterTicket::deserialize(ticket_str) {
            Ok(ticket) => {
                info!(
                    cluster_id = %ticket.cluster_id,
                    bootstrap_peers = ticket.bootstrap.len(),
                    "using topic ID from cluster ticket"
                );
                return ticket.topic_id;
            }
            Err(err) => {
                warn!(
                    error = %err,
                    "failed to parse gossip ticket, falling back to cookie-derived topic"
                );
            }
        }
    }
    derive_topic_id_from_cookie(&config.cookie)
}

/// Setup gossip discovery if enabled.
pub(super) async fn setup_gossip_discovery(
    config: &NodeConfig,
    gossip_topic_id: TopicId,
    iroh_manager: &Arc<IrohEndpointManager>,
    network_factory: &Arc<IrpcRaftNetworkFactory>,
) -> Option<GossipPeerDiscovery> {
    if !config.iroh.enable_gossip {
        info!(
            node_id = config.node_id,
            topic_id = %hex::encode(gossip_topic_id.as_bytes()),
            "gossip discovery disabled by configuration (topic ID still available for tickets)"
        );
        return None;
    }

    info!(
        node_id = config.node_id,
        topic_id = %hex::encode(gossip_topic_id.as_bytes()),
        "starting gossip discovery"
    );

    match spawn_gossip_peer_discovery(
        gossip_topic_id,
        config.node_id.into(),
        iroh_manager,
        Some(network_factory.clone()),
    )
    .await
    {
        Ok(discovery) => {
            info!(
                node_id = config.node_id,
                topic_id = %hex::encode(gossip_topic_id.as_bytes()),
                "gossip discovery started successfully"
            );
            Some(discovery)
        }
        Err(err) => {
            warn!(
                error = %err,
                node_id = config.node_id,
                "failed to start gossip discovery, continuing without it"
            );
            None
        }
    }
}

/// Initialize global content discovery service if enabled.
///
/// Content discovery uses the BitTorrent Mainline DHT to announce and
/// find blobs across clusters without direct federation.
///
/// When `auto_announce` is enabled in the configuration, this function
/// will also scan the local blob store and announce all existing blobs
/// to the DHT.
///
/// # Arguments
/// * `config` - Node configuration with content discovery settings
/// * `iroh_manager` - Iroh endpoint manager for network access
/// * `shutdown` - Cancellation token inherited from parent
///
/// # Returns
/// Tuple of (content_discovery_service, cancellation_token)
/// Both are None when content discovery is disabled.
pub(super) async fn initialize_content_discovery(
    config: &NodeConfig,
    iroh_manager: &Arc<IrohEndpointManager>,
    shutdown: &CancellationToken,
) -> Result<(Option<crate::content_discovery::ContentDiscoveryService>, Option<CancellationToken>)> {
    use crate::content_discovery::ContentDiscoveryService;

    if !config.content_discovery.enabled {
        return Ok((None, None));
    }

    info!(
        node_id = config.node_id,
        server_mode = config.content_discovery.server_mode,
        dht_port = config.content_discovery.dht_port,
        auto_announce = config.content_discovery.auto_announce,
        "initializing global content discovery (DHT)"
    );

    // Create a child cancellation token for the content discovery service
    let cancel = shutdown.child_token();

    // Spawn the content discovery service
    let (service, _task) = ContentDiscoveryService::spawn(
        Arc::new(iroh_manager.endpoint().clone()),
        iroh_manager.secret_key().clone(),
        config.content_discovery.clone(),
        cancel.clone(),
    )
    .await?;

    info!(
        node_id = config.node_id,
        public_key = %service.public_key(),
        "content discovery service started"
    );

    Ok((Some(service), Some(cancel)))
}
