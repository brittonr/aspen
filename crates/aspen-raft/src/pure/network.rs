//! Pure functions for network-layer logic.
//!
//! This module contains pure functions extracted from the network layer for
//! improved testability. All functions are deterministic and side-effect free.
//!
//! # Functions
//!
//! - [`classify_rpc_error`]: Classify RPC errors to determine connection status
//! - [`maybe_prefix_shard_id`]: Conditionally prefix message with shard ID
//! - [`extract_sharded_response`]: Validate and extract sharded response payload
//! - [`deserialize_rpc_response`]: Deserialize RPC response with backwards compatibility
//! - [`classify_response_health`]: Determine connection health from response type
//!
//! # Tiger Style
//!
//! - No allocations in classification functions
//! - Explicit error types for all failure modes
//! - Deterministic: same inputs always produce same outputs

use crate::node_failure_detection::ConnectionStatus;
use crate::pure::encoding::{SHARD_PREFIX_SIZE, encode_shard_prefix, try_decode_shard_prefix};
use crate::rpc::{RaftRpcResponse, RaftRpcResponseWithTimestamps, TimestampInfo};

/// Classify an RPC error to determine Raft and Iroh connection status.
///
/// This function analyzes error messages to classify failures into:
/// - Connection pool or peer address errors: Both Raft and Iroh disconnected (NodeCrash)
/// - Stream errors: Iroh connected but Raft disconnected (ActorCrash)
/// - Other errors (timeout, deserialization): Assume Iroh connected, Raft disconnected
///
/// # Arguments
///
/// * `error_message` - The error message string to classify
///
/// # Returns
///
/// A tuple of `(raft_status, iroh_status)` where each is a `ConnectionStatus`.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::classify_rpc_error;
/// use aspen_raft::node_failure_detection::ConnectionStatus;
///
/// // Connection pool error -> NodeCrash
/// let (raft, iroh) = classify_rpc_error("connection pool: failed to connect");
/// assert_eq!(raft, ConnectionStatus::Disconnected);
/// assert_eq!(iroh, ConnectionStatus::Disconnected);
///
/// // Stream error -> ActorCrash (Iroh up, Raft down)
/// let (raft, iroh) = classify_rpc_error("stream failure: reset by peer");
/// assert_eq!(raft, ConnectionStatus::Disconnected);
/// assert_eq!(iroh, ConnectionStatus::Connected);
/// ```
#[inline]
pub fn classify_rpc_error(error_message: &str) -> (ConnectionStatus, ConnectionStatus) {
    if error_message.contains("connection pool") || error_message.contains("peer address not found") {
        // Connection-level failure: both Raft and Iroh are down
        (ConnectionStatus::Disconnected, ConnectionStatus::Disconnected)
    } else if error_message.contains("stream") {
        // Stream-level failure: Iroh connection is up, but Raft communication failed
        (ConnectionStatus::Disconnected, ConnectionStatus::Connected)
    } else {
        // Other errors (timeout, deserialization, etc.): assume Iroh is up but Raft has issues
        (ConnectionStatus::Disconnected, ConnectionStatus::Connected)
    }
}

/// Conditionally prefix a serialized message with a shard ID.
///
/// When `shard_id` is `Some`, prepends a 4-byte big-endian shard ID to the message.
/// When `shard_id` is `None`, returns the message unchanged.
///
/// # Arguments
///
/// * `serialized` - The serialized RPC message bytes
/// * `shard_id` - Optional shard ID to prefix
///
/// # Returns
///
/// The message bytes, potentially prefixed with the shard ID.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::maybe_prefix_shard_id;
///
/// let message = vec![1, 2, 3, 4];
///
/// // With shard ID
/// let prefixed = maybe_prefix_shard_id(message.clone(), Some(42));
/// assert_eq!(&prefixed[..4], &[0, 0, 0, 42]); // shard prefix
/// assert_eq!(&prefixed[4..], &[1, 2, 3, 4]);  // original message
///
/// // Without shard ID
/// let unprefixed = maybe_prefix_shard_id(message.clone(), None);
/// assert_eq!(unprefixed, message);
/// ```
#[inline]
pub fn maybe_prefix_shard_id(serialized: Vec<u8>, shard_id: Option<u32>) -> Vec<u8> {
    if let Some(id) = shard_id {
        let mut prefixed = Vec::with_capacity(SHARD_PREFIX_SIZE + serialized.len());
        prefixed.extend_from_slice(&encode_shard_prefix(id));
        prefixed.extend_from_slice(&serialized);
        prefixed
    } else {
        serialized
    }
}

