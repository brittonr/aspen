//! Aspen cluster tickets with alloc-safe bootstrap metadata.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::net::SocketAddr;

use aspen_cluster_types::NodeAddress;
use aspen_cluster_types::NodeTransportAddr;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use thiserror::Error;

/// Result alias for cluster ticket operations.
pub type ClusterTicketResult<T> = Result<T, ClusterTicketError>;

/// Maximum number of bootstrap peers in a ticket.
pub const MAX_BOOTSTRAP_PEERS: usize = 16;

/// Maximum number of direct IP addresses per peer.
pub const MAX_DIRECT_ADDRS_PER_PEER: usize = 8;

/// Maximum length of the cluster identifier in bytes.
pub const MAX_CLUSTER_ID_BYTES: usize = 1024;

/// Error returned by alloc-safe cluster ticket parsing and validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ClusterTicketError {
    /// Ticket bytes or text failed to decode.
    #[error("failed to deserialize Aspen cluster ticket: {reason}")]
    Deserialize {
        /// Human-readable decode failure.
        reason: String,
    },

    /// Cluster identifier exceeded the supported size.
    #[error("cluster_id too long: {actual_bytes} bytes (max {max_bytes})")]
    ClusterIdTooLong {
        /// Actual byte length.
        actual_bytes: u32,
        /// Maximum allowed byte length.
        max_bytes: u32,
    },

    /// The ticket stored too many bootstrap peers.
    #[error("too many bootstrap peers: {actual_peers} (max {max_peers})")]
    TooManyBootstrapPeers {
        /// Actual number of peers.
        actual_peers: u32,
        /// Maximum allowed peers.
        max_peers: u32,
    },

    /// A bootstrap peer stored too many direct addresses.
    #[error("too many direct addresses: {actual_addrs} (max {max_addrs})")]
    TooManyDirectAddrs {
        /// Actual number of addresses.
        actual_addrs: u32,
        /// Maximum allowed addresses.
        max_addrs: u32,
    },

    /// Endpoint identifier could not be converted to a runtime key.
    #[error("invalid endpoint id: {endpoint_id}")]
    InvalidEndpointId {
        /// Stored endpoint identifier.
        endpoint_id: String,
    },

    /// Topic conversion into runtime iroh-gossip form failed.
    #[error("invalid cluster topic id: {reason}")]
    InvalidTopicId {
        /// Human-readable conversion failure.
        reason: String,
    },

    /// Signed ticket version is newer than this crate understands.
    #[error("unsupported signed ticket version {version} (max supported: {max_supported})")]
    UnsupportedSignedVersion {
        /// Encountered version.
        version: u8,
        /// Maximum supported version.
        max_supported: u8,
    },

    /// Signed ticket verification failed.
    #[error("signature verification failed")]
    InvalidSignature,

    /// Signed ticket is expired at the supplied validation time.
    #[error("signed Aspen cluster ticket is expired")]
    ExpiredSignedTicket {
        /// Stored expiry timestamp.
        expires_at_secs: u64,
        /// Validation time.
        now_secs: u64,
    },

    /// Signed ticket was issued too far in the future.
    #[error("signed Aspen cluster ticket issued_at {issued_at_secs} is in the future relative to {now_secs}")]
    SignedTicketIssuedInFuture {
        /// Stored issue timestamp.
        issued_at_secs: u64,
        /// Validation time.
        now_secs: u64,
    },
}

/// Fixed-width cluster topic identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClusterTopicId([u8; 32]);

impl ClusterTopicId {
    /// Create a new topic identifier from raw bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a topic identifier from an arbitrary byte slice.
    pub fn try_from_slice(bytes: &[u8]) -> ClusterTicketResult<Self> {
        let fixed_bytes: [u8; 32] = bytes.try_into().map_err(|_| ClusterTicketError::InvalidTopicId {
            reason: format!("expected 32 bytes, got {}", bytes.len()),
        })?;
        Ok(Self::from_bytes(fixed_bytes))
    }

