//! Actor-based gossip peer discovery with supervision.
//!
//! This module provides an actor wrapper around GossipPeerDiscovery that:
//! - Manages gossip lifecycle with supervision
//! - Provides message-based APIs for peer queries and control
//! - Enforces Tiger Style bounded peer counts
//! - Integrates with the cluster supervision tree
//!
//! # Architecture
//!
//! The actor follows ractor v0.15.9's pattern of immutable actor struct with mutable state:
//! - `GossipActor`: Empty shell (zero-sized type) implementing the `Actor` trait
//! - `GossipActorState`: All mutable runtime state including peer list and metrics
//! - `GossipMessage`: Message enum (no Clone due to RpcReplyPort one-time use)
//!
//! ## Key Design Patterns
//!
//! 1. **Zero-Sized Actor Type**: The actor struct contains no data. All mutable state is
//!    managed in `GossipActorState`, following ractor's immutability requirements.
//!
//! 2. **Hybrid Task Management**: The actor spawns a `GossipPeerDiscovery` task that runs
//!    the actual gossip protocol, while the actor provides supervision and message handling.
//!
//! 3. **Bounded Peer Management**: Enforces MAX_PEER_COUNT (default: 100) to prevent unbounded
//!    growth of the peer list, following Tiger Style principles.
//!
//! 4. **Automatic Network Registration**: Discovered peers are automatically registered with
//!    the `IrpcRaftNetworkFactory` to enable Raft RPC connectivity.
//!
//! # Example
//!
//! ```rust,ignore
//! use aspen::cluster::gossip_actor::{GossipActor, GossipMessage, GossipActorArgs};
//! use ractor::Actor;
//!
//! let (actor_ref, handle) = Actor::spawn(
//!     Some("gossip".to_string()),
//!     GossipActor,
//!     GossipActorArgs { topic_id, node_id, endpoint_manager, network_factory },
//! ).await?;
//!
//! // Query peers
//! let peers = ractor::call_t!(
//!     actor_ref,
//!     GossipMessage::GetPeers,
//!     1000
//! )?;
//!
//! // Get statistics
//! let stats = ractor::call_t!(
//!     actor_ref,
//!     GossipMessage::GetStats,
//!     1000
//! )?;
//! println!("Known peers: {}, Announcements sent: {}",
//!          stats.known_peers, stats.announcements_sent);
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use async_trait::async_trait;
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use ractor::{Actor, ActorProcessingErr, ActorRef, RpcReplyPort, SupervisionEvent};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use super::IrohEndpointManager;
use super::gossip_discovery::GossipPeerDiscovery;
use crate::raft::constants::MAX_PEER_COUNT;
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::types::NodeId;

// ============================================================================
// Messages
// ============================================================================

/// Messages for the gossip actor.
///
/// Note: Does NOT derive Clone because RpcReplyPort is not cloneable.
#[derive(Debug)]
pub enum GossipMessage {
    /// A new peer was discovered (internal notification).
    PeerDiscovered(PeerInfo),
    /// Get the list of known peers.
    GetPeers(RpcReplyPort<Vec<PeerInfo>>),
    /// Get statistics about gossip discovery.
    GetStats(RpcReplyPort<GossipStats>),
    /// Update our own endpoint address (for re-announcement).
    UpdateEndpoint(EndpointAddr),
    /// Set maximum peer count (Tiger Style: bounded).
    SetMaxPeers(u32),
    /// Force an immediate announcement.
    ForceAnnounce,
    /// Check if gossip is healthy.
    HealthCheck(RpcReplyPort<bool>),
    /// Graceful shutdown request.
    Shutdown,
}

impl ractor::Message for GossipMessage {}

// ============================================================================
// Types
// ============================================================================

/// Information about a discovered peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Node ID of the peer.
    pub node_id: NodeId,
    /// Iroh endpoint address of the peer.
    pub endpoint_addr: EndpointAddr,
    /// Seconds since last seen (for freshness tracking).
    pub last_seen_secs_ago: u64,
    /// Number of times this peer has been announced.
    pub announcement_count: u32,
}

