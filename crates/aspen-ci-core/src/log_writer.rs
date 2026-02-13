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

/// Completion marker metadata.
///
/// Written when a CI job completes to signal that the log stream is complete.
/// Watchers can check for this marker to know when to stop polling for new chunks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiLogCompleteMarker {
    /// Total number of log chunks written.
    pub total_chunks: u32,
    /// Timestamp when the job completed (ms since epoch).
    pub timestamp_ms: u64,
    /// Final job status.
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
            status: "success".to_string(),
        };

        let json = serde_json::to_string(&marker).unwrap();
        let parsed: CiLogCompleteMarker = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.total_chunks, 100);
        assert_eq!(parsed.status, "success");
    }
}
