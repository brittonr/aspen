//! CI job log streaming writer.
//!
//! Provides a buffered log writer that flushes log chunks to the KV store
//! for real-time streaming to TUI clients via WatchSession.
//!
//! The log chunk types (`CiLogChunk`, `CiLogCompleteMarker`) are defined in
//! `aspen-ci-core::log_writer` and re-exported here. This module provides the
//! actual writer implementation that interacts with the KV store.
//!
//! # Architecture
//!
//! ```text
//! CI Job (ShellCommand/NixBuild)
//!          |
//!          v
//!   SpawnedLogWriter (mpsc channel)
//!          |
//!          v
//!   CiLogWriter (buffered)
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

use std::sync::Arc;
use std::time::Duration;

// Re-export log chunk types from aspen-ci-core
pub use aspen_ci_core::CiLogChunk;
pub use aspen_ci_core::CiLogCompleteMarker;
use aspen_core::CI_LOG_COMPLETE_MARKER;
use aspen_core::CI_LOG_FLUSH_INTERVAL_MS;
use aspen_core::CI_LOG_KV_PREFIX;
use aspen_core::KeyValueStore;
use aspen_core::MAX_CI_LOG_CHUNK_SIZE;
use aspen_core::MAX_CI_LOG_CHUNKS_PER_JOB;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::debug;
use tracing::warn;

use crate::error::CiError;
use crate::error::Result;

/// Channel buffer capacity for log lines.
///
/// Tiger Style: Bounded to provide backpressure when KV writes are slow.
const LOG_CHANNEL_CAPACITY: usize = 1000;

/// CI job log writer that streams logs to KV store.
///
/// Buffers log lines and flushes to KV in chunks to balance
/// write efficiency against streaming latency.
pub struct CiLogWriter<S: KeyValueStore + ?Sized> {
    /// Pipeline run ID.
    run_id: String,
    /// Job ID within the pipeline.
    job_id: String,
    /// KV store for log persistence.
    kv_store: Arc<S>,
    /// Current chunk index (zero-padded in keys).
    chunk_index: u32,
    /// Buffered log content awaiting flush.
    buffer: String,
    /// Maximum chunks allowed per job.
    max_chunks: u32,
    /// Whether we've hit the chunk limit.
    limit_reached: bool,
}

