//! Cluster management RPC handler for Aspen.
//!
//! This crate provides the ClusterHandler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles cluster management operations:
//! - InitCluster: Initialize a single-node cluster
//! - AddLearner: Add a learner node to the cluster
//! - ChangeMembership: Change cluster membership
//! - PromoteLearner: Promote a learner to voter
//! - TriggerSnapshot: Trigger a Raft snapshot
//! - GetClusterState: Get current cluster state
//! - GetClusterTicket: Generate a cluster join ticket
//! - AddPeer: Add a peer to the network factory
//! - GetClusterTicketCombined: Generate a cluster ticket with multiple bootstrap peers
//! - GetClientTicket: Generate a client access ticket
//! - GetDocsTicket: Generate a docs sync ticket
//! - GetTopology: Get cluster topology information

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::ClusterHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `ClusterHandler` instances.
///
/// Priority 120 (core handler range: 100-199).
pub struct ClusterHandlerFactory;

impl ClusterHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for ClusterHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for ClusterHandlerFactory {
    fn create(&self, _ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        Some(Arc::new(ClusterHandler))
    }

    fn name(&self) -> &'static str {
        "ClusterHandler"
    }

    fn priority(&self) -> u32 {
        120
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(ClusterHandlerFactory);