/// Extract the payload from a sharded response, validating the shard ID prefix.
///
/// When `expected_shard_id` is `Some`, this function:
/// 1. Verifies the response has at least `SHARD_PREFIX_SIZE` bytes
/// 2. Decodes and validates the shard ID prefix matches the expected value
/// 3. Returns the payload bytes after the prefix
///
/// When `expected_shard_id` is `None`, returns the entire response buffer.
///
/// # Arguments
///
/// * `response_buf` - The raw response buffer
/// * `expected_shard_id` - Optional expected shard ID to validate
///
/// # Returns
///
/// - `Ok(&[u8])` - The payload bytes (after prefix if sharded)
/// - `Err(String)` - Error message if validation fails
///
/// # Example
///
/// ```
/// use aspen_raft::pure::extract_sharded_response;
///
/// // Valid sharded response
/// let response = vec![0, 0, 0, 42, 10, 20, 30]; // shard 42 + payload
/// let payload = extract_sharded_response(&response, Some(42)).unwrap();
/// assert_eq!(payload, &[10, 20, 30]);
///
/// // Shard ID mismatch
/// let result = extract_sharded_response(&response, Some(99));
/// assert!(result.is_err());
///
/// // Non-sharded response
/// let response = vec![10, 20, 30];
/// let payload = extract_sharded_response(&response, None).unwrap();
/// assert_eq!(payload, &[10, 20, 30]);
/// ```
#[inline]
pub fn extract_sharded_response<'a>(
    response_buf: &'a [u8],
    expected_shard_id: Option<u32>,
) -> Result<&'a [u8], String> {
    if let Some(expected_id) = expected_shard_id {
        let response_shard_id = try_decode_shard_prefix(response_buf).ok_or_else(|| {
            format!(
                "sharded response too short: expected at least {} bytes, got {}",
                SHARD_PREFIX_SIZE,
                response_buf.len()
            )
        })?;

        if response_shard_id != expected_id {
            return Err(format!("shard ID mismatch: expected {}, got {}", expected_id, response_shard_id));
        }

        Ok(&response_buf[SHARD_PREFIX_SIZE..])
    } else {
        Ok(response_buf)
    }
}

/// Deserialize an RPC response with backwards compatibility.
///
/// Attempts to deserialize the response in two formats:
/// 1. New format: `RaftRpcResponseWithTimestamps` (includes optional timestamps)
/// 2. Legacy format: `RaftRpcResponse` (no timestamps)
///
/// This allows graceful handling of responses from both new and old servers.
///
/// # Arguments
///
/// * `response_bytes` - The serialized response bytes
///
/// # Returns
///
/// - `Ok((RaftRpcResponse, Option<TimestampInfo>))` - The response and optional timestamps
/// - `Err(postcard::Error)` - If deserialization fails for both formats
///
/// # Example
///
/// ```ignore
/// use aspen_raft::pure::deserialize_rpc_response;
///
/// // New format with timestamps
/// let (response, timestamps) = deserialize_rpc_response(&bytes)?;
/// if let Some(ts) = timestamps {
///     println!("Server recv: {}, send: {}", ts.server_recv_ms, ts.server_send_ms);
/// }
/// ```
pub fn deserialize_rpc_response(
    response_bytes: &[u8],
) -> Result<(RaftRpcResponse, Option<TimestampInfo>), postcard::Error> {
    // Try to deserialize as response with timestamps first (new format)
    if let Ok(response_with_ts) = postcard::from_bytes::<RaftRpcResponseWithTimestamps>(response_bytes) {
        return Ok((response_with_ts.inner, response_with_ts.timestamps));
    }

    // Fall back to legacy format (no timestamps)
    let response = postcard::from_bytes::<RaftRpcResponse>(response_bytes)?;
    Ok((response, None))
}

