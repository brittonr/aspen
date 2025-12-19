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

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use iroh::{EndpointAddr, SecretKey, Signature};
use iroh_gossip::api::Event;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

use super::IrohEndpointManager;
use crate::raft::network::IrpcRaftNetworkFactory;
use crate::raft::types::NodeId;

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
/// Tiger Style: Fixed-size payload, explicit timestamp in microseconds, versioned for forward compatibility.
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

        let cancel_token = CancellationToken::new();
        let endpoint_addr = iroh_manager.node_addr().clone();
        let secret_key = iroh_manager.secret_key().clone();

        // Spawn announcer task
        let announcer_cancel = cancel_token.child_token();
        let announcer_sender = gossip_sender.clone();
        let announcer_node_id = node_id;
        let announcer_task = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(Self::ANNOUNCE_INTERVAL_SECS));
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

                        match signed.to_bytes() {
                            Ok(bytes) => {
                                if let Err(e) = announcer_sender.broadcast(bytes.into()).await {
                                    tracing::warn!("failed to broadcast peer announcement: {}", e);
                                } else {
                                    tracing::trace!(
                                        "broadcast signed peer announcement for node_id={}",
                                        announcer_node_id
                                    );
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
            loop {
                tokio::select! {
                    _ = receiver_cancel.cancelled() => {
                        tracing::debug!("gossip receiver shutting down");
                        break;
                    }
                    event = gossip_receiver.next() => match event {
                    Some(Ok(Event::Received(msg))) => {
                        // Parse signed announcement
                        let signed = match SignedPeerAnnouncement::from_bytes(&msg.content) {
                            Some(s) => s,
                            None => {
                                tracing::warn!("failed to parse signed peer announcement");
                                continue;
                            }
                        };

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
        let mut announcer_task = self
            .announcer_task
            .take()
            .expect("announcer_task already consumed");
        let mut receiver_task = self
            .receiver_task
            .take()
            .expect("receiver_task already consumed");

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
}