impl<S: KeyValueStore + ?Sized + 'static> CiLogWriter<S> {
    /// Create a new log writer for a CI job.
    ///
    /// # Arguments
    ///
    /// * `run_id` - Pipeline run ID
    /// * `job_id` - Job ID within the pipeline
    /// * `kv_store` - KV store for log persistence
    pub fn new(run_id: String, job_id: String, kv_store: Arc<S>) -> Self {
        Self {
            run_id,
            job_id,
            kv_store,
            chunk_index: 0,
            buffer: String::with_capacity(MAX_CI_LOG_CHUNK_SIZE as usize),
            max_chunks: MAX_CI_LOG_CHUNKS_PER_JOB,
            limit_reached: false,
        }
    }

    /// Build the KV key for a log chunk.
    ///
    /// Format: `_ci:logs:{run_id}:{job_id}:{chunk_index:010}`
    /// Zero-padded index ensures lexicographic ordering matches insertion order.
    fn chunk_key(&self, index: u32) -> String {
        format!("{}{}:{}:{:010}", CI_LOG_KV_PREFIX, self.run_id, self.job_id, index)
    }

    /// Write a line of log output.
    ///
    /// Buffers the line and flushes when buffer exceeds chunk size.
    /// Lines are prefixed with stream identifier for parsing.
    ///
    /// # Arguments
    ///
    /// * `line` - Log line content (without trailing newline)
    /// * `stream` - Stream identifier ("stdout", "stderr", "build")
    pub async fn write_line(&mut self, line: &str, stream: &str) -> Result<()> {
        if self.limit_reached {
            // Silently drop logs beyond limit (Tiger Style: bounded)
            return Ok(());
        }

        if self.chunk_index >= self.max_chunks {
            self.limit_reached = true;
            warn!(
                run_id = %self.run_id,
                job_id = %self.job_id,
                max_chunks = self.max_chunks,
                "CI log chunk limit reached, dropping subsequent logs"
            );
            return Ok(());
        }

        // Add line to buffer with stream prefix for parsing
        let prefixed = format!("[{}] {}\n", stream, line);
        self.buffer.push_str(&prefixed);

        // Flush if buffer exceeds chunk size
        if self.buffer.len() >= MAX_CI_LOG_CHUNK_SIZE as usize {
            self.flush().await?;
        }

        Ok(())
    }

    /// Flush buffered logs to KV store.
    ///
    /// Called periodically or when buffer is full.
    pub async fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() || self.limit_reached {
            return Ok(());
        }

        if self.chunk_index >= self.max_chunks {
            self.limit_reached = true;
            self.buffer.clear();
            return Ok(());
        }

        let key = self.chunk_key(self.chunk_index);
        let value = std::mem::take(&mut self.buffer);

        // Reserve capacity for next chunk
        self.buffer.reserve(MAX_CI_LOG_CHUNK_SIZE as usize);

        let timestamp_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        // Serialize chunk with metadata
        let chunk = CiLogChunk {
            index: self.chunk_index,
            content: value,
            timestamp_ms,
        };
        let chunk_json = serde_json::to_string(&chunk).map_err(|e| CiError::LogSerialization {
            reason: format!("failed to serialize chunk {}: {}", self.chunk_index, e),
        })?;

        let write_request = WriteRequest {
            command: WriteCommand::Set {
                key: key.clone(),
                value: chunk_json,
            },
        };

        self.kv_store.write(write_request).await.map_err(|e| CiError::LogWrite {
            reason: format!("failed to write chunk {} to {}: {}", self.chunk_index, key, e),
        })?;

        debug!(
            run_id = %self.run_id,
            job_id = %self.job_id,
            chunk_index = self.chunk_index,
            "Flushed CI log chunk"
        );

        self.chunk_index += 1;
        Ok(())
    }

    /// Mark the log stream as complete.
    ///
    /// Flushes any remaining buffer and writes a completion marker.
    /// The completion marker signals to watchers that the job has finished.
    ///
    /// # Arguments
    ///
    /// * `status` - Final job status ("success", "failed", "cancelled")
    pub async fn complete(&mut self, status: &str) -> Result<()> {
        // Flush remaining buffer
        self.flush().await?;

        // Write completion marker
        let marker_key = format!("{}{}:{}:{}", CI_LOG_KV_PREFIX, self.run_id, self.job_id, CI_LOG_COMPLETE_MARKER);

        let timestamp_ms =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        let marker = CiLogCompleteMarker {
            total_chunks: self.chunk_index,
            timestamp_ms,
            status: status.to_string(),
        };
        let marker_json = serde_json::to_string(&marker).map_err(|e| CiError::LogSerialization {
            reason: format!("failed to serialize completion marker: {}", e),
        })?;

        let write_request = WriteRequest {
            command: WriteCommand::Set {
                key: marker_key.clone(),
                value: marker_json,
            },
        };

        self.kv_store.write(write_request).await.map_err(|e| CiError::LogWrite {
            reason: format!("failed to write completion marker to {}: {}", marker_key, e),
        })?;

        debug!(
            run_id = %self.run_id,
            job_id = %self.job_id,
            total_chunks = self.chunk_index,
            status = %status,
            "CI log stream completed"
        );

        Ok(())
    }

    /// Get the current chunk index.
    pub fn chunk_count(&self) -> u32 {
        self.chunk_index
    }

    /// Check if the chunk limit has been reached.
    pub fn is_limit_reached(&self) -> bool {
        self.limit_reached
    }
}

/// Internal message type for the spawned log writer channel.
enum LogMessage {
    /// A log line to write.
    Line { content: String, stream: String },
    /// Signal to complete the stream with the given status.
    Complete { status: String },
}

/// Handle for sending log lines to a spawned log writer task.
///
/// This provides a non-blocking interface for CI workers to send log lines
/// without waiting for KV writes to complete.
#[derive(Clone)]
pub struct SpawnedLogWriter {
    tx: mpsc::Sender<LogMessage>,
}

