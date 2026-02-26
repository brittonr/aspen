//! Blob replication policy and replica tracking.
//!
//! This module provides configurable blob replication across cluster nodes
//! with support for:
//!
//! - Configurable replication factor (1-7 replicas)
//! - Minimum replica guarantees for durability
//! - Failure domain awareness for replica placement
//! - Background repair for under-replicated blobs
//! - Quorum writes for strong consistency
//!
//! ## Architecture
//!
//! ```text
//! Blob Added (via BlobStore)
//!     |
//!     v
//! BlobReplicationManager (subscribes to events)
//!     |
//!     +-> ReplicaTracker (Raft-backed metadata)
//!     |   - Records which nodes have which blobs
//!     |   - Stored under _system:blob:replica: prefix
//!     |
//!     +-> PlacementStrategy (selects target nodes)
//!     |   - Weighted selection based on capacity
//!     |   - Failure domain spreading
//!     |
//!     +-> Async/Sync Replication Tasks
//!         - Uses iroh-blobs P2P transfer
//!         - Updates ReplicaTracker on success
//! ```
//!
//! ## Usage
//!
//! Replication is typically configured via `BlobConfig` and managed
//! automatically by the cluster. Manual control is available via
//! the `BlobReplicationManager` API.
//!
//! ```ignore
//! use aspen_blob::replication::{ReplicationPolicy, ReplicaSet};
//!
//! // Create a custom policy for important data
//! let policy = ReplicationPolicy {
//!     replication_factor: 3,
//!     min_replicas: 2,
//!     failure_domain_key: Some("rack".to_string()),
//!     enable_quorum_writes: true,
//! };
//!
//! // Check replica status
//! let replicas = manager.get_replicas(&hash).await?;
//! if replicas.len() < policy.min_replicas {
//!     manager.trigger_repair(&hash).await?;
//! }
//! ```

pub mod adapters;
pub mod manager;
pub mod topology_watcher;

// Re-export adapter types
use std::collections::BTreeSet;
use std::time::SystemTime;

pub use adapters::IrohBlobTransfer;
pub use adapters::KvReplicaMetadataStore;
use iroh_blobs::Hash;
// Re-export manager types
pub use manager::BlobReplicationManager;
pub use manager::NodeInfo;
pub use manager::PlacementStrategy;
pub use manager::ReplicaBlobTransfer;
pub use manager::ReplicaMetadataStore;
pub use manager::ReplicationConfig;
pub use manager::WeightedPlacement;
use serde::Deserialize;
use serde::Serialize;
// Re-export topology watcher
pub use topology_watcher::spawn_topology_watcher;

use crate::constants::MAX_BLOB_SIZE;

// ============================================================================
// Tiger Style Constants
// ============================================================================

/// Maximum replication factor allowed.
/// Tiger Style: Bounded to prevent excessive resource usage.
pub const MAX_REPLICATION_FACTOR: u32 = 7;

/// Minimum replication factor (must have at least one copy).
pub const MIN_REPLICATION_FACTOR: u32 = 1;

/// Maximum concurrent replication tasks per node.
/// Tiger Style: Prevents overwhelming network/disk during bulk replication.
pub const MAX_CONCURRENT_REPLICATIONS: u32 = 10;

/// Maximum blobs to repair in a single repair cycle.
/// Tiger Style: Bounds repair queue to prevent memory exhaustion.
pub const MAX_REPAIR_BATCH_SIZE: u32 = 100;

/// Minimum interval between repair cycles (seconds).
/// Tiger Style: Prevents repair storms.
pub const MIN_REPAIR_INTERVAL_SECS: u64 = 10;

/// Maximum size of serialized ReplicaSet metadata (bytes).
/// Tiger Style: Bounds per-blob metadata size.
pub const MAX_REPLICA_METADATA_SIZE: usize = 1024;

/// Key prefix for replica metadata in Raft state machine.
/// Format: _system:blob:replica:{hash_hex}
pub const REPLICA_KEY_PREFIX: &str = "_system:blob:replica:";

/// Timeout for replication transfer to a single target (seconds).
pub const REPLICATION_TRANSFER_TIMEOUT_SECS: u64 = 300;

/// Timeout for quorum write acknowledgment (seconds).
pub const QUORUM_WRITE_TIMEOUT_SECS: u64 = 30;

// ============================================================================
// Replication Policy
// ============================================================================

