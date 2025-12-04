/// Testing infrastructure for Aspen distributed system tests.
///
/// This module provides deterministic testing primitives for multi-node Raft clusters,
/// enabling fast, reliable tests without real network I/O or timing dependencies.
///
/// ## Key Components
///
/// - `AspenRouter`: Manages multiple in-memory Raft nodes with simulated networking
/// - Wait helpers: Metrics-based assertions via OpenRaft's `Wait` API
/// - Network simulation: Configurable delays, failures, and partitions
///
/// ## Usage Pattern
///
/// ```ignore
/// let config = Arc::new(Config::default().validate()?);
/// let mut router = AspenRouter::new(config);
///
/// router.new_raft_node(0).await;
/// router.new_raft_node(1).await;
/// router.new_raft_node(2).await;
///
/// let node0 = router.get_raft_handle(&0)?;
/// node0.initialize(btreeset! {0,1,2}).await?;
///
/// // Use wait helpers instead of sleep
/// router.wait(&0, timeout()).applied_index(Some(1), "initialized").await?;
/// router.wait(&0, timeout()).current_leader(Some(0), "leader elected").await?;
/// ```
pub mod router;

pub use router::AspenRouter;
