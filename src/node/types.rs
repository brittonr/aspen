//! Type re-exports for the node module.
//!
//! This module re-exports types from `crate::raft::types` for convenience,
//! providing a single canonical `NodeId` type used throughout the codebase.
//!
//! # Design Rationale
//!
//! The `NodeId` type is defined in `raft::types` as it's primarily used by
//! the Raft consensus layer. This module re-exports it so that the node
//! module API can reference it without requiring users to import from `raft`.
//!
//! # Example
//!
//! ```ignore
//! use aspen::node::NodeId;
//!
//! let node_id = NodeId::new(1);
//! let node_id: NodeId = 42.into();
//! let node_id: NodeId = "123".parse()?;
//! ```

// Re-export NodeId from raft::types to provide a single canonical definition.
// This eliminates the duplicate type that was previously defined here.
pub use crate::raft::types::NodeId;