/// Replication policy for blobs.
///
/// Defines how many replicas to maintain and how to place them.
/// Can be set per-blob or use cluster-wide defaults from BlobConfig.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationPolicy {
    /// Target number of replicas to maintain.
    /// Must be between MIN_REPLICATION_FACTOR and MAX_REPLICATION_FACTOR.
    pub replication_factor: u32,

    /// Minimum replicas required before acknowledging writes.
    /// Only applies when enable_quorum_writes is true.
    /// Must be <= replication_factor.
    pub min_replicas: u32,

    /// Node tag key for failure domain spreading.
    /// When set, replicas are placed on nodes with different values for this tag.
    /// Example: "rack" spreads replicas across different racks.
    pub failure_domain_key: Option<String>,

    /// Whether to block writes until min_replicas are confirmed.
    /// When false, replication happens asynchronously.
    pub enable_quorum_writes: bool,
}

impl Default for ReplicationPolicy {
    fn default() -> Self {
        Self {
            replication_factor: 1,
            min_replicas: 1,
            failure_domain_key: None,
            enable_quorum_writes: false,
        }
    }
}

impl ReplicationPolicy {
    /// Create a new policy with the given replication factor.
    pub fn with_factor(replication_factor: u32) -> Self {
        Self {
            replication_factor: replication_factor.clamp(MIN_REPLICATION_FACTOR, MAX_REPLICATION_FACTOR),
            min_replicas: 1,
            ..Default::default()
        }
    }

    /// Create a quorum-based policy.
    ///
    /// Sets min_replicas to (replication_factor / 2) + 1 for majority quorum.
    pub fn quorum(replication_factor: u32) -> Self {
        let rf = replication_factor.clamp(MIN_REPLICATION_FACTOR, MAX_REPLICATION_FACTOR);
        let min = (rf / 2) + 1;
        Self {
            replication_factor: rf,
            min_replicas: min,
            enable_quorum_writes: true,
            ..Default::default()
        }
    }

    /// Set failure domain spreading.
    pub fn with_failure_domain(mut self, key: impl Into<String>) -> Self {
        self.failure_domain_key = Some(key.into());
        self
    }

    /// Validate the policy configuration.
    ///
    /// Returns an error message if the policy is invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.replication_factor < MIN_REPLICATION_FACTOR {
            return Err(format!(
                "replication_factor {} is below minimum {}",
                self.replication_factor, MIN_REPLICATION_FACTOR
            ));
        }

        if self.replication_factor > MAX_REPLICATION_FACTOR {
            return Err(format!(
                "replication_factor {} exceeds maximum {}",
                self.replication_factor, MAX_REPLICATION_FACTOR
            ));
        }

        if self.min_replicas > self.replication_factor {
            return Err(format!(
                "min_replicas {} exceeds replication_factor {}",
                self.min_replicas, self.replication_factor
            ));
        }

        if self.min_replicas < 1 {
            return Err("min_replicas must be at least 1".to_string());
        }

        Ok(())
    }
}

// ============================================================================
// Replica Set
// ============================================================================

/// Tracks which nodes have replicas of a specific blob.
///
/// This metadata is stored in the Raft state machine under the
/// `_system:blob:replica:` key prefix, ensuring linearizable tracking
/// across the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicaSet {
    /// BLAKE3 hash of the blob.
    pub hash: Hash,

    /// Size of the blob in bytes.
    #[serde(rename = "size")]
    pub size_bytes: u64,

    /// Node IDs that have confirmed replicas.
    /// Using BTreeSet for deterministic serialization order.
    pub nodes: BTreeSet<u64>,

    /// Replication policy for this blob.
    pub policy: ReplicationPolicy,

    /// Timestamp when this replica set was last updated (microseconds since epoch).
    pub updated_at_micros: u64,

    /// Timestamp when the blob was first added (microseconds since epoch).
    pub created_at_micros: u64,
}

impl ReplicaSet {
    /// Create a new replica set for a blob.
    pub fn new(hash: Hash, size_bytes: u64, policy: ReplicationPolicy, initial_node: u64) -> Self {
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0);

        let mut nodes = BTreeSet::new();
        nodes.insert(initial_node);

