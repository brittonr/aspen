//! Serialization snapshot tests for ClientRpcRequest/Response.
//!
//! These tests use `insta` to detect accidental changes to the wire format.
//! If a format change is intentional, run `cargo insta review` to approve it.

use aspen_client_api::messages::*;

// =============================================================================
// KV Request Snapshots
// =============================================================================

#[test]
fn snapshot_write_key_request() {
    let req = ClientRpcRequest::WriteKey {
        key: "my-key".to_string(),
        value: b"hello world".to_vec(),
    };
    insta::assert_json_snapshot!("write_key_request", req);
}

#[test]
fn snapshot_read_key_request() {
    let req = ClientRpcRequest::ReadKey {
        key: "my-key".to_string(),
    };
    insta::assert_json_snapshot!("read_key_request", req);
}

#[test]
fn snapshot_delete_key_request() {
    let req = ClientRpcRequest::DeleteKey {
        key: "my-key".to_string(),
    };
    insta::assert_json_snapshot!("delete_key_request", req);
}

#[test]
fn snapshot_scan_keys_request() {
    let req = ClientRpcRequest::ScanKeys {
        prefix: "app:".to_string(),
        limit: Some(100),
        continuation_token: None,
    };
    insta::assert_json_snapshot!("scan_keys_request", req);
}

// =============================================================================
// KV Response Snapshots
// =============================================================================

#[test]
fn snapshot_write_result_response() {
    let resp = ClientRpcResponse::WriteResult(cluster::WriteResultResponse {
        is_success: true,
        error: None,
    });
    insta::assert_json_snapshot!("write_result_response", resp);
}

#[test]
fn snapshot_read_result_response_found() {
    let resp = ClientRpcResponse::ReadResult(cluster::ReadResultResponse {
        was_found: true,
        value: Some(b"hello world".to_vec()),
        error: None,
    });
    insta::assert_json_snapshot!("read_result_found", resp);
}

#[test]
fn snapshot_read_result_response_not_found() {
    let resp = ClientRpcResponse::ReadResult(cluster::ReadResultResponse {
        was_found: false,
        value: None,
        error: None,
    });
    insta::assert_json_snapshot!("read_result_not_found", resp);
}

#[test]
fn snapshot_delete_result_response() {
    let resp = ClientRpcResponse::DeleteResult(kv::DeleteResultResponse {
        key: "my-key".to_string(),
        was_deleted: true,
        error: None,
    });
    insta::assert_json_snapshot!("delete_result_response", resp);
}

#[test]
fn snapshot_scan_result_response() {
    let resp = ClientRpcResponse::ScanResult(kv::ScanResultResponse {
        entries: vec![kv::ScanEntry {
            key: "app:user:1".to_string(),
            value: "Alice".to_string(),
            version: 3,
            create_revision: 1,
            mod_revision: 3,
        }],
        count: 1,
        is_truncated: false,
        continuation_token: None,
        error: None,
    });
    insta::assert_json_snapshot!("scan_result_response", resp);
}

// =============================================================================
// Cluster Request/Response Snapshots
// =============================================================================

#[test]
fn snapshot_get_health_request() {
    let req = ClientRpcRequest::GetHealth;
    insta::assert_json_snapshot!("get_health_request", req);
}

#[test]
fn snapshot_get_raft_metrics_request() {
    let req = ClientRpcRequest::GetRaftMetrics;
    insta::assert_json_snapshot!("get_raft_metrics_request", req);
}

// =============================================================================
// Error Response Snapshots
// =============================================================================

#[test]
fn snapshot_error_response() {
    let resp = ClientRpcResponse::Error(ErrorResponse {
        code: "NOT_LEADER".to_string(),
        message: "This node is not the current leader".to_string(),
    });
    insta::assert_json_snapshot!("error_response", resp);
}
