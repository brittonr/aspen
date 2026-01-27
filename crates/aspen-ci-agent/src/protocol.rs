//! Protocol types for host-guest communication over vsock.
//!
//! This module defines the wire protocol between the CloudHypervisorWorker (host)
//! and the aspen-ci-agent (guest). Communication uses length-prefixed JSON frames
//! over a vsock stream.
//!
//! ## Frame Format
//! ```text
//! +----------------+------------------+
//! | Length (4 BE)  | JSON payload     |
//! +----------------+------------------+
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Request from host to execute a command in the guest VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Unique job ID for correlation.
    pub id: String,

    /// Command to execute (absolute path or PATH lookup).
    pub command: String,

    /// Command arguments.
    pub args: Vec<String>,

    /// Working directory for execution.
    /// Must be under /workspace (virtiofs mount point).
    pub working_dir: PathBuf,

    /// Environment variables to set.
    pub env: HashMap<String, String>,

    /// Execution timeout in seconds.
    /// The agent will kill the process if it exceeds this.
    pub timeout_secs: u64,
}

/// Final result of command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Job ID (echoed from request).
    pub id: String,

    /// Process exit code (0 = success).
    pub exit_code: i32,

    /// Standard output (if not streaming).
    pub stdout: String,

    /// Standard error (if not streaming).
    pub stderr: String,

    /// Execution duration in milliseconds.
    pub duration_ms: u64,

    /// Error message if execution failed before completion.
    pub error: Option<String>,
}

/// Log message streamed during execution.
///
/// The agent sends these incrementally as output is produced,
/// allowing the host to stream logs to the CI system in real-time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum LogMessage {
    /// Chunk of stdout data.
    Stdout(String),

    /// Chunk of stderr data.
    Stderr(String),

    /// Execution completed with final result.
    Complete(ExecutionResult),

    /// Heartbeat to indicate agent is alive during long operations.
    Heartbeat { elapsed_secs: u64 },
}

/// Control messages from host to agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HostMessage {
    /// Execute a command.
    Execute(ExecutionRequest),

    /// Cancel a running job.
    Cancel { id: String },

    /// Check if agent is alive.
    Ping,

    /// Request agent shutdown.
    Shutdown,
}

/// Response messages from agent to host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentMessage {
    /// Log output from execution.
    Log(LogMessage),

    /// Response to ping.
    Pong,

    /// Agent is ready to accept jobs.
    Ready,

    /// Error response.
    Error { message: String },
}

/// Vsock connection parameters.
pub mod vsock {
    /// Default vsock port for the CI agent.
    /// Port 5000 is in the unprivileged range (>1024).
    pub const DEFAULT_PORT: u32 = 5000;

    /// CID for the host (always 2 in vsock).
    pub const HOST_CID: u32 = 2;

    /// CID for "any" (used for listening).
    pub const ANY_CID: u32 = u32::MAX;
}

/// Maximum message size (16 MB).
/// Prevents memory exhaustion from malformed frames.
pub const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_request_roundtrip() {
        let request = ExecutionRequest {
            id: "job-123".to_string(),
            command: "nix".to_string(),
            args: vec!["build".to_string(), ".#default".to_string()],
            working_dir: PathBuf::from("/workspace/project"),
            env: HashMap::from([
                ("HOME".to_string(), "/root".to_string()),
                ("NIX_CONFIG".to_string(), "experimental-features = nix-command flakes".to_string()),
            ]),
            timeout_secs: 3600,
        };

        let json = serde_json::to_string(&request).unwrap();
        let decoded: ExecutionRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.id, request.id);
        assert_eq!(decoded.command, request.command);
        assert_eq!(decoded.args, request.args);
    }

    #[test]
    fn test_log_message_variants() {
        let stdout = LogMessage::Stdout("Building...".to_string());
        let json = serde_json::to_string(&stdout).unwrap();
        assert!(json.contains("\"type\":\"Stdout\""));

        let result = ExecutionResult {
            id: "job-123".to_string(),
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 1500,
            error: None,
        };
        let complete = LogMessage::Complete(result);
        let json = serde_json::to_string(&complete).unwrap();
        assert!(json.contains("\"type\":\"Complete\""));
    }

    #[test]
    fn test_host_message_tagged() {
        let msg = HostMessage::Ping;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"Ping"}"#);

        let msg = HostMessage::Cancel { id: "job-456".to_string() };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"Cancel\""));
        assert!(json.contains("\"id\":\"job-456\""));
    }
}