        Self {
            hash,
            size_bytes,
            nodes,
            policy,
            updated_at_micros: now,
            created_at_micros: now,
        }
    }

    /// Check if the replica set meets the minimum replica requirement.
    pub fn is_healthy(&self) -> bool {
        self.nodes.len() as u32 >= self.policy.min_replicas
    }

    /// Check if the replica set has reached the target replication factor.
    pub fn is_fully_replicated(&self) -> bool {
        self.nodes.len() as u32 >= self.policy.replication_factor
    }

    /// Get the number of additional replicas needed.
    pub fn replicas_needed(&self) -> u32 {
        self.policy.replication_factor.saturating_sub(self.nodes.len() as u32)
    }

    /// Add a node to the replica set.
    ///
    /// Returns true if the node was newly added.
    pub fn add_node(&mut self, node_id: u64) -> bool {
        let added = self.nodes.insert(node_id);
        if added {
            self.updated_at_micros = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_micros() as u64)
                .unwrap_or(self.updated_at_micros);
        }
        added
    }

    /// Remove a node from the replica set.
    ///
    /// Returns true if the node was present and removed.
    pub fn remove_node(&mut self, node_id: u64) -> bool {
        let removed = self.nodes.remove(&node_id);
        if removed {
            self.updated_at_micros = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_micros() as u64)
                .unwrap_or(self.updated_at_micros);
        }
        removed
    }

    /// Get the KV key for this replica set.
    pub fn kv_key(&self) -> String {
        format!("{}{}", REPLICA_KEY_PREFIX, self.hash.to_hex())
    }

    /// Parse a replica set from its KV key.
    ///
    /// Returns the hash if the key matches the replica prefix.
    pub fn parse_key(key: &str) -> Option<Hash> {
        key.strip_prefix(REPLICA_KEY_PREFIX).and_then(|hex| {
            let bytes = hex::decode(hex).ok()?;
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Some(Hash::from_bytes(arr))
            } else {
                None
            }
        })
    }

    /// Serialize to JSON for KV storage.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ============================================================================
// Replication Status
// ============================================================================

/// Status of blob replication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationStatus {
    /// Blob has zero replicas (critical - data loss imminent).
    Critical,

    /// Blob is below min_replicas (at risk of data loss).
    UnderReplicated,

    /// Blob meets min_replicas but not replication_factor.
    Degraded,

    /// Blob meets or exceeds replication_factor.
    Healthy,

    /// Over-replicated (more copies than needed).
    OverReplicated,
}

impl ReplicaSet {
    /// Get the replication status of this blob.
    pub fn status(&self) -> ReplicationStatus {
        let count = self.nodes.len() as u32;

        if count == 0 {
            ReplicationStatus::Critical
        } else if count < self.policy.min_replicas {
            ReplicationStatus::UnderReplicated
        } else if count < self.policy.replication_factor {
            ReplicationStatus::Degraded
        } else if count > self.policy.replication_factor {
            ReplicationStatus::OverReplicated
        } else {
            ReplicationStatus::Healthy
        }
    }
}

// ============================================================================
// Replication Request/Response
// ============================================================================

/// Request to replicate a blob to target nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationRequest {
    /// Hash of the blob to replicate.
    pub hash: Hash,

    /// Size of the blob in bytes (for capacity planning).
    #[serde(rename = "size")]
    pub size_bytes: u64,

    /// Target node IDs to replicate to.
    pub targets: Vec<u64>,

    /// Whether to wait for confirmation (quorum write).
    pub wait_for_ack: bool,

    /// Timeout for the replication operation (seconds).
    pub timeout_secs: u64,
}

impl ReplicationRequest {
    /// Create a new replication request.
    pub fn new(hash: Hash, size_bytes: u64, targets: Vec<u64>) -> Self {
        Self {
            hash,
            size_bytes,
            targets,
            wait_for_ack: false,
            timeout_secs: REPLICATION_TRANSFER_TIMEOUT_SECS,
        }
    }

    /// Set whether to wait for acknowledgment.
    pub fn with_ack(mut self, wait: bool) -> Self {
        self.wait_for_ack = wait;
        self
    }

    /// Validate the request.
    pub fn validate(&self) -> Result<(), String> {
        if self.size_bytes > MAX_BLOB_SIZE {
            return Err(format!("blob size {} exceeds maximum {}", self.size_bytes, MAX_BLOB_SIZE));
        }

        if self.targets.is_empty() {
            return Err("no target nodes specified".to_string());
        }

        if self.targets.len() > MAX_REPLICATION_FACTOR as usize {
            return Err(format!("too many targets {} (max {})", self.targets.len(), MAX_REPLICATION_FACTOR));
        }

        Ok(())
    }
}

/// Result of a replication operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationResult {
    /// Hash of the blob.
    pub hash: Hash,

    /// Nodes that successfully received the blob.
    pub successful: Vec<u64>,

    /// Nodes that failed to receive the blob, with error messages.
    pub failed: Vec<(u64, String)>,

    /// Total time taken for replication (milliseconds).
    pub duration_ms: u64,
}

impl ReplicationResult {
    /// Check if the replication met the minimum replica requirement.
    pub fn meets_quorum(&self, min_replicas: u32) -> bool {
        // +1 for the source node
        (self.successful.len() as u32 + 1) >= min_replicas
    }

