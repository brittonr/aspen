//! Madsim-based deterministic simulation testing for Aspen.
//!
//! This crate provides a centralized tester abstraction for madsim-based
//! deterministic Raft testing. `AspenRaftTester` provides a high-level API
//! that reduces test boilerplate by 80%.
//!
//! # Design Principles (Tiger Style)
//!
//! - **Bounded resources**: All operations respect MAX_PEERS, MAX_BATCH_SIZE limits
//! - **Explicit types**: Uses u64 for indices, NodeId for node identification
//! - **Fail-fast**: All errors propagate immediately via Result
//! - **Deterministic**: Environment-based seeding with fallback to test name hash
//!
//! # Key Types
//!
//! - [`AspenRaftTester`]: Main tester abstraction for simulation tests
//! - [`TesterConfig`]: Configuration for test scenarios
//! - [`BuggifyConfig`]: FoundationDB-style fault injection configuration
//! - [`LivenessConfig`]: Liveness monitoring and violation detection
//!
//! # Example
//!
//! ```ignore
//! use aspen_testing_madsim::{AspenRaftTester, TesterConfig};
//!
//! #[madsim::test]
//! async fn test_leader_crash_and_reelection() {
//!     let mut t = AspenRaftTester::new(3, "leader_crash").await;
//!
//!     let leader = t.check_one_leader().await.expect("No initial leader");
//!     t.crash_node(leader).await;
//!
//!     madsim::time::sleep(Duration::from_secs(10)).await;
//!
//!     let new_leader = t.check_one_leader().await.expect("No new leader");
//!     assert_ne!(leader, new_leader, "Same leader after crash");
//!
//!     t.end();
//! }
//! ```
//!
//! # References
//!
//! - [MadRaft Tester](https://github.com/madsim-rs/MadRaft) - Similar abstraction pattern
//! - [FoundationDB Testing](https://apple.github.io/foundationdb/testing.html) - BUGGIFY
//!   inspiration
//! - [RisingWave DST](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/)

pub mod madsim_tester;

// Re-export all public types from madsim_tester
pub use madsim_tester::AspenRaftTester;
pub use madsim_tester::BuggifyConfig;
pub use madsim_tester::BuggifyFault;
pub use madsim_tester::LivenessConfig;
pub use madsim_tester::LivenessMetrics;
pub use madsim_tester::LivenessMode;
pub use madsim_tester::LivenessReport;
pub use madsim_tester::LivenessViolation;
pub use madsim_tester::SimulationMetrics;
pub use madsim_tester::TesterConfig;
pub use madsim_tester::ViolationType;