/// Classify the connection health based on an RPC response.
///
/// Analyzes the response type to determine Raft and Iroh connection status:
/// - `FatalError`: Raft disconnected (core is down), Iroh connected (we got a response)
/// - Other responses: Both Raft and Iroh connected (successful RPC)
///
/// # Arguments
///
/// * `response` - The RPC response to classify
///
/// # Returns
///
/// A tuple of `(raft_status, iroh_status)` where each is a `ConnectionStatus`.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::classify_response_health;
/// use aspen_raft::node_failure_detection::ConnectionStatus;
/// use aspen_raft::rpc::{RaftRpcResponse, RaftFatalErrorKind};
/// use openraft::raft::AppendEntriesResponse;
///
/// // Successful response -> both connected
/// let response = RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success);
/// let (raft, iroh) = classify_response_health(&response);
/// assert_eq!(raft, ConnectionStatus::Connected);
/// assert_eq!(iroh, ConnectionStatus::Connected);
///
/// // Fatal error -> Raft down, Iroh up
/// let response = RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked);
/// let (raft, iroh) = classify_response_health(&response);
/// assert_eq!(raft, ConnectionStatus::Disconnected);
/// assert_eq!(iroh, ConnectionStatus::Connected);
/// ```
#[inline]
pub fn classify_response_health(response: &RaftRpcResponse) -> (ConnectionStatus, ConnectionStatus) {
    match response {
        RaftRpcResponse::FatalError(_) => {
            // Peer's RaftCore is down, but we got a response so Iroh is connected
            (ConnectionStatus::Disconnected, ConnectionStatus::Connected)
        }
        _ => {
            // Successful RPC: both Raft and Iroh are connected
            (ConnectionStatus::Connected, ConnectionStatus::Connected)
        }
    }
}

#[cfg(test)]
mod tests {
    use openraft::Vote;
    use openraft::raft::{AppendEntriesResponse, VoteResponse};

    use super::*;
    use crate::rpc::RaftFatalErrorKind;
    use crate::types::NodeId;

    // =========================================================================
    // classify_rpc_error Tests
    // =========================================================================

