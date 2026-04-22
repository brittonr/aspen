//! Raft membership metadata types.
//!
//! This module defines the transport-neutral node metadata stored in Raft's
//! membership set.

use core::fmt;

use aspen_core::NodeAddress;
use serde::Deserialize;
use serde::Serialize;

/// Stable parseable endpoint id used by [`RaftMemberInfo::default`].
///
/// This is the iroh public key string derived from the historical zero-seed
/// default (`SecretKey::from([0u8; 32]).public()`). Keeping it parseable avoids
/// surprising runtime behavior in watchers and relay/bootstrap helpers that
/// still validate endpoint ids even for `Default` test helper nodes.
const DEFAULT_MEMBER_ENDPOINT_ID: &str =
    "3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29";

/// Raft membership metadata containing transport-neutral connection information.
///
/// This type is stored in Raft's membership set alongside each `NodeId`. Unlike
/// openraft's `BasicNode` which stores a simple address string, `RaftMemberInfo`
/// stores Aspen's alloc-safe [`NodeAddress`] plus optional relay metadata.
/// Runtime crates convert that address into concrete iroh endpoint data at the
/// shell boundary when they need to open network connections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RaftMemberInfo {
    /// The alloc-safe node address for connecting to this member.
    pub node_addr: NodeAddress,
    /// Optional cluster-internal relay server URL for this node.
    ///
    /// When a node runs its own relay server, this field stores the URL
    /// (for example, `"https://relay.example.com"`) so other cluster members
    /// can use it for NAT traversal and P2P connectivity.
    pub relay_url: Option<String>,
}

impl RaftMemberInfo {
    /// Creates a new `RaftMemberInfo` with the given transport-neutral node address.
    pub fn new(node_addr: NodeAddress) -> Self {
        Self {
            node_addr,
            relay_url: None,
        }
    }

    /// Creates a new `RaftMemberInfo` with a node address and relay URL.
    pub fn with_relay(node_addr: NodeAddress, relay_url: String) -> Self {
        Self {
            node_addr,
            relay_url: Some(relay_url),
        }
    }

    /// Borrow this member's stable endpoint identifier string.
    pub fn endpoint_id(&self) -> &str {
        self.node_addr.endpoint_id()
    }

    /// Count the stored transport addresses attached to this member.
    pub fn transport_addr_count(&self) -> usize {
        self.node_addr.transport_addrs().count()
    }
}

impl Default for RaftMemberInfo {
    /// Creates a placeholder `RaftMemberInfo` used by test helpers that require `Default`.
    fn default() -> Self {
        Self {
            node_addr: NodeAddress::from_parts(DEFAULT_MEMBER_ENDPOINT_ID, Vec::new()),
            relay_url: None,
        }
    }
}

impl fmt::Display for RaftMemberInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RaftMemberInfo({})", self.endpoint_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_addr(endpoint_id: &str) -> NodeAddress {
        NodeAddress::from_parts(endpoint_id, Vec::new())
    }

    #[test]
    fn test_raft_member_info_default() {
        let info = RaftMemberInfo::default();
        assert_eq!(info.endpoint_id(), DEFAULT_MEMBER_ENDPOINT_ID);
        assert_eq!(info.transport_addr_count(), 0);
        assert!(format!("{}", info).starts_with("RaftMemberInfo("));
    }

    #[test]
    fn test_raft_member_info_display() {
        let info = RaftMemberInfo::default();
        let display = format!("{}", info);
        assert!(display.contains("RaftMemberInfo("));
        assert!(display.contains(DEFAULT_MEMBER_ENDPOINT_ID));
    }

    #[test]
    fn test_raft_member_info_equality() {
        let info1 = RaftMemberInfo::default();
        let info2 = RaftMemberInfo::default();
        assert_eq!(info1, info2);
    }

    #[test]
    fn test_raft_member_info_new() {
        let info = RaftMemberInfo::new(test_node_addr("node-1"));
        assert_eq!(info.endpoint_id(), "node-1");
        assert_eq!(info.transport_addr_count(), 0);
    }

    #[test]
    fn test_raft_member_info_with_relay() {
        let info = RaftMemberInfo::with_relay(test_node_addr("node-2"), "https://relay.test".to_string());
        assert_eq!(info.endpoint_id(), "node-2");
        assert_eq!(info.relay_url.as_deref(), Some("https://relay.test"));
    }

    #[test]
    fn test_raft_member_info_serde_roundtrip() {
        let original = RaftMemberInfo::with_relay(test_node_addr("node-3"), "https://relay.test".to_string());
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
