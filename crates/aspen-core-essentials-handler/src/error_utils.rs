//! Shared error utilities for KV and lease handlers.
//!
//! Provides NOT_LEADER detection and error sanitization for all handlers
//! that perform writes through the Raft-backed KV store. Without these
//! guards, raw Raft ForwardToLeader errors leak into domain response
//! fields (e.g., WriteResultResponse.error), preventing clients from
//! detecting the NOT_LEADER condition and rotating to the leader peer.

use aspen_core::error::KeyValueStoreError;

/// Check if a KV error is a NOT_LEADER error.
///
/// Returns true for:
/// - `KeyValueStoreError::NotLeader { .. }` (structured variant from `map_raft_write_error`)
/// - Any error whose message contains ForwardToLeader/NOT_LEADER/not leader (string fallback for
///   errors that bypass structured mapping)
///
/// When this returns true, the handler should return `ClientRpcResponse::error("NOT_LEADER", ...)`
/// instead of embedding the error in a domain response field. This allows clients to detect the
/// condition at the top level and rotate to the next bootstrap peer.
pub fn is_not_leader_error(e: &KeyValueStoreError) -> bool {
    matches!(e, KeyValueStoreError::NotLeader { .. }) || {
        let msg = e.to_string();
        msg.contains("ForwardToLeader") || msg.contains("NOT_LEADER") || msg.contains("not leader")
    }
}

/// Sanitize a KV error message for client consumption.
///
/// For NOT_LEADER errors, returns the fixed string `"NOT_LEADER"` to avoid
/// leaking internal Raft topology (leader IDs, node addresses).
/// For all other errors, returns the error's Display string as-is.
pub fn sanitize_kv_error(e: &KeyValueStoreError) -> String {
    let msg = e.to_string();
    if msg.contains("ForwardToLeader") || msg.contains("NOT_LEADER") || msg.contains("not leader") {
        return "NOT_LEADER".to_string();
    }
    msg
}
