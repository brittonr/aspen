//! Blob service executor for typed RPC dispatch.
//!
//! Wraps the existing blob handler functions as a `ServiceExecutor`,
//! passing captured dependencies instead of the full `ClientProtocolContext`.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ServiceExecutor;
use async_trait::async_trait;

/// Service executor for blob storage operations.
///
/// Captures blob-related dependencies at construction time.
/// Delegates to the existing handler functions via a minimal context.
pub struct BlobServiceExecutor {
    /// Minimal context holding only the deps blob operations need.
    pub(crate) ctx: aspen_rpc_core::ClientProtocolContext,
}

impl BlobServiceExecutor {
    /// Create a new blob service executor using a pre-built context.
    ///
    /// The context should have `blob_store` set; other fields may be None.
    pub fn new(ctx: aspen_rpc_core::ClientProtocolContext) -> Self {
        Self { ctx }
    }
}

#[async_trait]
impl ServiceExecutor for BlobServiceExecutor {
    fn service_name(&self) -> &'static str {
        "blob"
    }

    fn handles(&self) -> &'static [&'static str] {
        &[
            "AddBlob",
            "GetBlob",
            "HasBlob",
            "GetBlobTicket",
            "ListBlobs",
            "ProtectBlob",
            "UnprotectBlob",
            "DeleteBlob",
            "DownloadBlob",
            "DownloadBlobByHash",
            "DownloadBlobByProvider",
            "GetBlobStatus",
            "BlobReplicatePull",
            "GetBlobReplicationStatus",
            "TriggerBlobReplication",
            "RunBlobRepairCycle",
        ]
    }

    fn priority(&self) -> u32 {
        520
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        // Delegate to the existing BlobHandler which still uses ctx internally.
        // This gives us ServiceExecutor registration without rewriting 16 functions.
        crate::handler::BlobHandler.handle(request, &self.ctx).await
    }
}

// Re-use the existing handle method
use aspen_rpc_core::RequestHandler;
