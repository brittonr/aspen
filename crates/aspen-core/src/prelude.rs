//! Common imports for Aspen core types.
//!
//! This prelude provides a curated subset of the most commonly used types
//! and traits for working with Aspen's distributed systems primitives.
//!
//! # Usage
//!
//! ```
//! use aspen_core::prelude::*;
//! ```

// Core traits
// Essential types
pub use crate::cluster::ClusterNode;
pub use crate::cluster::ClusterState;
// Common constants
pub use crate::constants::api::DEFAULT_SCAN_LIMIT;
pub use crate::constants::api::MAX_KEY_SIZE;
pub use crate::constants::api::MAX_SCAN_RESULTS;
pub use crate::constants::api::MAX_VALUE_SIZE;
// Error types
pub use crate::error::ControlPlaneError;
pub use crate::error::KeyValueStoreError;
// KV operations
pub use crate::kv::DeleteRequest;
pub use crate::kv::DeleteResult;
pub use crate::kv::KeyValueWithRevision;
pub use crate::kv::ReadRequest;
pub use crate::kv::ReadResult;
pub use crate::kv::ScanRequest;
pub use crate::kv::ScanResult;
pub use crate::kv::WriteCommand;
pub use crate::kv::WriteRequest;
pub use crate::kv::WriteResult;
pub use crate::traits::ClusterController;
pub use crate::traits::CoordinationBackend;
pub use crate::traits::KeyValueStore;
pub use crate::types::ClusterMetrics;
pub use crate::types::NodeAddress;
pub use crate::types::NodeId;
pub use crate::types::NodeState;
