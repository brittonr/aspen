//! Madsim-compatible Raft network layer for deterministic simulation testing.
//!
//! This module is only available when the "testing" feature is enabled.

/// Madsim-compatible Raft network layer for deterministic simulation testing.
///
/// This module provides a deterministic network implementation for OpenRaft using madsim,
/// enabling automated detection of distributed systems bugs through deterministic simulation.
/// Unlike the production IrpcRaftNetwork (Iroh-based) and InMemoryNetwork (testing helper),
/// MadsimRaftNetwork uses madsim::net::TcpStream for fully deterministic P2P communication.
///
/// **Key Differences from Existing Network Implementations:**
///
/// 1. **IrpcRaftNetwork** (src/raft/network.rs):
///    - Production: Uses Iroh P2P networking for real distributed systems
///    - Non-deterministic: Real network I/O, timing variations, connection failures
///    - Purpose: Production consensus between physical/virtual nodes
///
/// 2. **InMemoryNetwork** (src/testing/router.rs):
///    - Testing: In-memory message passing via AspenRouter
///    - Deterministic: No real network, controlled delays/failures
///    - Limitation: Can't detect network-level bugs (reordering, partitions, etc.)
///
/// 3. **MadsimRaftNetwork** (this module):
///    - Simulation: madsim::net::TcpStream for virtual network I/O
///    - Deterministic: Reproducible with seeds, controlled time/failures
///    - Purpose: Automated bug detection (split-brain, message loss, reordering, etc.)
///
/// **Architecture:**
///
/// ```text
/// MadsimRaftRouter
///   ├─ MadsimNetworkFactory (per node)
///   │   └─ MadsimRaftNetwork (per RPC target)
///   │       └─ madsim::net::TcpStream
///   └─ FailureInjector (network/node failures)
/// ```
///
/// **Tiger Style Principles:**
/// - Bounded resources: Fixed max RPC size, connection limits
/// - Explicit types: u32/u64 for IDs, no usize
/// - Fail-fast: Errors propagated, no silent failures
mod byzantine;
mod factory;
mod failure_injection;
mod network;
mod router;
#[cfg(test)]
mod tests;

pub use byzantine::ByzantineCorruptionMode;
pub use byzantine::ByzantineFailureInjector;
pub use factory::MadsimNetworkFactory;
pub use failure_injection::FailureInjector;
pub use network::MadsimRaftNetwork;
pub use router::MadsimRaftRouter;
