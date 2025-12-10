//! Type definitions for Raft consensus configuration.
//!
//! This module defines the core type configuration for openraft, specifying
//! the concrete types used for nodes, requests, responses, and storage.
//!
//! # Type Configuration
//!
//! - **NodeId**: `u64` - Unique identifier for cluster nodes
//! - **Node**: `AspenNode` - Custom node type storing Iroh P2P addresses
//! - **AppRequest**: Application-level write commands (Set, SetMulti)
//! - **AppResponse**: Application-level read/write responses
//!
//! # Tiger Style
//!
//! - Explicitly sized types: `u64` for NodeId (not usize for portability)
//! - Bounded operations: SetMulti limited by MAX_SETMULTI_KEYS constant

use std::fmt;

use iroh::EndpointAddr;
use openraft::declare_raft_types;
use serde::{Deserialize, Serialize};

pub type NodeId = u64;

/// Custom node type for Aspen that stores Iroh P2P addresses.
///
/// Unlike `BasicNode` which stores a simple address string, `AspenNode`
/// stores the full Iroh `EndpointAddr` containing:
/// - Endpoint ID (public key identifier)
/// - Relay URLs for NAT traversal
/// - Direct socket addresses
///
/// This enables peer addresses to be replicated via Raft consensus,
/// persisted in the state machine, and recovered on restart without
/// requiring gossip rediscovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspenNode {
    /// The Iroh endpoint address for connecting to this node.
    pub iroh_addr: EndpointAddr,
}

impl AspenNode {
    /// Creates a new `AspenNode` with the given Iroh endpoint address.
    pub fn new(iroh_addr: EndpointAddr) -> Self {
        Self { iroh_addr }
    }
}

impl Default for AspenNode {
    /// Creates a default `AspenNode` with a zero endpoint ID.
    ///
    /// This is primarily used by testing utilities (e.g., openraft's `membership_ent`)
    /// that require nodes to implement `Default`. In production, use `AspenNode::new()`
    /// or `create_test_aspen_node()` to create nodes with proper endpoint addresses.
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

impl fmt::Display for AspenNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AspenNode({})", self.iroh_addr.id)
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
}

declare_raft_types!(
    /// Declare type config used by Aspen's embedded Raft node.
    pub AppTypeConfig:
        D = AppRequest,
        R = AppResponse,
        NodeId = NodeId,
        Node = AspenNode,
);
