/// Common test infrastructure module
///
/// This module provides shared test utilities, helpers, and fixtures
/// used across integration tests.
pub mod test_utils;

// Re-export commonly used items at module level
pub use test_utils::TEST_TIMEOUT;
pub use test_utils::create_test_endpoint_manager;
pub use test_utils::create_test_iroh_endpoint;
pub use test_utils::setup_test_cluster;
pub use test_utils::setup_test_node;
pub use test_utils::setup_test_node_with_dir;
pub use test_utils::shutdown_cluster;
pub use test_utils::shutdown_node;
pub use test_utils::test_kv_data;
pub use test_utils::test_temp_dir;
pub use test_utils::wait_for_cluster_stability;
pub use test_utils::wait_for_leader;