impl SpawnedLogWriter {
    /// Spawn a background log writer task.
    ///
    /// Creates a background task that receives log lines via channel and
    /// periodically flushes them to the KV store.
    ///
    /// # Arguments
    ///
    /// * `run_id` - Pipeline run ID
    /// * `job_id` - Job ID within the pipeline
    /// * `kv_store` - KV store for log persistence
    ///
    /// # Returns
    ///
    /// A tuple of (handle, join_handle) where handle is used to send logs
    /// and join_handle can be awaited to ensure the writer completes.
    pub fn spawn<S: KeyValueStore + ?Sized + Send + Sync + 'static>(
        run_id: String,
        job_id: String,
        kv_store: Arc<S>,
    ) -> (Self, tokio::task::JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel::<LogMessage>(LOG_CHANNEL_CAPACITY);

        let handle = tokio::spawn(async move {
            let mut writer = CiLogWriter::new(run_id.clone(), job_id.clone(), kv_store);
            let mut flush_interval = interval(Duration::from_millis(CI_LOG_FLUSH_INTERVAL_MS));

            // Skip the immediate first tick
            flush_interval.tick().await;

            loop {
                tokio::select! {
                    biased;

                    msg = rx.recv() => {
                        match msg {
                            Some(LogMessage::Line { content, stream }) => {
                                if let Err(e) = writer.write_line(&content, &stream).await {
                                    warn!(
                                        run_id = %run_id,
                                        job_id = %job_id,
                                        error = %e,
                                        "Failed to write log line"
                                    );
                                }
                            }
                            Some(LogMessage::Complete { status }) => {
                                // Final flush and completion marker
                                if let Err(e) = writer.complete(&status).await {
                                    warn!(
                                        run_id = %run_id,
                                        job_id = %job_id,
                                        error = %e,
                                        "Failed to complete log stream"
                                    );
                                }
                                break;
                            }
                            None => {
                                // Channel closed without explicit complete - treat as failed
                                warn!(
                                    run_id = %run_id,
                                    job_id = %job_id,
                                    "Log writer channel closed without completion"
                                );
                                if let Err(e) = writer.complete("unknown").await {
                                    warn!(
                                        run_id = %run_id,
                                        job_id = %job_id,
                                        error = %e,
                                        "Failed to complete log stream on channel close"
                                    );
                                }
                                break;
                            }
                        }
                    }
                    _ = flush_interval.tick() => {
                        if let Err(e) = writer.flush().await {
                            warn!(
                                run_id = %run_id,
                                job_id = %job_id,
                                error = %e,
                                "Failed to flush log buffer"
                            );
                        }
                    }
                }
            }
        });

        (Self { tx }, handle)
    }

    /// Send a log line to the writer.
    ///
    /// This method will wait if the channel is full (backpressure).
    ///
    /// # Arguments
    ///
    /// * `content` - Log line content (without trailing newline)
    /// * `stream` - Stream identifier ("stdout", "stderr", "build")
    pub async fn write(&self, content: String, stream: &str) -> std::result::Result<(), mpsc::error::SendError<()>> {
        self.tx
            .send(LogMessage::Line {
                content,
                stream: stream.to_string(),
            })
            .await
            .map_err(|_| mpsc::error::SendError(()))
    }

    /// Send a log line without waiting (non-blocking).
    ///
    /// Returns an error if the channel is full or closed.
    /// Useful for high-throughput logging where dropped lines are acceptable.
    ///
    /// # Arguments
    ///
    /// * `content` - Log line content (without trailing newline)
    /// * `stream` - Stream identifier ("stdout", "stderr", "build")
    pub fn try_write(&self, content: String, stream: &str) -> std::result::Result<(), mpsc::error::TrySendError<()>> {
        self.tx
            .try_send(LogMessage::Line {
                content,
                stream: stream.to_string(),
            })
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => mpsc::error::TrySendError::Full(()),
                mpsc::error::TrySendError::Closed(_) => mpsc::error::TrySendError::Closed(()),
            })
    }

    /// Signal that the log stream is complete.
    ///
    /// This flushes remaining logs and writes a completion marker.
    /// After calling this, no more logs should be written.
    ///
    /// # Arguments
    ///
    /// * `status` - Final job status ("success", "failed", "cancelled")
    pub async fn complete(&self, status: &str) -> std::result::Result<(), mpsc::error::SendError<()>> {
        self.tx
            .send(LogMessage::Complete {
                status: status.to_string(),
            })
            .await
            .map_err(|_| mpsc::error::SendError(()))
    }

    /// Check if the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_key_format() {
        // Verify zero-padded key format for lexicographic ordering
        let prefix = CI_LOG_KV_PREFIX;
        let key0 = format!("{}run1:job1:{:010}", prefix, 0u32);
        let key9 = format!("{}run1:job1:{:010}", prefix, 9u32);
        let key99 = format!("{}run1:job1:{:010}", prefix, 99u32);
        let key999 = format!("{}run1:job1:{:010}", prefix, 999u32);

        // Lexicographic order should match numeric order
        assert!(key0 < key9);
        assert!(key9 < key99);
        assert!(key99 < key999);
    }

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
