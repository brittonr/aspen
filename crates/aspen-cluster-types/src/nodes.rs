use serde::Deserialize;
use serde::Serialize;

// ============================================================================
// NodeAddress - P2P endpoint address wrapper
// ============================================================================

/// P2P endpoint address for connecting to a node.
///
/// This type wraps `iroh::EndpointAddr` to decouple the public API from the
/// underlying iroh implementation. It provides the same functionality while
/// allowing internal implementation changes without breaking the public API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeAddress(iroh::EndpointAddr);

impl NodeAddress {
    /// Create a new NodeAddress from an iroh EndpointAddr.
    pub fn new(addr: iroh::EndpointAddr) -> Self {
        Self(addr)
    }

    /// Get the node's public key ID as a string.
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }

    /// Get a reference to the underlying iroh EndpointAddr.
    pub fn inner(&self) -> &iroh::EndpointAddr {
        &self.0
    }
}

impl From<iroh::EndpointAddr> for NodeAddress {
    fn from(addr: iroh::EndpointAddr) -> Self {
        Self(addr)
    }
}

impl From<NodeAddress> for iroh::EndpointAddr {
    fn from(addr: NodeAddress) -> Self {
        addr.0
    }
}

impl std::fmt::Display for NodeAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.id)
    }
}

// ============================================================================
// NodeState - Raft node state wrapper
// ============================================================================

/// The current state of a node in the Raft cluster.
///
/// This is an API-owned enum that abstracts away the underlying openraft
/// implementation details, providing a stable public interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeState {
    /// A learner node that replicates data but does not participate in voting.
    Learner,
    /// A voting follower that replicates the leader's log.
    Follower,
    /// A node attempting to become leader through an election.
    Candidate,
    /// The elected leader that handles all client requests.
    Leader,
    /// The node is shutting down.
    Shutdown,
}

impl NodeState {
    /// Returns true if this node is the leader.
    #[must_use]
    pub fn is_leader(&self) -> bool {
        matches!(self, Self::Leader)
    }

    /// Returns true if this node can accept reads (leader or follower with ReadIndex).
    #[must_use]
    pub fn can_serve_reads(&self) -> bool {
        matches!(self, Self::Leader | Self::Follower)
    }

    /// Returns true if this node is healthy (not shutdown).
    #[must_use]
    pub fn is_healthy(&self) -> bool {
        !matches!(self, Self::Shutdown)
    }

    /// Convert to a numeric value for metrics/serialization.
    #[must_use]
    pub fn as_u8(&self) -> u8 {
        match self {
            Self::Learner => 0,
            Self::Follower => 1,
            Self::Candidate => 2,
            Self::Leader => 3,
            Self::Shutdown => 4,
        }
    }
}

// ============================================================================
// NodeId - Type-safe node identifier
// ============================================================================

/// Type-safe node identifier for Raft cluster nodes.
///
/// This newtype wrapper around `u64` prevents accidental mixing with other
/// numeric types like log indices, term numbers, or port numbers.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Default,
    Serialize,
    Deserialize
)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Create a new `NodeId` from a raw `u64`.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for NodeId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<NodeId> for u64 {
    fn from(value: NodeId) -> Self {
        value.0
    }
}

impl std::str::FromStr for NodeId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(NodeId)
    }
}

// ============================================================================
// ClusterNode - Node participating in the control-plane cluster
// ============================================================================

/// Describes a node participating in the control-plane cluster.
///
/// Contains both the node's identifier and its P2P endpoint address,
/// which is stored in Raft membership state for persistent discovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterNode {
    /// Unique identifier for this node within the cluster.
    pub id: u64,
    /// Display address for logging and human-readable output.
    pub addr: String,
    /// Optional legacy Raft address (host:port) for backwards compatibility.
    pub raft_addr: Option<String>,
    /// P2P endpoint address for connecting to this node.
    pub node_addr: Option<NodeAddress>,
}

impl ClusterNode {
    /// Create a new ClusterNode with a simple string address (legacy).
    pub fn new(id: u64, addr: impl Into<String>, raft_addr: Option<String>) -> Self {
        Self {
            id,
            addr: addr.into(),
            raft_addr,
            node_addr: None,
        }
    }

    /// Create a new ClusterNode with a P2P endpoint address.
    pub fn with_node_addr(id: u64, node_addr: NodeAddress) -> Self {
        Self {
            id,
            addr: node_addr.id(),
            raft_addr: None,
            node_addr: Some(node_addr),
        }
    }

    /// Create a ClusterNode with an iroh EndpointAddr.
    pub fn with_iroh_addr(id: u64, iroh_addr: iroh::EndpointAddr) -> Self {
        Self::with_node_addr(id, NodeAddress::new(iroh_addr))
    }

    /// Get the node address as an iroh EndpointAddr, if available.
    pub fn iroh_addr(&self) -> Option<&iroh::EndpointAddr> {
        self.node_addr.as_ref().map(|addr| addr.inner())
    }
}
