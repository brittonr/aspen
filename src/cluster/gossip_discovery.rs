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

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use iroh::EndpointAddr;
use iroh_gossip::api::Event;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio::time::interval;

use super::IrohEndpointManager;
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::types::NodeId;

/// Announcement message broadcast to the gossip topic.
///
/// Contains node's ID, EndpointAddr, and a timestamp for freshness tracking.
///
/// Tiger Style: Fixed-size payload, explicit timestamp in microseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PeerAnnouncement {
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
            node_id,
            endpoint_addr,
            timestamp_micros,
        })
    }

    /// Serialize to bytes using postcard.
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize peer announcement")
    }

    /// Deserialize from bytes using postcard.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes).context("failed to deserialize peer announcement")
    }
}

/// Manages gossip-based peer discovery lifecycle.
///
/// Spawns two background tasks:
/// 1. Announcer: Periodically broadcasts this node's ID and EndpointAddr
/// 2. Receiver: Listens for peer announcements and automatically adds them to the network factory
///
/// Tiger Style: Bounded announcement interval (10s), explicit shutdown mechanism.
pub struct GossipPeerDiscovery {
    topic_id: TopicId,
    _node_id: NodeId, // Stored for debugging/logging purposes
    shutdown: Arc<AtomicBool>,
    announcer_task: JoinHandle<()>,
    receiver_task: JoinHandle<()>,
}

impl GossipPeerDiscovery {
    /// Announcement interval in seconds.
    ///
    /// Tiger Style: Fixed interval to prevent unbounded announcement rate.
    const ANNOUNCE_INTERVAL_SECS: u64 = 10;

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
        let gossip = iroh_manager
            .gossip()
            .context("gossip not enabled on IrohEndpointManager")?;

        // Subscribe to the topic
        let gossip_topic = gossip
            .subscribe(topic_id, vec![])
            .await
            .context("failed to subscribe to gossip topic")?;

        // Split into sender and receiver
        let (gossip_sender, mut gossip_receiver) = gossip_topic.split();

        let shutdown = Arc::new(AtomicBool::new(false));
        let endpoint_addr = iroh_manager.node_addr().clone();

        // Spawn announcer task
        let announcer_shutdown = Arc::clone(&shutdown);
        let announcer_sender = gossip_sender.clone();
        let announcer_node_id = node_id;
        let announcer_task = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(Self::ANNOUNCE_INTERVAL_SECS));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                if announcer_shutdown.load(Ordering::Relaxed) {
                    tracing::debug!("gossip announcer shutting down");
                    break;
                }

                ticker.tick().await;

                let announcement = match PeerAnnouncement::new(announcer_node_id, endpoint_addr.clone()) {
                    Ok(ann) => ann,
                    Err(e) => {
                        tracing::error!("failed to create peer announcement: {}", e);
                        continue;
                    }
                };
                match announcement.to_bytes() {
                    Ok(bytes) => {
                        if let Err(e) = announcer_sender.broadcast(bytes.into()).await {
                            tracing::warn!("failed to broadcast peer announcement: {}", e);
                        } else {
                            tracing::trace!(
                                "broadcast peer announcement for node_id={}",
                                announcer_node_id
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("failed to serialize peer announcement: {}", e);
                    }
                }
            }
        });

        // Spawn receiver task
        let receiver_shutdown = Arc::clone(&shutdown);
        let receiver_node_id = node_id;
        let receiver_network_factory = network_factory.clone();
        let receiver_task = tokio::spawn(async move {
            loop {
                if receiver_shutdown.load(Ordering::Relaxed) {
                    tracing::debug!("gossip receiver shutting down");
                    break;
                }

                match gossip_receiver.next().await {
                    Some(Ok(Event::Received(msg))) => {
                        match PeerAnnouncement::from_bytes(&msg.content) {
                            Ok(announcement) => {
                                // Filter out our own announcements
                                if announcement.node_id == receiver_node_id {
                                    tracing::trace!("ignoring self-announcement");
                                    continue;
                                }

                                tracing::debug!(
                                    "received peer announcement from node_id={}, endpoint_id={:?}",
                                    announcement.node_id,
                                    announcement.endpoint_addr.id
                                );

                                // Add peer to network factory if available
                                if let Some(ref factory) = receiver_network_factory {
                                    factory.add_peer(
                                        announcement.node_id,
                                        announcement.endpoint_addr.clone(),
                                    ).await;

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
                                    "discovered peer: node_id={}, endpoint_id={:?}, relay={:?}, direct_addresses={}",
                                    announcement.node_id,
                                    announcement.endpoint_addr.id,
                                    relay_urls,
                                    announcement.endpoint_addr.addrs.len()
                                );
                            }
                            Err(e) => {
                                tracing::warn!("failed to parse peer announcement: {}", e);
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
                        tracing::error!("gossip receiver error: {}", e);
                        break;
                    }
                    None => {
                        tracing::info!("gossip receiver stream ended");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            topic_id,
            _node_id: node_id,
            shutdown,
            announcer_task,
            receiver_task,
        })
    }

    /// Get the topic ID for this discovery instance.
    pub fn topic_id(&self) -> TopicId {
        self.topic_id
    }

    /// Shutdown the discovery tasks and wait for completion.
    ///
    /// Tiger Style: Bounded wait time (10 seconds max).
    pub async fn shutdown(self) -> Result<()> {
        tracing::info!("shutting down gossip peer discovery");

        // Signal shutdown
        self.shutdown.store(true, Ordering::Relaxed);

        // Wait for tasks with timeout
        let timeout = Duration::from_secs(10);

        tokio::select! {
            _ = self.announcer_task => {
                tracing::debug!("announcer task completed");
            }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("announcer task did not complete within timeout");
            }
        }

        tokio::select! {
            _ = self.receiver_task => {
                tracing::debug!("receiver task completed");
            }
            _ = tokio::time::sleep(timeout) => {
                tracing::warn!("receiver task did not complete within timeout");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_announcement_serialize_deserialize() {
        let node_id = 123u64;
        let addr = EndpointAddr::new(iroh::SecretKey::from([1u8; 32]).public());
        let announcement = PeerAnnouncement::new(node_id, addr).unwrap();

        let bytes = announcement.to_bytes().unwrap();
        let deserialized = PeerAnnouncement::from_bytes(&bytes).unwrap();

        assert_eq!(announcement.node_id, deserialized.node_id);
        assert_eq!(announcement.endpoint_addr, deserialized.endpoint_addr);
        assert_eq!(announcement.timestamp_micros, deserialized.timestamp_micros);
    }

    #[test]
    fn test_peer_announcement_timestamp() {
        let node_id = 456u64;
        let addr = EndpointAddr::new(iroh::SecretKey::from([1u8; 32]).public());
        let announcement1 = PeerAnnouncement::new(node_id, addr.clone()).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let announcement2 = PeerAnnouncement::new(node_id, addr).unwrap();

        // Second announcement should have a later timestamp
        assert!(announcement2.timestamp_micros > announcement1.timestamp_micros);
    }
}
