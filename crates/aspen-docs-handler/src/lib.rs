//! Docs/Sync RPC handler for Aspen.
//!
//! This crate provides the DocsHandler extracted from aspen-rpc-handlers
//! for better modularity and faster incremental builds.
//!
//! Handles docs operations:
//! - DocsSet: Set a document entry
//! - DocsGet: Get a document entry
//! - DocsDelete: Delete a document entry
//! - DocsList: List document entries
//! - DocsStatus: Get sync status
//!
//! Handles peer cluster operations:
//! - AddPeerCluster: Add a peer cluster for sync
//! - RemovePeerCluster: Remove a peer cluster
//! - ListPeerClusters: List all peer clusters
//! - GetPeerClusterStatus: Get status of a peer cluster
//! - UpdatePeerClusterFilter: Update sync filter
//! - UpdatePeerClusterPriority: Update sync priority
//! - SetPeerClusterEnabled: Enable/disable sync
//! - GetKeyOrigin: Get origin cluster for a key

mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use handler::DocsHandler;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `DocsHandler` instances.
///
/// Only creates handler if docs_sync or peer_manager is available.
/// Priority 530 (feature handler range: 500-599).
pub struct DocsHandlerFactory;

impl DocsHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for DocsHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for DocsHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // Only create handler if docs_sync or peer_manager is available
        if ctx.docs_sync.is_some() || ctx.peer_manager.is_some() {
            Some(Arc::new(DocsHandler))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "DocsHandler"
    }

    fn priority(&self) -> u32 {
        530
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(DocsHandlerFactory);
