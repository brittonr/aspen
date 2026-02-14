//! Output reference types and log consumer utilities.

use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;
use tracing::debug;

use crate::agent::protocol::LogMessage;

/// Inline log threshold (256 KB).
/// This is the maximum size of stdout/stderr we keep for inline display.
/// We keep the TAIL of output to preserve error messages that typically appear at the end.
pub(crate) const INLINE_LOG_THRESHOLD: usize = 256 * 1024;

/// Marker prepended when output is truncated.
pub(crate) const TRUNCATION_MARKER: &str = "...[truncated - showing last 256 KB of output]...\n";

/// Reference to job output (inline or stored in blob store).
///
/// This is a local copy of aspen_jobs::OutputRef to avoid dependency on shell-worker feature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputRef {
    /// Output stored inline (small outputs <= 64KB).
    Inline {
        /// The inline content.
        content: String,
    },
    /// Output stored in blob store (large outputs).
    Blob {
        /// Hex-encoded blake3 hash of the blob.
        hash: String,
        /// Size in bytes.
        size: u64,
    },
}

/// Spawn a task to consume log messages and accumulate stdout/stderr.
///
/// This keeps the TAIL of output when it exceeds INLINE_LOG_THRESHOLD,
/// because error messages typically appear at the end of build output.
pub(crate) fn spawn_log_consumer(
    job_id: String,
    mut log_rx: mpsc::Receiver<LogMessage>,
) -> tokio::task::JoinHandle<(String, String)> {
    tokio::spawn(async move {
        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut stdout_truncated = false;
        let mut stderr_truncated = false;

        while let Some(msg) = log_rx.recv().await {
            match msg {
                LogMessage::Stdout(data) => {
                    stdout.push_str(&data);
                    // Keep the tail when exceeding threshold
                    if stdout.len() > INLINE_LOG_THRESHOLD {
                        let start = stdout.len() - INLINE_LOG_THRESHOLD;
                        stdout = stdout[start..].to_string();
                        stdout_truncated = true;
                    }
                    debug!(job_id = %job_id, len = data.len(), "stdout chunk");
                }
                LogMessage::Stderr(data) => {
                    stderr.push_str(&data);
                    // Keep the tail when exceeding threshold
                    if stderr.len() > INLINE_LOG_THRESHOLD {
                        let start = stderr.len() - INLINE_LOG_THRESHOLD;
                        stderr = stderr[start..].to_string();
                        stderr_truncated = true;
                    }
                    debug!(job_id = %job_id, len = data.len(), "stderr chunk");
                }
                LogMessage::Heartbeat { elapsed_secs } => {
                    debug!(job_id = %job_id, elapsed_secs, "heartbeat");
                }
                LogMessage::Complete(_) => {
                    // Final result handled separately
                }
            }
        }

        // Prepend truncation marker if output was truncated
        if stdout_truncated {
            stdout = format!("{}{}", TRUNCATION_MARKER, stdout);
        }
        if stderr_truncated {
            stderr = format!("{}{}", TRUNCATION_MARKER, stderr);
        }

        (stdout, stderr)
    })
}
