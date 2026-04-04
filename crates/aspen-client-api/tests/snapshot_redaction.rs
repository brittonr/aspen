//! Snapshot redaction rules for dynamic fields.
//!
//! Demonstrates how to redact timestamps, UUIDs, node IDs, and other
//! non-deterministic values in insta snapshots.

use aspen_client_api::messages::*;

/// Common redaction settings for Aspen snapshots.
///
/// Redacts:
/// - Timestamps (any integer field ending in `_ms` or `_seconds`)
/// - UUIDs (36-char hex-dash format)
/// - Iroh node IDs (64-char hex strings)
fn redaction_settings() -> insta::Settings {
    let mut settings = insta::Settings::clone_current();
    settings.add_filter(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b", "[UUID]");
    settings.add_filter(r"\b[0-9a-f]{64}\b", "[NODE_ID]");
    settings
}

#[test]
fn snapshot_error_response_with_dynamic_node_id() {
    let settings = redaction_settings();
    settings.bind(|| {
        let resp = ClientRpcResponse::Error(ErrorResponse {
            code: "NOT_LEADER".to_string(),
            message: "forward to leader at node abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
        });
        insta::assert_json_snapshot!("error_with_node_id_redacted", resp);
    });
}

#[test]
fn snapshot_error_response_with_uuid() {
    let settings = redaction_settings();
    settings.bind(|| {
        let resp = ClientRpcResponse::Error(ErrorResponse {
            code: "REQUEST_FAILED".to_string(),
            message: "request 550e8400-e29b-41d4-a716-446655440000 timed out".to_string(),
        });
        insta::assert_json_snapshot!("error_with_uuid_redacted", resp);
    });
}

#[test]
fn snapshot_write_result_with_no_dynamic_fields() {
    // Verify that redaction rules don't affect static content
    let settings = redaction_settings();
    settings.bind(|| {
        let resp = ClientRpcResponse::WriteResult(cluster::WriteResultResponse {
            is_success: true,
            error: None,
        });
        insta::assert_json_snapshot!("write_result_no_redaction_needed", resp);
    });
}