    /// Borrow the raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Alloc-safe endpoint identifier wrapper.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClusterEndpointId(String);

impl ClusterEndpointId {
    /// Create an endpoint identifier from explicit alloc-safe data.
    pub fn new(endpoint_id: impl Into<String>) -> Self {
        Self(endpoint_id.into())
    }

    /// Borrow the endpoint identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClusterEndpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Bootstrap peer information including direct socket addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapPeer {
    /// The peer's alloc-safe endpoint identifier.
    pub endpoint_id: ClusterEndpointId,
    /// Direct socket addresses for this peer.
    pub direct_addrs: Vec<SocketAddr>,
}

impl BootstrapPeer {
    /// Create a bootstrap peer with only an endpoint identifier.
    pub fn new(endpoint_id: impl Into<ClusterEndpointId>) -> Self {
        Self {
            endpoint_id: endpoint_id.into(),
            direct_addrs: Vec::new(),
        }
    }

    /// Create a bootstrap peer from an alloc-safe node address.
    pub fn from_node_address(node_addr: NodeAddress) -> Self {
        Self {
            endpoint_id: ClusterEndpointId::new(node_addr.endpoint_id()),
            direct_addrs: direct_addrs_from_node_address(&node_addr),
        }
    }

    /// Convert this peer into an alloc-safe node address.
    pub fn to_node_address(&self) -> NodeAddress {
        NodeAddress::from_parts(self.endpoint_id.to_string(), direct_addr_transports(&self.direct_addrs))
    }

    /// Inject a direct address if it is not already present.
    pub fn inject_direct_addr(&mut self, addr: SocketAddr) {
        if !self.direct_addrs.contains(&addr) && self.direct_addrs.len() < MAX_DIRECT_ADDRS_PER_PEER {
            self.direct_addrs.push(addr);
        }
    }
}

impl Serialize for BootstrapPeer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_node_address().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BootstrapPeer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let node_addr = NodeAddress::deserialize(deserializer)?;
        Ok(Self::from_node_address(node_addr))
    }
}

/// Aspen cluster ticket for gossip-based peer discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AspenClusterTicket {
    /// Gossip topic ID for cluster membership.
    pub topic_id: ClusterTopicId,
    /// Bootstrap peers with direct addresses.
    pub bootstrap: Vec<BootstrapPeer>,
    /// Human-readable cluster identifier.
    pub cluster_id: String,
}

impl AspenClusterTicket {
    /// Maximum number of bootstrap peers in a ticket.
    pub const MAX_BOOTSTRAP_PEERS: usize = MAX_BOOTSTRAP_PEERS;

    /// Maximum direct addresses per peer.
    pub const MAX_DIRECT_ADDRS_PER_PEER: usize = MAX_DIRECT_ADDRS_PER_PEER;

    /// Create a new ticket with a topic ID and cluster identifier.
    pub fn new(topic_id: impl Into<ClusterTopicId>, cluster_id: String) -> Self {
        Self {
            topic_id: topic_id.into(),
            bootstrap: Vec::new(),
            cluster_id,
        }
    }

    /// Add a bootstrap peer.
    pub fn add_bootstrap_peer(&mut self, mut peer: BootstrapPeer) -> ClusterTicketResult<()> {
        ensure_bootstrap_capacity(self.bootstrap.len())?;
        truncate_direct_addrs(&mut peer.direct_addrs);
        self.bootstrap.push(peer);
        Ok(())
    }

    /// Add a bootstrap peer from an alloc-safe node address.
    pub fn add_node_address(&mut self, node_addr: NodeAddress) -> ClusterTicketResult<()> {
        self.add_bootstrap_peer(BootstrapPeer::from_node_address(node_addr))
    }

