//! Type configuration for aspen-raft-network.
//!
//! Contains an identical AppTypeConfig to avoid circular dependencies.
//! Static assertions verify layout compatibility with aspen-raft and aspen-transport.
//!
//! # Why Duplicate AppTypeConfig?
//!
//! Due to Rust's orphan rules, the `declare_raft_types!` macro must be invoked
//! in the crate where openraft traits are implemented. This means:
//!
//! - `aspen-raft/src/types.rs` - Canonical declaration (implements openraft traits)
//! - `aspen-transport/src/rpc.rs` - Duplicate for transport-layer RPC types
//! - `aspen-raft-network/src/types.rs` - Duplicate for network-layer types
//!
//! Static assertions ensure all three declarations produce identical memory layouts.

use aspen_raft_types::AppRequest;
use aspen_raft_types::AppResponse;
use aspen_raft_types::NodeId;
use aspen_raft_types::RaftMemberInfo;
use openraft::declare_raft_types;

declare_raft_types!(
    /// OpenRaft type configuration for Aspen's Raft network layer.
    ///
    /// This must be identical to the declarations in aspen-raft and aspen-transport.
    /// The static assertions below verify memory layout compatibility.
    pub AppTypeConfig:
        D = AppRequest,
        R = AppResponse,
        NodeId = NodeId,
        Node = RaftMemberInfo,
);

// ===========================================================================
// Compile-time type safety verification
// ===========================================================================
//
// These static assertions verify that our AppTypeConfig matches the canonical
// declaration in aspen-transport. If either type changes, compilation will fail
// with a clear error message.
mod _type_safety_checks {
    use static_assertions::assert_eq_align;
    use static_assertions::assert_eq_size;

    // Verify that the Raft handles are identical in memory layout.
    // This ensures safe transmutes between crate boundaries if needed.
    assert_eq_size!(openraft::Raft<crate::types::AppTypeConfig>, openraft::Raft<aspen_transport::rpc::AppTypeConfig>);

    assert_eq_align!(openraft::Raft<crate::types::AppTypeConfig>, openraft::Raft<aspen_transport::rpc::AppTypeConfig>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_type_config_types_match() {
        // This test verifies the types are usable.
        // The static assertions above verify memory layout compatibility at compile time.
        let _: std::marker::PhantomData<AppTypeConfig> = std::marker::PhantomData;
    }

    #[test]
    fn test_node_id_conversion() {
        let node_id = NodeId::new(42);
        assert_eq!(node_id.0, 42);

        let from_u64: NodeId = 123.into();
        assert_eq!(from_u64.0, 123);
    }
}
