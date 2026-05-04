//! CI job log streaming types.
//!
//! This module provides the data types for CI job log streaming. The actual
//! log writer implementation that interacts with the KV store remains in
//! `aspen-ci` since it requires async runtime and KV store dependencies.
//!
//! # Architecture
//!
//! ```text
//! CI Job (ShellCommand/NixBuild)
//!          |
//!          v
//!   SpawnedLogWriter (mpsc channel) [in aspen-ci]
//!          |
//!          v
//!   CiLogWriter (buffered) [in aspen-ci]
//!          |
//!          v
//!   KV Store (_ci:logs:{run_id}:{job_id}:{chunk_index})
//!          |
//!          v
//!   WatchSession (LOG_SUBSCRIBER_ALPN)
//!          |
//!          v
//!   TUI Client (real-time display)
//! ```
//!
//! # Tiger Style
//!
//! - Bounded chunk size (8KB) for predictable KV write latency
//! - Bounded total chunks (10K) to prevent disk exhaustion
//! - Bounded channel capacity (1000 messages) for backpressure
//! - Periodic flush (500ms) to balance latency vs. throughput

use serde::Deserialize;
use serde::Serialize;

/// A CI log chunk stored in KV.
///
/// Each chunk contains a portion of the log output from a CI job.
/// Chunks are indexed for sequential reading and ordered by timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiLogChunk {
    /// Chunk index within the job's log stream.
    pub index: u32,
    /// Log content (may contain multiple lines).
    pub content: String,
    /// Timestamp when this chunk was written (ms since epoch).
    pub timestamp_ms: u64,
}

/// Stable status stored in CI log completion markers.
///
/// This describes the log stream state, not the CI job's success/failure status.
/// Operators should use `CiGetStatus` or `CiGetRunReceipt` for job and pipeline
/// terminal labels.
pub const CI_LOG_COMPLETE_STATUS: &str = "done";

/// Completion marker metadata.
///
/// Written when a CI job log writer closes to signal that the log stream is
/// complete. Watchers can check for this marker to know when to stop polling for
/// new chunks; the marker status is a log-stream status, not the job result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiLogCompleteMarker {
    /// Total number of log chunks written.
    pub total_chunks: u32,
    /// Timestamp when the log stream completed (ms since epoch).
    pub timestamp_ms: u64,
    /// Log-stream completion status (`CI_LOG_COMPLETE_STATUS`).
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_log_chunk_serialization() {
        let chunk = CiLogChunk {
            index: 42,
            content: "[stdout] Hello, world!\n[stderr] Warning!\n".to_string(),
            timestamp_ms: 1234567890,
        };

        let json = serde_json::to_string(&chunk).unwrap();
        let parsed: CiLogChunk = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.index, 42);
        assert_eq!(parsed.content, chunk.content);
        assert_eq!(parsed.timestamp_ms, 1234567890);
    }

    #[test]
    fn test_completion_marker_serialization() {
        let marker = CiLogCompleteMarker {
            total_chunks: 100,
            timestamp_ms: 9876543210,
            status: CI_LOG_COMPLETE_STATUS.to_string(),
        };

        let json = serde_json::to_string(&marker).unwrap();
        let parsed: CiLogCompleteMarker = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.total_chunks, 100);
        assert_eq!(parsed.status, CI_LOG_COMPLETE_STATUS);
        assert_eq!(CI_LOG_COMPLETE_STATUS, "done");
    }

    /// Verify CiLogChunk can be deserialized from raw bytes, simulating
    /// the value payload of a WatchEvent::Set from the log subscriber.
    #[test]
    fn test_ci_log_chunk_from_watch_event_value() {
        let chunk = CiLogChunk {
            index: 7,
            content: "[stdout] building crate aspen-core\n[stderr] warning: unused import\n".to_string(),
            timestamp_ms: 1700000050000,
        };

        // Simulate what the Raft log subscriber stores: JSON-serialized chunk as bytes.
        let value_bytes = serde_json::to_vec(&chunk).unwrap();

        // This is exactly how the CLI and TUI parse WatchEvent::Set values.
        let parsed: CiLogChunk = serde_json::from_slice(&value_bytes).unwrap();

        assert_eq!(parsed.index, 7);
        assert_eq!(parsed.timestamp_ms, 1700000050000);
        assert!(parsed.content.contains("[stdout] building crate aspen-core"));
        assert!(parsed.content.contains("[stderr] warning: unused import"));
    }
}
