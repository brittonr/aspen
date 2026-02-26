//! Blob storage RPC handler for Aspen.
//!
//! This crate provides the blob handler extracted from
//! aspen-rpc-handlers for better modularity and faster incremental builds.
//!
//! Handles blob storage operations:
//! - AddBlob, GetBlob, HasBlob: Basic blob operations
//! - GetBlobTicket, ListBlobs: Blob discovery
//! - ProtectBlob, UnprotectBlob, DeleteBlob: Blob lifecycle
//! - DownloadBlob, DownloadBlobByHash, DownloadBlobByProvider: Remote blob retrieval
//! - GetBlobStatus: Blob status inspection
//! - BlobReplicatePull, GetBlobReplicationStatus, TriggerBlobReplication: Replication
//! - RunBlobRepairCycle: Cluster-wide blob repair

mod executor;
pub(crate) mod handler;

use std::sync::Arc;

// Re-export core types for convenience
pub use aspen_rpc_core::ClientProtocolContext;
pub use aspen_rpc_core::HandlerFactory;
pub use aspen_rpc_core::RequestHandler;
pub use aspen_rpc_core::ServiceHandler;
pub use executor::BlobServiceExecutor;

// =============================================================================
// Handler Factory (Plugin Registration)
// =============================================================================

/// Factory for creating `BlobHandler` instances.
///
/// This factory enables plugin-style registration via the `inventory` crate.
/// The handler is only created if the `blob_store` is available in the context.
///
/// # Priority
///
/// Priority 520 (feature handler range: 500-599).
pub struct BlobHandlerFactory;

impl BlobHandlerFactory {
    /// Create a new factory instance.
    pub const fn new() -> Self {
        Self
    }
}

impl Default for BlobHandlerFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl HandlerFactory for BlobHandlerFactory {
    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // Only create handler if blob store is configured
        if ctx.blob_store.is_some() {
            let executor = Arc::new(BlobServiceExecutor::new(ctx.clone()));
            Some(Arc::new(ServiceHandler::new(executor)))
        } else {
            None
        }
    }

    fn name(&self) -> &'static str {
        "BlobHandler"
    }

    fn priority(&self) -> u32 {
        520
    }
}

// Self-register via inventory
aspen_rpc_core::submit_handler_factory!(BlobHandlerFactory);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factory_name() {
        let factory = BlobHandlerFactory;
        assert_eq!(factory.name(), "BlobHandler");
    }

    #[test]
    fn test_factory_priority() {
        let factory = BlobHandlerFactory;
        assert_eq!(factory.priority(), 520);
    }
}