/// Gossip statistics exposed via GetStats message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipStats {
    /// Topic ID as hex string.
    pub topic_id: String,
    /// Number of known peers.
    pub known_peers: u32,
    /// Maximum peers allowed (Tiger Style limit).
    pub max_peers: u32,
    /// Total announcements sent.
    pub announcements_sent: u64,
    /// Total announcements received.
    pub announcements_received: u64,
    /// Discovery errors encountered.
    pub discovery_errors: u64,
    /// Whether gossip discovery is running.
    pub is_running: bool,
}

/// Arguments passed to the actor on startup.
#[derive(Clone)]
pub struct GossipActorArgs {
    /// Gossip topic ID (derived from cluster cookie).
    pub topic_id: TopicId,
    /// Our node ID.
    pub node_id: NodeId,
    /// Iroh endpoint manager.
    pub endpoint_manager: Arc<IrohEndpointManager>,
    /// Optional network factory to register discovered peers with.
    pub network_factory: Option<Arc<IrpcRaftNetworkFactory>>,
}

// ============================================================================
// Actor and State
// ============================================================================

/// Actor shell for gossip peer discovery.
///
/// This is an empty struct following ractor's pattern where the actor
/// is stateless and all mutable state lives in `Self::State`.
pub struct GossipActor;

/// Mutable state for the gossip actor.
///
/// Tiger Style: Peer count bounded by max_peers.
pub struct GossipActorState {
    /// The underlying gossip discovery handle.
    discovery_handle: Option<GossipPeerDiscovery>,
    /// Known peers with metadata (bounded by max_peers).
    known_peers: HashMap<NodeId, PeerInfo>,
    /// Maximum number of peers to track.
    max_peers: u32,
    /// Total announcements sent.
    announcements_sent: u64,
    /// Total announcements received.
    announcements_received: u64,
    /// Discovery errors encountered.
    discovery_errors: u64,
    /// Cached arguments for potential restart.
    args: GossipActorArgs,
}

impl GossipActorState {
    /// Start gossip discovery.
    async fn start_discovery(&mut self) -> Result<(), ActorProcessingErr> {
        if self.discovery_handle.is_some() {
            warn!("gossip discovery already running");
            return Ok(());
        }

        info!(
            topic_id = ?self.args.topic_id,
            node_id = self.args.node_id,
            "starting gossip discovery"
        );

        match GossipPeerDiscovery::spawn(
            self.args.topic_id,
            self.args.node_id,
            &self.args.endpoint_manager,
            self.args.network_factory.clone(),
        )
        .await
        .context("failed to spawn gossip discovery")
        {
            Ok(handle) => {
                self.discovery_handle = Some(handle);
                Ok(())
            }
            Err(e) => {
                self.discovery_errors = self.discovery_errors.saturating_add(1);
                Err(ActorProcessingErr::from(e.to_string()))
            }
        }
    }

    /// Stop gossip discovery gracefully.
    async fn stop_discovery(&mut self) {
        if let Some(handle) = self.discovery_handle.take() {
            info!("stopping gossip discovery");
            if let Err(e) = handle.shutdown().await {
                error!(error = %e, "failed to shutdown gossip discovery gracefully");
                self.discovery_errors = self.discovery_errors.saturating_add(1);
            }
        }
    }

    /// Add or update a discovered peer.
    ///
    /// Tiger Style: Enforces max_peers limit by removing oldest peer when full.
    fn add_peer(&mut self, peer: PeerInfo) {
        // Check if we're at capacity and this is a new peer
        if self.known_peers.len() >= self.max_peers as usize
            && !self.known_peers.contains_key(&peer.node_id)
        {
            // Find and remove the oldest peer (highest last_seen_secs_ago)
            if let Some(oldest_id) = self
                .known_peers
                .iter()
                .max_by_key(|(_, p)| p.last_seen_secs_ago)
                .map(|(id, _)| *id)
            {
                warn!(
                    removed_peer = oldest_id,
                    max_peers = self.max_peers,
                    "peer limit reached, removing oldest peer"
                );
                self.known_peers.remove(&oldest_id);
            }
        }

        // Update or insert the peer
        self.known_peers
            .entry(peer.node_id)
            .and_modify(|p| {
                p.endpoint_addr = peer.endpoint_addr.clone();
                p.last_seen_secs_ago = 0;
                p.announcement_count = p.announcement_count.saturating_add(1);
            })
            .or_insert(peer);

        self.announcements_received = self.announcements_received.saturating_add(1);
    }

