//! Aspen cluster tickets with direct address support.

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

/// Bootstrap peer information including direct socket addresses.
///
/// Unlike legacy tickets which only stored EndpointId (public key), this struct
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

/// Aspen cluster ticket for gossip-based peer discovery.
///
/// Contains all information needed to join an Aspen cluster via iroh-gossip:
/// - `topic_id`: The gossip topic for cluster membership announcements
/// - `bootstrap`: List of initial peers with direct socket addresses
/// - `cluster_id`: Human-readable cluster identifier
///
/// This ticket format includes direct socket addresses for bootstrap peers,
/// enabling connection without discovery mechanisms. Use this for:
/// - VM networking where mDNS doesn't work
/// - Relay-disabled environments
/// - Air-gapped deployments
///
/// Tiger Style: Fixed limits on peers and addresses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AspenClusterTicket {
    /// Gossip topic ID for cluster membership.
    pub topic_id: TopicId,
    /// Bootstrap peers with direct addresses (max 16 peers).
    pub bootstrap: Vec<BootstrapPeer>,
    /// Human-readable cluster identifier.
    pub cluster_id: String,
}

impl AspenClusterTicket {
    /// Maximum number of bootstrap peers in a ticket.
    ///
    /// Tiger Style: Fixed limit to prevent unbounded ticket size.
    pub const MAX_BOOTSTRAP_PEERS: usize = 16;

    /// Maximum direct addresses per peer.
    pub const MAX_DIRECT_ADDRS_PER_PEER: usize = 8;

    /// Create a new ticket with a topic ID and cluster identifier.
    ///
    /// The bootstrap peer list is initially empty. Use `with_bootstrap_addr()` or
    /// `add_bootstrap_addr()` to add peers.
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

    /// Create a ticket with a single bootstrap peer from just an EndpointId (no addresses).
    pub fn with_bootstrap(topic_id: TopicId, cluster_id: String, bootstrap_peer: EndpointId) -> Self {
        let mut ticket = Self::new(topic_id, cluster_id);
        ticket.bootstrap.push(BootstrapPeer::new(bootstrap_peer));
        ticket
    }

    /// Add a bootstrap peer from an EndpointAddr.
    ///
    /// Returns `Err` if the maximum number of bootstrap peers is reached.
    ///
    /// Tiger Style: Fail fast on limit violation.
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

    /// Add a bootstrap peer from just an EndpointId (no direct addresses).
    ///
    /// Returns `Err` if the maximum number of bootstrap peers (16) is reached.
    ///
    /// Tiger Style: Fail fast on limit violation.
    pub fn add_bootstrap(&mut self, peer: EndpointId) -> Result<()> {
        if self.bootstrap.len() >= Self::MAX_BOOTSTRAP_PEERS {
            anyhow::bail!("cannot add more than {} bootstrap peers to ticket", Self::MAX_BOOTSTRAP_PEERS);
        }
        self.bootstrap.push(BootstrapPeer::new(peer));
        Ok(())
    }

    /// Get all endpoint addresses for direct connection.
    pub fn endpoint_addrs(&self) -> Vec<EndpointAddr> {
        self.bootstrap.iter().map(|p| p.to_endpoint_addr()).collect()
    }

    /// Get just the endpoint IDs.
    pub fn endpoint_ids(&self) -> BTreeSet<EndpointId> {
        self.bootstrap.iter().map(|p| p.endpoint_id).collect()
    }

    /// Serialize the ticket to a base32-encoded string.
    ///
    /// The format is: `aspen{base32-encoded-postcard-payload}`
    ///
    /// # Example
    ///
    /// ```
    /// # use aspen_ticket::AspenClusterTicket;
    /// # use iroh_gossip::proto::TopicId;
    /// let ticket = AspenClusterTicket::new(
    ///     TopicId::from_bytes([1u8; 32]),
    ///     "test-cluster".into(),
    /// );
    /// let serialized = ticket.serialize();
    /// assert!(serialized.starts_with("aspen"));
    /// ```
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket from a base32-encoded string.
    ///
    /// Returns an error if the string is not a valid Aspen ticket.
    ///
    /// # Example
    ///
    /// ```
    /// # use aspen_ticket::AspenClusterTicket;
    /// # use iroh_gossip::proto::TopicId;
    /// let ticket = AspenClusterTicket::new(
    ///     TopicId::from_bytes([1u8; 32]),
    ///     "test-cluster".into(),
    /// );
    /// let serialized = ticket.serialize();
    /// let deserialized = AspenClusterTicket::deserialize(&serialized)?;
    /// assert_eq!(ticket, deserialized);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn deserialize(input: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(input).context("failed to deserialize Aspen ticket")
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

impl Ticket for AspenClusterTicket {
    const KIND: &'static str = "aspen";

    fn to_bytes(&self) -> Vec<u8> {
        // Tiger Style: .expect() required by iroh_tickets::Ticket trait - cannot return Result.
        // WHY: The Ticket trait defines `fn to_bytes(&self) -> Vec<u8>` with no Result.
        // WHAT would fail: Only a postcard library bug, memory corruption, or OOM.
        // WHY safe: All fields are bounded (MAX_BOOTSTRAP_PEERS=16, MAX_DIRECT_ADDRS_PER_PEER=8)
        //   with deterministic serialization of primitive types (TopicId, Vec<BootstrapPeer>, String).
        postcard::to_stdvec(&self).expect(
            "AspenClusterTicket postcard serialization failed - \
             indicates library bug or memory corruption",
        )
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}
