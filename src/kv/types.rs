//! Type definitions for the key-value service.
//!
//! Defines the core types used throughout the KV module, primarily the NodeId
//! newtype wrapper for type-safe node identification. This module is intentionally
//! minimal, providing strong typing without coupling to specific implementations.
//!
//! # Key Components
//!
//! - `NodeId`: Newtype wrapper around u64 for Raft node identifiers
//! - Trait implementations: Display, FromStr, From conversions for ergonomic use
//! - Serialization: Serde support for configuration and network serialization
//!
//! # Design Rationale
//!
//! NodeId is a newtype (not a type alias) to prevent accidental mixing with other
//! u64 values like port numbers, log indices, or term numbers. The compiler enforces
//! correct usage at type-check time rather than runtime, eliminating a class of bugs.
//!
//! # Tiger Style
//!
//! - Explicit types: u64 wrapper instead of bare integers (prevents confusion)
//! - Zero overhead: Newtype compiles to u64, no runtime cost
//! - Ergonomic conversions: From/Into traits for seamless integration
//! - String parsing: FromStr with explicit error handling (ParseIntError)
//! - Ordering: Derived PartialOrd/Ord for deterministic sorting
//!
//! # Example
//!
//! ```ignore
//! use aspen::kv::types::NodeId;
//!
//! // Type-safe construction
//! let node_id = NodeId::new(1);
//! let node_id: NodeId = 42.into();
//!
//! // String parsing
//! let node_id: NodeId = "123".parse()?;
//!
//! // Display
//! println!("Node: {}", node_id); // "Node: 123"
//!
//! // Extract inner value
//! let raw: u64 = node_id.into();
//! ```

use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Node identifier used throughout the KV surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NodeId(pub u64);

impl NodeId {
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