    /// Get current statistics.
    fn get_stats(&self) -> GossipStats {
        GossipStats {
            topic_id: format!("{:?}", self.args.topic_id),
            known_peers: self.known_peers.len() as u32,
            max_peers: self.max_peers,
            announcements_sent: self.announcements_sent,
            announcements_received: self.announcements_received,
            discovery_errors: self.discovery_errors,
            is_running: self.discovery_handle.is_some(),
        }
    }

    /// Get list of known peers.
    fn get_peers(&self) -> Vec<PeerInfo> {
        self.known_peers.values().cloned().collect()
    }
}

// ============================================================================
// Actor Implementation
// ============================================================================

#[async_trait]
impl Actor for GossipActor {
    type Msg = GossipMessage;
    type State = GossipActorState;
    type Arguments = GossipActorArgs;

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!(
            topic_id = ?args.topic_id,
            node_id = args.node_id,
            "GossipActor starting"
        );

        let mut state = GossipActorState {
            discovery_handle: None,
            known_peers: HashMap::new(),
            max_peers: MAX_PEER_COUNT,
            announcements_sent: 0,
            announcements_received: 0,
            discovery_errors: 0,
            args,
        };

        // Start discovery immediately
        state.start_discovery().await?;

        Ok(state)
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        info!("GossipActor stopping");
        state.stop_discovery().await;
        Ok(())
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            GossipMessage::PeerDiscovered(peer) => {
                let node_id = peer.node_id;
                info!(
                    node_id = node_id,
                    endpoint_id = ?peer.endpoint_addr.id,
                    "peer discovered via gossip"
                );
                state.add_peer(peer);

                // Forward to network factory if available
                if let Some(ref factory) = state.args.network_factory
                    && let Some(peer_info) = state.known_peers.get(&node_id)
                {
                    factory
                        .add_peer(peer_info.node_id, peer_info.endpoint_addr.clone())
                        .await;
                }
            }
            GossipMessage::GetPeers(reply) => {
                let peers = state.get_peers();
                if reply.send(peers).is_err() {
                    warn!("failed to send peers reply - caller dropped");
                }
            }
            GossipMessage::GetStats(reply) => {
                let stats = state.get_stats();
                if reply.send(stats).is_err() {
                    warn!("failed to send stats reply - caller dropped");
                }
            }
            GossipMessage::UpdateEndpoint(_addr) => {
                info!("endpoint update received (announcement triggered on next interval)");
                // The GossipPeerDiscovery already handles periodic announcements.
                // A future enhancement could trigger an immediate announcement.
            }
            GossipMessage::SetMaxPeers(max) => {
                // Tiger Style: Enforce reasonable bounds [1, 10000]
                let bounded_max = max.clamp(1, 10000);
                info!(
                    old_max = state.max_peers,
                    new_max = bounded_max,
                    "updating max peers"
                );
                state.max_peers = bounded_max;

                // Remove excess peers if we're over the new limit
                while state.known_peers.len() > state.max_peers as usize {
                    if let Some(oldest_id) = state
                        .known_peers
                        .iter()
                        .max_by_key(|(_, p)| p.last_seen_secs_ago)
                        .map(|(id, _)| *id)
                    {
                        state.known_peers.remove(&oldest_id);
                    } else {
                        break;
                    }
                }
            }
            GossipMessage::ForceAnnounce => {
                info!("force announce requested");
                state.announcements_sent = state.announcements_sent.saturating_add(1);
                // The underlying GossipPeerDiscovery handles announcements on a fixed interval.
                // This is a placeholder for future immediate announcement support.
            }
            GossipMessage::HealthCheck(reply) => {
                let is_healthy = state.discovery_handle.is_some();
                if reply.send(is_healthy).is_err() {
                    warn!("failed to send health check reply - caller dropped");
                }
            }
            GossipMessage::Shutdown => {
                info!("received shutdown request");
                state.stop_discovery().await;
                myself.stop(None);
            }
        }
        Ok(())
    }

    async fn handle_supervisor_evt(
        &self,
        _myself: ActorRef<Self::Msg>,
        event: SupervisionEvent,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match event {
            SupervisionEvent::ActorFailed(actor_cell, error) => {
                error!(
                    actor = ?actor_cell.get_id(),
                    error = %error,
                    "supervised actor failed"
                );
                state.discovery_errors = state.discovery_errors.saturating_add(1);

                // Restart discovery if it's not running
                if state.discovery_handle.is_none() {
                    warn!("restarting gossip discovery after supervised failure");
                    if let Err(e) = state.start_discovery().await {
                        error!(error = %e, "failed to restart gossip discovery");
                    }
                }
            }
            SupervisionEvent::ActorTerminated(actor_cell, _, reason) => {
                info!(
                    actor = ?actor_cell.get_id(),
                    reason = ?reason,
                    "supervised actor terminated"
                );
            }
            _ => {}
        }
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gossip_stats_creation() {
        let stats = GossipStats {
            topic_id: "test-topic".to_string(),
            known_peers: 5,
            max_peers: 100,
            announcements_sent: 50,
            announcements_received: 75,
            discovery_errors: 2,
            is_running: true,
        };

        assert_eq!(stats.known_peers, 5);
        assert_eq!(stats.max_peers, 100);
        assert_eq!(stats.announcements_sent, 50);
        assert_eq!(stats.announcements_received, 75);
        assert_eq!(stats.discovery_errors, 2);
        assert!(stats.is_running);
    }

    #[test]
    fn test_gossip_stats_serialization() {
        let stats = GossipStats {
            topic_id: "cluster-123".to_string(),
            known_peers: 10,
            max_peers: 1000,
            announcements_sent: 100,
            announcements_received: 150,
            discovery_errors: 1,
            is_running: true,
        };

        // Should serialize to JSON without panic
        let json = serde_json::to_string(&stats).expect("serialization failed");
        assert!(json.contains("cluster-123"));
        assert!(json.contains("1000"));

        // Should deserialize back
        let deserialized: GossipStats =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(deserialized.known_peers, 10);
    }

    #[test]
    fn test_peer_info_creation() {
        let addr = EndpointAddr::new(iroh::SecretKey::from([1u8; 32]).public());
        let peer = PeerInfo {
            node_id: 123,
            endpoint_addr: addr.clone(),
            last_seen_secs_ago: 0,
            announcement_count: 1,
        };

        assert_eq!(peer.node_id, 123);
        assert_eq!(peer.endpoint_addr, addr);
        assert_eq!(peer.last_seen_secs_ago, 0);
        assert_eq!(peer.announcement_count, 1);
    }

    #[test]
    fn test_max_peers_bounds() {
        // Test the clamping logic that will be used in SetMaxPeers
        let test_cases = [
            (0u32, 1u32),   // Below minimum -> clamp to 1
            (1, 1),         // At minimum -> unchanged
            (500, 500),     // Normal value -> unchanged
            (10000, 10000), // At maximum -> unchanged
            (20000, 10000), // Above maximum -> clamp to 10000
        ];

        for (input, expected) in test_cases {
            let bounded = input.clamp(1, 10000);
            assert_eq!(bounded, expected, "clamp({}) should be {}", input, expected);
        }
    }

    #[test]
    fn test_peer_info_serialization() {
        let addr = EndpointAddr::new(iroh::SecretKey::from([2u8; 32]).public());
        let peer = PeerInfo {
            node_id: 456,
            endpoint_addr: addr,
            last_seen_secs_ago: 30,
            announcement_count: 5,
        };

        // Should serialize to JSON without panic
        let json = serde_json::to_string(&peer).expect("serialization failed");
        assert!(json.contains("456"));
        assert!(json.contains("30"));

        // Should deserialize back
        let deserialized: PeerInfo = serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(deserialized.node_id, 456);
        assert_eq!(deserialized.announcement_count, 5);
    }
}
