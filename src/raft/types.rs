//! Type definitions for Raft consensus configuration.
//!
//! This module defines the core type configuration for openraft, specifying
//! the concrete types used for nodes, requests, responses, and storage.
//!
//! # Type Configuration
//!
//! - **NodeId**: Newtype wrapper around `u64` for type-safe node identification
//! - **Node**: `RaftMemberInfo` - Raft membership metadata with Iroh P2P addresses
//! - **AppRequest**: Application-level write commands (Set, SetMulti)
//! - **AppResponse**: Application-level read/write responses
//!
//! # Tiger Style
//!
//! - Explicitly sized types: `u64` for NodeId (not usize for portability)
//! - Newtype pattern: Prevents accidental mixing with log indices, terms, ports
//! - Bounded operations: SetMulti limited by MAX_SETMULTI_KEYS constant

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use iroh::EndpointAddr;
use openraft::declare_raft_types;
use serde::{Deserialize, Serialize};

/// Type-safe node identifier for Raft cluster nodes.
///
/// This newtype wrapper around `u64` prevents accidental mixing with other
/// numeric types like log indices, term numbers, or port numbers. The compiler
/// enforces correct usage at type-check time rather than runtime.
///
/// # Tiger Style
///
/// - Zero overhead: Compiles to bare `u64`, no runtime cost
/// - Ergonomic conversions: `From`/`Into` traits for seamless integration
/// - String parsing: `FromStr` with explicit error handling
/// - Ordering: Derived `PartialOrd`/`Ord` for deterministic sorting
///
/// # Example
///
/// ```ignore
/// use aspen::raft::types::NodeId;
///
/// let node_id = NodeId::new(1);
/// let node_id: NodeId = 42.into();
/// let node_id: NodeId = "123".parse()?;
/// let raw: u64 = node_id.into();
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Create a new `NodeId` from a raw `u64`.
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl FromStr for NodeId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u64>().map(NodeId)
    }
}

/// Raft membership metadata containing Iroh P2P connection information.
///
/// This type is stored in Raft's membership set alongside each `NodeId`. Unlike
/// openraft's `BasicNode` which stores a simple address string, `RaftMemberInfo`
/// stores the full Iroh `EndpointAddr` containing:
/// - Endpoint ID (public key identifier)
/// - Relay URLs for NAT traversal
/// - Direct socket addresses
///
/// This enables peer addresses to be replicated via Raft consensus,
/// persisted in the state machine, and recovered on restart without
/// requiring gossip rediscovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftMemberInfo {
    /// The Iroh endpoint address for connecting to this node.
    pub iroh_addr: EndpointAddr,
}

impl RaftMemberInfo {
    /// Creates a new `RaftMemberInfo` with the given Iroh endpoint address.
    pub fn new(iroh_addr: EndpointAddr) -> Self {
        Self { iroh_addr }
    }
}

impl Default for RaftMemberInfo {
    /// Creates a default `RaftMemberInfo` with a zero endpoint ID.
    ///
    /// This is primarily used by testing utilities (e.g., openraft's `membership_ent`)
    /// that require nodes to implement `Default`. In production, use `RaftMemberInfo::new()`
    /// or `create_test_raft_member_info()` to create nodes with proper endpoint addresses.
    fn default() -> Self {
        use iroh::{EndpointId, SecretKey};

        let seed = [0u8; 32];
        let secret_key = SecretKey::from(seed);
        let endpoint_id: EndpointId = secret_key.public();

        Self {
            iroh_addr: EndpointAddr::new(endpoint_id),
        }
    }
}

impl fmt::Display for RaftMemberInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RaftMemberInfo({})", self.iroh_addr.id)
    }
}

/// Application-level requests replicated through Raft.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AppRequest {
    Set { key: String, value: String },
    SetMulti { pairs: Vec<(String, String)> },
    Delete { key: String },
    DeleteMulti { keys: Vec<String> },
}

impl fmt::Display for AppRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppRequest::Set { key, value } => write!(f, "Set {{ key: {key}, value: {value} }}"),
            AppRequest::SetMulti { pairs } => {
                write!(f, "SetMulti {{ pairs: [")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "({k}, {v})")?;
                }
                write!(f, "] }}")
            }
            AppRequest::Delete { key } => write!(f, "Delete {{ key: {key} }}"),
            AppRequest::DeleteMulti { keys } => {
                write!(f, "DeleteMulti {{ keys: [")?;
                for (i, k) in keys.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}")?;
                }
                write!(f, "] }}")
            }
        }
    }
}

/// Response returned to HTTP clients after applying a request.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppResponse {
    pub value: Option<String>,
    /// Indicates whether a delete operation actually removed a key.
    /// None for operations where deletion is not applicable.
    pub deleted: Option<bool>,
}

declare_raft_types!(
    /// Declare type config used by Aspen's embedded Raft node.
    pub AppTypeConfig:
        D = AppRequest,
        R = AppResponse,
        NodeId = NodeId,
        Node = RaftMemberInfo,
);
