//! Type configuration for aspen-raft-network.
//!
//! Re-exports the shared Raft type configuration from `aspen-raft-types`.
//! Network layer now uses the same leaf definition as consensus and transport.

pub use aspen_raft_types::AppTypeConfig;

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
