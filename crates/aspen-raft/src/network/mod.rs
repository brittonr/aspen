//! IRPC-based Raft network layer over Iroh P2P transport.
//!
//! Provides the network factory and client implementations for Raft RPC communication
//! between cluster nodes using Iroh's QUIC-based P2P networking. Includes connection
//! pooling for efficient stream multiplexing and failure detection for distinguishing
//! between actor crashes and node failures.
//!
//! # Key Components
//!
//! - `IrpcRaftNetworkFactory`: Factory for creating per-peer network clients
//! - `IrpcRaftNetwork`: Single-peer network client for sending Raft RPCs
//! - Connection pooling via `RaftConnectionPool` for stream reuse
//! - `NodeFailureDetector` integration for health monitoring
//!
//! # Tiger Style
//!
//! - Bounded peer map (MAX_PEERS) prevents Sybil attacks and memory exhaustion
//! - Explicit error handling with failure detector updates
//! - Connection pooling with idle cleanup for resource efficiency
//! - Fixed timeouts (IROH_READ_TIMEOUT) for bounded operation
//! - Size limits (MAX_RPC_MESSAGE_SIZE, MAX_SNAPSHOT_SIZE) for memory safety
//!
//! # Test Coverage
//!
//! Comprehensive unit tests (42 tests) covering:
//! - Factory creation, configuration, and peer management
//! - Client creation (new_client, new_client_for_shard)
//! - Connection pooling and peer address handling
//! - RPC serialization (Vote, AppendEntries, Snapshot)
//! - Error classification (NodeCrash, ActorCrash, Unreachable)
//! - Failure detector integration
//!
//! Full cluster-level testing via madsim simulation tests
//!
//! # Architecture
//!
//! ```text
//! RaftNode
//!    |
//!    v
//! IrpcRaftNetworkFactory
//!    |
//!    +---> IrpcRaftNetwork (peer 1) ---> RaftConnectionPool ---> Iroh QUIC
//!    +---> IrpcRaftNetwork (peer 2) ---> RaftConnectionPool ---> Iroh QUIC
//!    +---> IrpcRaftNetwork (peer 3) ---> RaftConnectionPool ---> Iroh QUIC
//! ```
//!
//! # Example
//!
//! ```ignore
//! use aspen::raft::network::IrpcRaftNetworkFactory;
//!
//! // With authentication enabled (production)
//! let factory = IrpcRaftNetworkFactory::new(endpoint_manager, peer_addrs, true);
//!
//! // Without authentication (legacy/testing)
//! let factory = IrpcRaftNetworkFactory::new(endpoint_manager, peer_addrs, false);
//!
//! // Factory is passed to openraft::Raft::new() for network creation
//! ```

mod client;
mod factory;
#[cfg(test)]
mod tests;

pub use client::IrpcRaftNetwork;
pub use factory::IrpcRaftNetworkFactory;

use crate::node_failure_detection::ConnectionStatus;
use crate::types::NodeId;

/// Update message for the failure detector.
///
/// Tiger Style: Bounded channel prevents unbounded task spawning.
/// Instead of spawning a new task for each failure update, we send
/// through a bounded channel and let a single consumer process updates.
#[derive(Debug)]
pub(crate) struct FailureDetectorUpdate {
    pub(crate) node_id: NodeId,
    pub(crate) raft_status: ConnectionStatus,
    pub(crate) iroh_status: ConnectionStatus,
}