    #[test]
    fn test_classify_rpc_error_connection_pool() {
        let (raft, iroh) = classify_rpc_error("connection pool: failed to connect");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_classify_rpc_error_peer_address_not_found() {
        let (raft, iroh) = classify_rpc_error("peer address not found in peer map");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_classify_rpc_error_stream_failure() {
        let (raft, iroh) = classify_rpc_error("stream failure: connection refused");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_rpc_error_stream_reset() {
        let (raft, iroh) = classify_rpc_error("stream reset by peer");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_rpc_error_timeout() {
        let (raft, iroh) = classify_rpc_error("timeout reading RPC response");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_rpc_error_deserialization() {
        let (raft, iroh) = classify_rpc_error("failed to deserialize RPC response");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_rpc_error_empty_message() {
        let (raft, iroh) = classify_rpc_error("");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_rpc_error_unknown_error() {
        let (raft, iroh) = classify_rpc_error("some unknown error occurred");
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    // =========================================================================
    // maybe_prefix_shard_id Tests
    // =========================================================================

    #[test]
    fn test_maybe_prefix_shard_id_with_shard() {
        let message = vec![1, 2, 3, 4];
        let prefixed = maybe_prefix_shard_id(message, Some(42));
        assert_eq!(prefixed.len(), 8); // 4 byte prefix + 4 byte message
        assert_eq!(&prefixed[..4], &[0, 0, 0, 42]);
        assert_eq!(&prefixed[4..], &[1, 2, 3, 4]);
    }

    #[test]
    fn test_maybe_prefix_shard_id_without_shard() {
        let message = vec![1, 2, 3, 4];
        let result = maybe_prefix_shard_id(message.clone(), None);
        assert_eq!(result, message);
    }

    #[test]
    fn test_maybe_prefix_shard_id_empty_message() {
        let message: Vec<u8> = vec![];
        let prefixed = maybe_prefix_shard_id(message, Some(99));
        assert_eq!(prefixed.len(), 4);
        assert_eq!(prefixed, vec![0, 0, 0, 99]);
    }

    #[test]
    fn test_maybe_prefix_shard_id_max_shard() {
        let message = vec![1];
        let prefixed = maybe_prefix_shard_id(message, Some(u32::MAX));
        assert_eq!(&prefixed[..4], &[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(&prefixed[4..], &[1]);
    }

    #[test]
    fn test_maybe_prefix_shard_id_zero_shard() {
        let message = vec![10, 20];
        let prefixed = maybe_prefix_shard_id(message, Some(0));
        assert_eq!(&prefixed[..4], &[0, 0, 0, 0]);
        assert_eq!(&prefixed[4..], &[10, 20]);
    }

    // =========================================================================
    // extract_sharded_response Tests
    // =========================================================================

    #[test]
    fn test_extract_sharded_response_valid() {
        let response = vec![0, 0, 0, 42, 10, 20, 30];
        let payload = extract_sharded_response(&response, Some(42)).unwrap();
        assert_eq!(payload, &[10, 20, 30]);
    }

    #[test]
    fn test_extract_sharded_response_shard_mismatch() {
        let response = vec![0, 0, 0, 42, 10, 20, 30];
        let result = extract_sharded_response(&response, Some(99));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("shard ID mismatch"));
    }

    #[test]
    fn test_extract_sharded_response_too_short() {
        let response = vec![0, 0, 0]; // only 3 bytes
        let result = extract_sharded_response(&response, Some(42));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_extract_sharded_response_no_shard() {
        let response = vec![10, 20, 30];
        let payload = extract_sharded_response(&response, None).unwrap();
        assert_eq!(payload, &[10, 20, 30]);
    }

    #[test]
    fn test_extract_sharded_response_empty_payload() {
        let response = vec![0, 0, 0, 42]; // just prefix, no payload
        let payload = extract_sharded_response(&response, Some(42)).unwrap();
        assert!(payload.is_empty());
    }

    #[test]
    fn test_extract_sharded_response_empty_buffer_no_shard() {
        let response: Vec<u8> = vec![];
        let payload = extract_sharded_response(&response, None).unwrap();
        assert!(payload.is_empty());
    }

    #[test]
    fn test_extract_sharded_response_empty_buffer_with_shard() {
        let response: Vec<u8> = vec![];
        let result = extract_sharded_response(&response, Some(1));
        assert!(result.is_err());
    }

    // =========================================================================
    // deserialize_rpc_response Tests
    // =========================================================================

    #[test]
    fn test_deserialize_rpc_response_vote_legacy() {
        let response = RaftRpcResponse::Vote(VoteResponse {
            vote: Vote::new(1, NodeId::from(1)),
            vote_granted: true,
            last_log_id: None,
        });
        let bytes = postcard::to_stdvec(&response).expect("serialize");

        let (deserialized, timestamps) = deserialize_rpc_response(&bytes).expect("deserialize");
        assert!(matches!(deserialized, RaftRpcResponse::Vote(v) if v.vote_granted));
        assert!(timestamps.is_none());
    }

    #[test]
    fn test_deserialize_rpc_response_with_timestamps() {
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success),
            timestamps: Some(TimestampInfo {
                server_recv_ms: 1000,
                server_send_ms: 1005,
            }),
        };
        let bytes = postcard::to_stdvec(&response).expect("serialize");

        let (deserialized, timestamps) = deserialize_rpc_response(&bytes).expect("deserialize");
        assert!(matches!(deserialized, RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success)));
        assert!(timestamps.is_some());
        let ts = timestamps.unwrap();
        assert_eq!(ts.server_recv_ms, 1000);
        assert_eq!(ts.server_send_ms, 1005);
    }

    #[test]
    fn test_deserialize_rpc_response_with_none_timestamps() {
        let response = RaftRpcResponseWithTimestamps {
            inner: RaftRpcResponse::AppendEntries(AppendEntriesResponse::Conflict),
            timestamps: None,
        };
        let bytes = postcard::to_stdvec(&response).expect("serialize");

        let (deserialized, timestamps) = deserialize_rpc_response(&bytes).expect("deserialize");
        assert!(matches!(deserialized, RaftRpcResponse::AppendEntries(AppendEntriesResponse::Conflict)));
        assert!(timestamps.is_none());
    }

    #[test]
    fn test_deserialize_rpc_response_fatal_error() {
        let response = RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked);
        let bytes = postcard::to_stdvec(&response).expect("serialize");

        let (deserialized, timestamps) = deserialize_rpc_response(&bytes).expect("deserialize");
        assert!(matches!(deserialized, RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked)));
        assert!(timestamps.is_none());
    }

    #[test]
    fn test_deserialize_rpc_response_invalid_bytes() {
        let invalid_bytes = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result = deserialize_rpc_response(&invalid_bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_rpc_response_empty_bytes() {
        let empty_bytes: Vec<u8> = vec![];
        let result = deserialize_rpc_response(&empty_bytes);
        assert!(result.is_err());
    }

    // =========================================================================
    // classify_response_health Tests
    // =========================================================================

    #[test]
    fn test_classify_response_health_vote_success() {
        let response = RaftRpcResponse::Vote(VoteResponse {
            vote: Vote::new(1, NodeId::from(1)),
            vote_granted: true,
            last_log_id: None,
        });
        let (raft, iroh) = classify_response_health(&response);
        assert_eq!(raft, ConnectionStatus::Connected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_response_health_append_entries_success() {
        let response = RaftRpcResponse::AppendEntries(AppendEntriesResponse::Success);
        let (raft, iroh) = classify_response_health(&response);
        assert_eq!(raft, ConnectionStatus::Connected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_response_health_append_entries_conflict() {
        let response = RaftRpcResponse::AppendEntries(AppendEntriesResponse::Conflict);
        let (raft, iroh) = classify_response_health(&response);
        assert_eq!(raft, ConnectionStatus::Connected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_response_health_fatal_panicked() {
        let response = RaftRpcResponse::FatalError(RaftFatalErrorKind::Panicked);
        let (raft, iroh) = classify_response_health(&response);
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_response_health_fatal_stopped() {
        let response = RaftRpcResponse::FatalError(RaftFatalErrorKind::Stopped);
        let (raft, iroh) = classify_response_health(&response);
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_response_health_fatal_storage_error() {
        let response = RaftRpcResponse::FatalError(RaftFatalErrorKind::StorageError);
        let (raft, iroh) = classify_response_health(&response);
        assert_eq!(raft, ConnectionStatus::Disconnected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }

    #[test]
    fn test_classify_response_health_install_snapshot_ok() {
        use openraft::raft::SnapshotResponse;
        let response = RaftRpcResponse::InstallSnapshot(Ok(SnapshotResponse {
            vote: Vote::new(1, NodeId::from(1)),
        }));
        let (raft, iroh) = classify_response_health(&response);
        assert_eq!(raft, ConnectionStatus::Connected);
        assert_eq!(iroh, ConnectionStatus::Connected);
    }
}
