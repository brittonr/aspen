//! Output capture for process execution
//!
//! Handles stdout/stderr streaming and collection from child processes

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::RwLock;

/// Captured output from process execution
#[derive(Debug, Clone)]
pub struct CapturedOutput {
    pub stdout: Vec<String>,
    pub stderr: Vec<String>,
}

impl CapturedOutput {
    /// Create empty captured output
    pub fn empty() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    /// Check if stdout has any content
    pub fn has_stdout(&self) -> bool {
        !self.stdout.is_empty()
    }

    /// Check if stderr has any content
    pub fn has_stderr(&self) -> bool {
        !self.stderr.is_empty()
    }

    /// Get stdout as joined string
    pub fn stdout_string(&self) -> String {
        self.stdout.join("\n")
    }

    /// Get stderr as joined string
    pub fn stderr_string(&self) -> String {
        self.stderr.join("\n")
    }
}

/// Handles output capture from child processes
pub struct OutputCapture {
    stdout_lines: Arc<RwLock<Vec<String>>>,
    stderr_lines: Arc<RwLock<Vec<String>>>,
}

impl OutputCapture {
    /// Create a new output capture handler
    pub fn new() -> Self {
        Self {
            stdout_lines: Arc::new(RwLock::new(Vec::new())),
            stderr_lines: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start capturing stdout from child process
    ///
    /// Uses batching to reduce lock contention - accumulates lines in a local
    /// buffer and flushes every 50 lines or on completion.
    pub fn capture_stdout(&self, stdout: ChildStdout) {
        let lines = self.stdout_lines.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut line_stream = reader.lines();
            let mut batch = Vec::with_capacity(50);
            const BATCH_SIZE: usize = 50;

            while let Ok(Some(line)) = line_stream.next_line().await {
                batch.push(line);

                // Flush batch when it reaches threshold
                if batch.len() >= BATCH_SIZE {
                    let mut output = lines.write().await;
                    output.extend(batch.drain(..));
                }
            }

            // Flush remaining lines
            if !batch.is_empty() {
                let mut output = lines.write().await;
                output.extend(batch);
            }
        });
    }

    /// Start capturing stderr from child process
    ///
    /// Uses batching to reduce lock contention - accumulates lines in a local
    /// buffer and flushes every 50 lines or on completion.
    pub fn capture_stderr(&self, stderr: ChildStderr) {
        let lines = self.stderr_lines.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut line_stream = reader.lines();
            let mut batch = Vec::with_capacity(50);
            const BATCH_SIZE: usize = 50;

            while let Ok(Some(line)) = line_stream.next_line().await {
                batch.push(line);

                // Flush batch when it reaches threshold
                if batch.len() >= BATCH_SIZE {
                    let mut errors = lines.write().await;
                    errors.extend(batch.drain(..));
                }
            }

            // Flush remaining lines
            if !batch.is_empty() {
                let mut errors = lines.write().await;
                errors.extend(batch);
            }
        });
    }

    /// Get all captured output
    pub async fn get_output(&self) -> CapturedOutput {
        CapturedOutput {
            stdout: self.stdout_lines.read().await.clone(),
            stderr: self.stderr_lines.read().await.clone(),
        }
    }
}

impl Default for OutputCapture {
    fn default() -> Self {
        Self::new()
    }
}