    /// Check if all targets succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.failed.is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replication_policy_validation() {
        // Valid policy
        let policy = ReplicationPolicy::with_factor(3);
        assert!(policy.validate().is_ok());

        // Invalid: replication_factor too low
        let policy = ReplicationPolicy {
            replication_factor: 0,
            ..Default::default()
        };
        assert!(policy.validate().is_err());

        // Invalid: replication_factor too high
        let policy = ReplicationPolicy {
            replication_factor: MAX_REPLICATION_FACTOR + 1,
            ..Default::default()
        };
        assert!(policy.validate().is_err());

        // Invalid: min_replicas > replication_factor
        let policy = ReplicationPolicy {
            replication_factor: 3,
            min_replicas: 5,
            ..Default::default()
        };
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_quorum_policy() {
        let policy = ReplicationPolicy::quorum(3);
        assert_eq!(policy.replication_factor, 3);
        assert_eq!(policy.min_replicas, 2); // (3/2)+1 = 2
        assert!(policy.enable_quorum_writes);

        let policy = ReplicationPolicy::quorum(5);
        assert_eq!(policy.replication_factor, 5);
        assert_eq!(policy.min_replicas, 3); // (5/2)+1 = 3
    }

    #[test]
    fn test_replica_set_status() {
        let hash = Hash::from_bytes([0x42; 32]);
        let policy = ReplicationPolicy {
            replication_factor: 3,
            min_replicas: 2,
            ..Default::default()
        };

        let mut replicas = ReplicaSet::new(hash, 1024, policy, 1);
        assert_eq!(replicas.status(), ReplicationStatus::UnderReplicated);
        assert!(!replicas.is_healthy());

        replicas.add_node(2);
        assert_eq!(replicas.status(), ReplicationStatus::Degraded);
        assert!(replicas.is_healthy());
        assert!(!replicas.is_fully_replicated());

        replicas.add_node(3);
        assert_eq!(replicas.status(), ReplicationStatus::Healthy);
        assert!(replicas.is_fully_replicated());

        replicas.add_node(4);
        assert_eq!(replicas.status(), ReplicationStatus::OverReplicated);
    }

    #[test]
    fn test_replica_set_serialization() {
        let hash = Hash::from_bytes([0xAB; 32]);
        let policy = ReplicationPolicy::with_factor(3);
        let replicas = ReplicaSet::new(hash, 2048, policy, 1);

        let json = replicas.to_json().unwrap();
        let parsed = ReplicaSet::from_json(&json).unwrap();

        assert_eq!(replicas.hash, parsed.hash);
        assert_eq!(replicas.size_bytes, parsed.size_bytes);
        assert_eq!(replicas.nodes, parsed.nodes);
        assert_eq!(replicas.policy, parsed.policy);
    }

    #[test]
    fn test_replica_key_parsing() {
        let hash = Hash::from_bytes([0xCD; 32]);
        let policy = ReplicationPolicy::default();
        let replicas = ReplicaSet::new(hash, 512, policy, 1);

        let key = replicas.kv_key();
        assert!(key.starts_with(REPLICA_KEY_PREFIX));

        let parsed_hash = ReplicaSet::parse_key(&key).unwrap();
        assert_eq!(hash, parsed_hash);

        // Invalid key
        assert!(ReplicaSet::parse_key("invalid:key").is_none());
    }

    #[test]
    fn test_replication_request_validation() {
        let hash = Hash::from_bytes([0x11; 32]);

        // Valid request
        let req = ReplicationRequest::new(hash, 1024, vec![1, 2, 3]);
        assert!(req.validate().is_ok());

        // Invalid: no targets
        let req = ReplicationRequest::new(hash, 1024, vec![]);
        assert!(req.validate().is_err());

        // Invalid: too many targets
        let targets: Vec<u64> = (0..=MAX_REPLICATION_FACTOR as u64).collect();
        let req = ReplicationRequest::new(hash, 1024, targets);
        assert!(req.validate().is_err());

        // Invalid: blob too large
        let req = ReplicationRequest::new(hash, MAX_BLOB_SIZE + 1, vec![1]);
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_replication_result_quorum() {
        let hash = Hash::from_bytes([0x22; 32]);
        let result = ReplicationResult {
            hash,
            successful: vec![2, 3],
            failed: vec![],
            duration_ms: 100,
        };

        // Source (1) + successful (2) = 3 replicas
        assert!(result.meets_quorum(3));
        assert!(result.meets_quorum(2));
        assert!(!result.meets_quorum(4));
        assert!(result.all_succeeded());
    }
}
