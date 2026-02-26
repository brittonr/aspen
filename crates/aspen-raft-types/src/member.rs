//! Raft membership metadata types.
//!
//! This module defines the node metadata stored in Raft's membership set.

use std::fmt;

use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh::SecretKey;
use serde::Deserialize;
use serde::Serialize;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raft_member_info_default() {
        let info = RaftMemberInfo::default();
        // Default should create a valid RaftMemberInfo with a zero-seed endpoint
        assert!(format!("{}", info).starts_with("RaftMemberInfo("));
    }

    #[test]
    fn test_raft_member_info_display() {
        let info = RaftMemberInfo::default();
        let display = format!("{}", info);
        // Should contain "RaftMemberInfo(" prefix
        assert!(display.contains("RaftMemberInfo("));
    }

    #[test]
    fn test_raft_member_info_equality() {
        let info1 = RaftMemberInfo::default();
        let info2 = RaftMemberInfo::default();
        // Two defaults with same seed should be equal
        assert_eq!(info1, info2);
    }

    #[test]
    fn test_raft_member_info_new() {
        let seed = [1u8; 32];
        let secret_key = SecretKey::from(seed);
        let endpoint_id: EndpointId = secret_key.public();
        let addr = EndpointAddr::new(endpoint_id);

        let info = RaftMemberInfo::new(addr.clone());
        assert_eq!(info.iroh_addr.id, endpoint_id);
    }

    #[test]
    fn test_raft_member_info_serde_roundtrip() {
        let original = RaftMemberInfo::default();
        let json = serde_json::to_string(&original).expect("serialize");
        let deserialized: RaftMemberInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_raft_member_info_clone() {
        let original = RaftMemberInfo::default();
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }
}
