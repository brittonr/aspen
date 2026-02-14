//! V2 Aspen cluster tickets with direct address support.

use std::collections::BTreeSet;
use std::net::SocketAddr;

use anyhow::Context;
use anyhow::Result;
use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh::TransportAddr;
use iroh_gossip::proto::TopicId;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

use crate::AspenClusterTicket;

/// Bootstrap peer information including direct socket addresses.
///
/// Unlike the V1 ticket which only stores EndpointId (public key), this struct
/// includes the direct socket addresses needed to establish connections without
/// relying on discovery mechanisms (mDNS, DNS, DHT, or relay).
///
/// This is essential for:
/// - VM-to-host networking where multicast (mDNS) doesn't traverse
/// - Air-gapped environments without relay/DNS access
/// - Testing scenarios with disabled discovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapPeer {
    /// The peer's cryptographic identity (public key).
    pub endpoint_id: EndpointId,
    /// Direct socket addresses for this peer (e.g., "10.100.0.11:7777").
    /// Can be empty if only relay-based connection is intended.
    pub direct_addrs: Vec<SocketAddr>,
}

impl BootstrapPeer {
    /// Create a new bootstrap peer with only the endpoint ID (no direct addresses).
    pub fn new(endpoint_id: EndpointId) -> Self {
        Self {
            endpoint_id,
            direct_addrs: Vec::new(),
        }
    }

    /// Create a bootstrap peer with direct addresses from an EndpointAddr.
    pub fn from_endpoint_addr(addr: &EndpointAddr) -> Self {
        // Extract direct IP addresses from TransportAddr enum
        let direct_addrs = addr
            .addrs
            .iter()
            .filter_map(|transport_addr| match transport_addr {
                TransportAddr::Ip(socket_addr) => Some(*socket_addr),
                _ => None,
            })
            .collect();

        Self {
            endpoint_id: addr.id,
            direct_addrs,
        }
    }

    /// Convert to an EndpointAddr for connection.
    pub fn to_endpoint_addr(&self) -> EndpointAddr {
        let addrs: BTreeSet<TransportAddr> = self.direct_addrs.iter().map(|addr| TransportAddr::Ip(*addr)).collect();

        EndpointAddr {
            id: self.endpoint_id,
            addrs,
        }
    }
}

impl From<&EndpointAddr> for BootstrapPeer {
    fn from(addr: &EndpointAddr) -> Self {
        Self::from_endpoint_addr(addr)
    }
}

impl From<EndpointId> for BootstrapPeer {
    fn from(id: EndpointId) -> Self {
        Self::new(id)
    }
}

/// Aspen cluster ticket V2 with direct address support.
///
/// This ticket format includes direct socket addresses for bootstrap peers,
/// enabling connection without discovery mechanisms. Use this for:
/// - VM networking where mDNS doesn't work
/// - Relay-disabled environments
/// - Air-gapped deployments
///
/// The ticket is backward-compatible: clients that only support V1 can fall back
/// to discovery-based connection using just the endpoint IDs.
///
/// Tiger Style: Fixed limits on peers and addresses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AspenClusterTicketV2 {
    /// Gossip topic ID for cluster membership.
    pub topic_id: TopicId,
    /// Bootstrap peers with direct addresses (max 16 peers).
    pub bootstrap: Vec<BootstrapPeer>,
    /// Human-readable cluster identifier.
    pub cluster_id: String,
}

impl AspenClusterTicketV2 {
    /// Maximum number of bootstrap peers in a ticket.
    pub const MAX_BOOTSTRAP_PEERS: usize = 16;

    /// Maximum direct addresses per peer.
    pub const MAX_DIRECT_ADDRS_PER_PEER: usize = 8;

    /// Create a new V2 ticket with a topic ID and cluster identifier.
    pub fn new(topic_id: TopicId, cluster_id: String) -> Self {
        Self {
            topic_id,
            bootstrap: Vec::new(),
            cluster_id,
        }
    }

    /// Create a ticket with a single bootstrap peer from an EndpointAddr.
    pub fn with_bootstrap_addr(topic_id: TopicId, cluster_id: String, addr: &EndpointAddr) -> Self {
        let mut ticket = Self::new(topic_id, cluster_id);
        ticket.bootstrap.push(BootstrapPeer::from_endpoint_addr(addr));
        ticket
    }

    /// Add a bootstrap peer from an EndpointAddr.
    ///
    /// Returns `Err` if the maximum number of bootstrap peers is reached.
    pub fn add_bootstrap_addr(&mut self, addr: &EndpointAddr) -> Result<()> {
        if self.bootstrap.len() >= Self::MAX_BOOTSTRAP_PEERS {
            anyhow::bail!("cannot add more than {} bootstrap peers to ticket", Self::MAX_BOOTSTRAP_PEERS);
        }
        let mut peer = BootstrapPeer::from_endpoint_addr(addr);
        // Limit direct addresses per peer
        peer.direct_addrs.truncate(Self::MAX_DIRECT_ADDRS_PER_PEER);
        self.bootstrap.push(peer);
        Ok(())
    }

    /// Get all endpoint addresses for direct connection.
    pub fn endpoint_addrs(&self) -> Vec<EndpointAddr> {
        self.bootstrap.iter().map(|p| p.to_endpoint_addr()).collect()
    }

    /// Get just the endpoint IDs (for V1 compatibility).
    pub fn endpoint_ids(&self) -> BTreeSet<EndpointId> {
        self.bootstrap.iter().map(|p| p.endpoint_id).collect()
    }

    /// Convert to a V1 ticket (loses direct address information).
    pub fn to_v1(&self) -> AspenClusterTicket {
        let mut ticket = AspenClusterTicket::new(self.topic_id, self.cluster_id.clone());
        for peer in &self.bootstrap {
            // Ignore errors from max peers limit
            let _ = ticket.add_bootstrap(peer.endpoint_id);
        }
        ticket
    }

    /// Create from a V1 ticket (no direct addresses).
    pub fn from_v1(v1: &AspenClusterTicket) -> Self {
        Self {
            topic_id: v1.topic_id,
            bootstrap: v1.bootstrap.iter().map(|id| BootstrapPeer::new(*id)).collect(),
            cluster_id: v1.cluster_id.clone(),
        }
    }

    /// Serialize the ticket to a base32-encoded string.
    ///
    /// The format is: `aspenv2{base32-encoded-postcard-payload}`
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket from a base32-encoded string.
    pub fn deserialize(input: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(input).context("failed to deserialize Aspen V2 ticket")
    }

    /// Inject an additional direct address into all bootstrap peers.
    ///
    /// This is used for VM connectivity where the bridge IP must be added
    /// to the ticket so VMs can reach the host's Iroh endpoint from the
    /// isolated VM network (e.g., 10.200.0.0/24).
    ///
    /// The address is only added if it's not already present in the peer's
    /// direct address list.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address to inject (e.g., 10.200.0.1:PORT)
    pub fn inject_direct_addr(&mut self, addr: SocketAddr) {
        for peer in &mut self.bootstrap {
            if !peer.direct_addrs.contains(&addr) {
                peer.direct_addrs.push(addr);
            }
        }
    }
}

impl Ticket for AspenClusterTicketV2 {
    const KIND: &'static str = "aspenv2";

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect(
            "AspenClusterTicketV2 postcard serialization failed - \
             indicates library bug or memory corruption",
        )
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}
