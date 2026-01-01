/// Common test infrastructure module
///
/// This module provides shared test utilities, helpers, and fixtures
/// used across integration tests.

pub mod test_utils;

// Re-export commonly used items at module level
pub use test_utils::{
    create_test_endpoint_manager,
    create_test_iroh_endpoint,
    setup_test_cluster,
    setup_test_node,
    setup_test_node_with_dir,
    shutdown_cluster,
    shutdown_node,
    test_kv_data,
    test_temp_dir,
    wait_for_cluster_stability,
    wait_for_leader,
    TEST_TIMEOUT,
};