    /// Serialize the ticket to a base32-encoded string.
    pub fn serialize(&self) -> String {
        self.validate().expect("AspenClusterTicket must be valid before serialization");
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket from a base32-encoded string.
    pub fn deserialize(input: &str) -> ClusterTicketResult<Self> {
        let ticket = <Self as Ticket>::deserialize(input).map_err(|error| ClusterTicketError::Deserialize {
            reason: error.to_string(),
        })?;
        ticket.validate()?;
        Ok(ticket)
    }

    /// Inject an additional direct address into all bootstrap peers.
    pub fn inject_direct_addr(&mut self, addr: SocketAddr) {
        for peer in &mut self.bootstrap {
            peer.inject_direct_addr(addr);
        }
    }

    fn validate(&self) -> ClusterTicketResult<()> {
        validate_cluster_id(&self.cluster_id)?;
        validate_bootstrap_count(self.bootstrap.len())?;
        for peer in &self.bootstrap {
            validate_direct_addr_count(peer.direct_addrs.len())?;
        }
        Ok(())
    }
}

impl Ticket for AspenClusterTicket {
    const KIND: &'static str = "aspen";

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("AspenClusterTicket serialization is infallible for bounded fields")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

fn direct_addrs_from_node_address(node_addr: &NodeAddress) -> Vec<SocketAddr> {
    let mut direct_addrs = Vec::new();
    for transport_addr in node_addr.transport_addrs() {
        if let NodeTransportAddr::Ip(socket_addr) = transport_addr {
            direct_addrs.push(*socket_addr);
        }
    }
    direct_addrs
}

fn direct_addr_transports(addrs: &[SocketAddr]) -> Vec<NodeTransportAddr> {
    addrs.iter().copied().map(NodeTransportAddr::Ip).collect()
}

fn ensure_bootstrap_capacity(current_len: usize) -> ClusterTicketResult<()> {
    if current_len >= MAX_BOOTSTRAP_PEERS {
        return Err(ClusterTicketError::TooManyBootstrapPeers {
            actual_peers: u32::try_from(current_len.saturating_add(1)).unwrap_or(u32::MAX),
            max_peers: u32::try_from(MAX_BOOTSTRAP_PEERS).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

fn truncate_direct_addrs(direct_addrs: &mut Vec<SocketAddr>) {
    direct_addrs.truncate(MAX_DIRECT_ADDRS_PER_PEER);
}

fn validate_bootstrap_count(count: usize) -> ClusterTicketResult<()> {
    if count > MAX_BOOTSTRAP_PEERS {
        return Err(ClusterTicketError::TooManyBootstrapPeers {
            actual_peers: u32::try_from(count).unwrap_or(u32::MAX),
            max_peers: u32::try_from(MAX_BOOTSTRAP_PEERS).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

fn validate_cluster_id(cluster_id: &str) -> ClusterTicketResult<()> {
    let actual_bytes = cluster_id.len();
    if actual_bytes > MAX_CLUSTER_ID_BYTES {
        return Err(ClusterTicketError::ClusterIdTooLong {
            actual_bytes: u32::try_from(actual_bytes).unwrap_or(u32::MAX),
            max_bytes: u32::try_from(MAX_CLUSTER_ID_BYTES).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

fn validate_direct_addr_count(count: usize) -> ClusterTicketResult<()> {
    if count > MAX_DIRECT_ADDRS_PER_PEER {
        return Err(ClusterTicketError::TooManyDirectAddrs {
            actual_addrs: u32::try_from(count).unwrap_or(u32::MAX),
            max_addrs: u32::try_from(MAX_DIRECT_ADDRS_PER_PEER).unwrap_or(u32::MAX),
        });
    }
    Ok(())
}

#[cfg(feature = "iroh")]
mod iroh_runtime {
    use alloc::collections::BTreeSet;

    use iroh_base::EndpointAddr;
    use iroh_base::PublicKey;
    use iroh_base::TransportAddr;
    use iroh_gossip::proto::TopicId;

    use super::AspenClusterTicket;
    use super::BootstrapPeer;
    use super::ClusterEndpointId;
    use super::ClusterTicketError;
    use super::ClusterTicketResult;
    use super::ClusterTopicId;
    use super::MAX_DIRECT_ADDRS_PER_PEER;

    impl From<TopicId> for ClusterTopicId {
        fn from(topic_id: TopicId) -> Self {
            Self::from_bytes(*topic_id.as_bytes())
        }
    }

    impl ClusterTopicId {
        /// Convert to an iroh-gossip topic identifier.
        pub fn to_topic_id(self) -> TopicId {
            TopicId::from_bytes(*self.as_bytes())
        }
    }

    impl From<PublicKey> for ClusterEndpointId {
        fn from(endpoint_id: PublicKey) -> Self {
            Self::new(endpoint_id.to_string())
        }
    }

    impl ClusterEndpointId {
        /// Convert to a runtime endpoint identifier.
        pub fn try_into_iroh(&self) -> ClusterTicketResult<PublicKey> {
            self.as_str()
                .parse()
                .map_err(|_| ClusterTicketError::InvalidEndpointId {
                    endpoint_id: self.as_str().to_string(),
                })
        }
    }

    impl BootstrapPeer {
        /// Create a bootstrap peer with direct addresses from an endpoint address.
        pub fn from_endpoint_addr(addr: &EndpointAddr) -> Self {
            let direct_addrs = addr
                .addrs
                .iter()
                .filter_map(|transport_addr| match transport_addr {
                    TransportAddr::Ip(socket_addr) => Some(*socket_addr),
                    _ => None,
                })
                .take(MAX_DIRECT_ADDRS_PER_PEER)
                .collect();
            Self {
                endpoint_id: ClusterEndpointId::from(addr.id),
                direct_addrs,
            }
        }

        /// Convert this bootstrap peer into a runtime endpoint address.
        pub fn try_to_endpoint_addr(&self) -> ClusterTicketResult<EndpointAddr> {
            let endpoint_id = self.endpoint_id.try_into_iroh()?;
            let addrs = self
                .direct_addrs
                .iter()
                .copied()
                .map(TransportAddr::Ip)
                .collect::<BTreeSet<_>>();
            Ok(EndpointAddr { id: endpoint_id, addrs })
        }

        /// Convert this bootstrap peer into a runtime endpoint address.
        pub fn to_endpoint_addr(&self) -> EndpointAddr {
            self.try_to_endpoint_addr()
                .expect("BootstrapPeer runtime conversion requires a valid endpoint id")
        }
    }

    impl From<&EndpointAddr> for BootstrapPeer {
        fn from(addr: &EndpointAddr) -> Self {
            Self::from_endpoint_addr(addr)
        }
    }

    impl From<PublicKey> for BootstrapPeer {
        fn from(endpoint_id: PublicKey) -> Self {
            Self::new(endpoint_id)
        }
    }

    impl AspenClusterTicket {
        /// Create a ticket with a single bootstrap peer from an endpoint address.
        pub fn with_bootstrap_addr(topic_id: TopicId, cluster_id: String, addr: &EndpointAddr) -> Self {
            let mut ticket = Self::new(topic_id, cluster_id);
            ticket.bootstrap.push(BootstrapPeer::from_endpoint_addr(addr));
            ticket
        }

        /// Create a ticket with a single bootstrap peer from an endpoint identifier.
        pub fn with_bootstrap(topic_id: TopicId, cluster_id: String, bootstrap_peer: PublicKey) -> Self {
            let mut ticket = Self::new(topic_id, cluster_id);
            ticket.bootstrap.push(BootstrapPeer::new(bootstrap_peer));
            ticket
        }

        /// Add a bootstrap peer from an endpoint address.
        pub fn add_bootstrap_addr(&mut self, addr: &EndpointAddr) -> ClusterTicketResult<()> {
            self.add_bootstrap_peer(BootstrapPeer::from_endpoint_addr(addr))
        }

        /// Add a bootstrap peer from an endpoint identifier.
        pub fn add_bootstrap(&mut self, peer: PublicKey) -> ClusterTicketResult<()> {
            self.add_bootstrap_peer(BootstrapPeer::new(peer))
        }

        /// Get all runtime endpoint addresses for direct connection.
        pub fn try_endpoint_addrs(&self) -> ClusterTicketResult<Vec<EndpointAddr>> {
            self.bootstrap.iter().map(BootstrapPeer::try_to_endpoint_addr).collect()
        }

        /// Get all runtime endpoint addresses for direct connection.
        pub fn endpoint_addrs(&self) -> Vec<EndpointAddr> {
            self.try_endpoint_addrs()
                .expect("AspenClusterTicket runtime conversion requires valid endpoint ids")
        }

        /// Get all runtime endpoint identifiers.
        pub fn try_endpoint_ids(&self) -> ClusterTicketResult<BTreeSet<PublicKey>> {
            self.bootstrap
                .iter()
                .map(|peer| peer.endpoint_id.try_into_iroh())
                .collect()
        }

        /// Get all runtime endpoint identifiers.
        pub fn endpoint_ids(&self) -> BTreeSet<PublicKey> {
            self.try_endpoint_ids()
                .expect("AspenClusterTicket runtime conversion requires valid endpoint ids")
        }
    }
}

#[cfg(all(test, not(feature = "iroh")))]
mod test_iroh_runtime {
    use alloc::collections::BTreeSet;

    use iroh::EndpointAddr;
    use iroh::EndpointId;
    use iroh::TransportAddr;
    use iroh_gossip::proto::TopicId;

    use super::AspenClusterTicket;
    use super::BootstrapPeer;
    use super::ClusterEndpointId;
    use super::ClusterTicketError;
    use super::ClusterTicketResult;
    use super::ClusterTopicId;
    use super::MAX_DIRECT_ADDRS_PER_PEER;

    impl From<TopicId> for ClusterTopicId {
        fn from(topic_id: TopicId) -> Self {
            Self::from_bytes(*topic_id.as_bytes())
        }
    }

    impl PartialEq<EndpointId> for ClusterEndpointId {
        fn eq(&self, other: &EndpointId) -> bool {
            self.as_str() == other.to_string()
        }
    }

    impl ClusterTopicId {
        pub fn to_topic_id(self) -> TopicId {
            TopicId::from_bytes(*self.as_bytes())
        }
    }

    impl From<EndpointId> for ClusterEndpointId {
        fn from(endpoint_id: EndpointId) -> Self {
            Self::new(endpoint_id.to_string())
        }
    }

    impl ClusterEndpointId {
        pub fn try_into_iroh(&self) -> ClusterTicketResult<EndpointId> {
            self.as_str()
                .parse()
                .map_err(|_| ClusterTicketError::InvalidEndpointId {
                    endpoint_id: self.as_str().to_string(),
                })
        }
    }

    impl BootstrapPeer {
        pub fn from_endpoint_addr(addr: &EndpointAddr) -> Self {
            let direct_addrs = addr
                .addrs
                .iter()
                .filter_map(|transport_addr| match transport_addr {
                    TransportAddr::Ip(socket_addr) => Some(*socket_addr),
                    _ => None,
                })
                .take(MAX_DIRECT_ADDRS_PER_PEER)
                .collect();
            Self {
                endpoint_id: ClusterEndpointId::from(addr.id),
                direct_addrs,
            }
        }

        pub fn try_to_endpoint_addr(&self) -> ClusterTicketResult<EndpointAddr> {
            let endpoint_id = self.endpoint_id.try_into_iroh()?;
            let addrs = self
                .direct_addrs
                .iter()
                .copied()
                .map(TransportAddr::Ip)
                .collect::<BTreeSet<_>>();
            Ok(EndpointAddr { id: endpoint_id, addrs })
        }

        pub fn to_endpoint_addr(&self) -> EndpointAddr {
            self.try_to_endpoint_addr()
                .expect("BootstrapPeer runtime conversion requires a valid endpoint id")
        }
    }

    impl From<&EndpointAddr> for BootstrapPeer {
        fn from(addr: &EndpointAddr) -> Self {
            Self::from_endpoint_addr(addr)
        }
    }

    impl From<EndpointId> for BootstrapPeer {
        fn from(endpoint_id: EndpointId) -> Self {
            Self::new(endpoint_id)
        }
    }

    impl AspenClusterTicket {
        pub fn with_bootstrap_addr(topic_id: TopicId, cluster_id: String, addr: &EndpointAddr) -> Self {
            let mut ticket = Self::new(topic_id, cluster_id);
            ticket.bootstrap.push(BootstrapPeer::from_endpoint_addr(addr));
            ticket
        }

        pub fn with_bootstrap(topic_id: TopicId, cluster_id: String, bootstrap_peer: EndpointId) -> Self {
            let mut ticket = Self::new(topic_id, cluster_id);
            ticket.bootstrap.push(BootstrapPeer::new(bootstrap_peer));
            ticket
        }

        pub fn add_bootstrap_addr(&mut self, addr: &EndpointAddr) -> ClusterTicketResult<()> {
            self.add_bootstrap_peer(BootstrapPeer::from_endpoint_addr(addr))
        }

        pub fn add_bootstrap(&mut self, peer: EndpointId) -> ClusterTicketResult<()> {
            self.add_bootstrap_peer(BootstrapPeer::new(peer))
        }

        pub fn try_endpoint_addrs(&self) -> ClusterTicketResult<Vec<EndpointAddr>> {
            self.bootstrap.iter().map(BootstrapPeer::try_to_endpoint_addr).collect()
        }

        pub fn endpoint_addrs(&self) -> Vec<EndpointAddr> {
            self.try_endpoint_addrs()
                .expect("AspenClusterTicket runtime conversion requires valid endpoint ids")
        }

        pub fn try_endpoint_ids(&self) -> ClusterTicketResult<BTreeSet<EndpointId>> {
            self.bootstrap
                .iter()
                .map(|peer| peer.endpoint_id.try_into_iroh())
                .collect()
        }

        pub fn endpoint_ids(&self) -> BTreeSet<EndpointId> {
            self.try_endpoint_ids()
                .expect("AspenClusterTicket runtime conversion requires valid endpoint ids")
        }
    }
}

#[cfg(test)]
mod tests {
    use core::net::SocketAddr;

    use super::AspenClusterTicket;
    use super::ClusterTopicId;
    use iroh_tickets::Ticket;

    fn make_test_ticket() -> AspenClusterTicket {
        let key = iroh::SecretKey::from([1u8; 32]);
        let addr = iroh::EndpointAddr::new(key.public());
        AspenClusterTicket::with_bootstrap_addr(ClusterTopicId::from_bytes([42u8; 32]).to_topic_id(), "test-cluster".to_string(), &addr)
    }

    #[test]
    fn to_bytes_produces_nonempty_payload() {
        let ticket = make_test_ticket();
        let bytes = <AspenClusterTicket as Ticket>::to_bytes(&ticket);
        assert!(!bytes.is_empty(), "to_bytes must not produce empty payload");
    }

    #[test]
    fn roundtrip_via_ticket_trait() {
        let ticket = make_test_ticket();
        let serialized = ticket.serialize();
        let restored = AspenClusterTicket::deserialize(&serialized).unwrap();
        assert_eq!(ticket, restored);
    }

    #[test]
    fn roundtrip_with_multiple_peers_and_addrs() {
        let mut ticket = AspenClusterTicket::new(ClusterTopicId::from_bytes([7u8; 32]), "multi-peer".to_string());
        for offset in 0..5u8 {
            let seed = [offset.saturating_add(10); 32];
            let secret_key = iroh::SecretKey::from(seed);
            let mut addr = iroh::EndpointAddr::new(secret_key.public());
            let port = 7000u16.saturating_add(u16::from(offset));
            addr.addrs.insert(iroh::TransportAddr::Ip(SocketAddr::from(([127, 0, 0, 1], port))));
            ticket.add_bootstrap_addr(&addr).unwrap();
        }
        let serialized = ticket.serialize();
        let restored = AspenClusterTicket::deserialize(&serialized).unwrap();
        assert_eq!(ticket, restored);
    }
}
