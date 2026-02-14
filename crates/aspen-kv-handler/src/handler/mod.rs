//! Key-Value request handler.
//!
//! Handles: ReadKey, WriteKey, DeleteKey, ScanKeys, BatchRead, BatchWrite,
//! ConditionalBatchWrite, CompareAndSwapKey, CompareAndDeleteKey.
//!
//! Each operation domain is handled by a dedicated sub-handler:
//! - `ReadHandler` for read and batch read operations
//! - `WriteHandler` for write and batch write operations
//! - `DeleteHandler` for delete operations
//! - `ScanHandler` for scan operations
//! - `CasHandler` for compare-and-swap and compare-and-delete operations

mod cas;
mod delete;
mod read;
mod scan;
mod write;

use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_rpc_core::ClientProtocolContext;
use aspen_rpc_core::RequestHandler;
use cas::CasHandler;
use delete::DeleteHandler;
use read::ReadHandler;
use scan::ScanHandler;
use write::WriteHandler;

/// Handler for key-value operations.
pub struct KvHandler;

#[async_trait::async_trait]
impl RequestHandler for KvHandler {
    fn can_handle(&self, request: &ClientRpcRequest) -> bool {
        let read = ReadHandler;
        let write = WriteHandler;
        let delete = DeleteHandler;
        let scan = ScanHandler;
        let cas = CasHandler;

        read.can_handle(request)
            || write.can_handle(request)
            || delete.can_handle(request)
            || scan.can_handle(request)
            || cas.can_handle(request)
    }

    async fn handle(
        &self,
        request: ClientRpcRequest,
        ctx: &ClientProtocolContext,
    ) -> anyhow::Result<ClientRpcResponse> {
        let read = ReadHandler;
        let write = WriteHandler;
        let delete = DeleteHandler;
        let scan = ScanHandler;
        let cas = CasHandler;

        if read.can_handle(&request) {
            return read.handle(request, ctx).await;
        }
        if write.can_handle(&request) {
            return write.handle(request, ctx).await;
        }
        if delete.can_handle(&request) {
            return delete.handle(request, ctx).await;
        }
        if scan.can_handle(&request) {
            return scan.handle(request, ctx).await;
        }
        if cas.can_handle(&request) {
            return cas.handle(request, ctx).await;
        }

        Err(anyhow::anyhow!("request not handled by KvHandler"))
    }

    fn name(&self) -> &'static str {
        "KvHandler"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_handle_read_key() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ReadKey {
            key: "test".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_write_key() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::WriteKey {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_can_handle_delete_key() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::DeleteKey {
            key: "test".to_string(),
        }));
    }

    #[test]
    fn test_can_handle_scan_keys() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ScanKeys {
            prefix: "test".to_string(),
            limit: Some(10),
            continuation_token: None,
        }));
    }

    #[test]
    fn test_can_handle_batch_read() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::BatchRead {
            keys: vec!["a".to_string(), "b".to_string()],
        }));
    }

    #[test]
    fn test_can_handle_batch_write() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::BatchWrite { operations: vec![] }));
    }

    #[test]
    fn test_can_handle_conditional_batch_write() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::ConditionalBatchWrite {
            conditions: vec![],
            operations: vec![],
        }));
    }

    #[test]
    fn test_can_handle_compare_and_swap() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CompareAndSwapKey {
            key: "test".to_string(),
            expected: None,
            new_value: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_can_handle_compare_and_delete() {
        let handler = KvHandler;
        assert!(handler.can_handle(&ClientRpcRequest::CompareAndDeleteKey {
            key: "test".to_string(),
            expected: vec![1, 2, 3],
        }));
    }

    #[test]
    fn test_rejects_unrelated_requests() {
        let handler = KvHandler;

        // Core requests
        assert!(!handler.can_handle(&ClientRpcRequest::Ping));
        assert!(!handler.can_handle(&ClientRpcRequest::GetHealth));

        // Cluster requests
        assert!(!handler.can_handle(&ClientRpcRequest::InitCluster));
        assert!(!handler.can_handle(&ClientRpcRequest::GetClusterState));

        // Coordination requests
        assert!(!handler.can_handle(&ClientRpcRequest::LockAcquire {
            key: "test".to_string(),
            holder_id: "holder".to_string(),
            ttl_ms: 30000,
            timeout_ms: 0,
        }));
    }

    #[test]
    fn test_handler_name() {
        let handler = KvHandler;
        assert_eq!(handler.name(), "KvHandler");
    }
}
