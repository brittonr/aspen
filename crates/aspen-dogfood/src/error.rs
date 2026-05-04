//! Error types for the dogfood orchestrator.

use snafu::Snafu;

pub type DogfoodResult<T> = Result<T, DogfoodError>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum DogfoodError {
    /// Client RPC call failed.
    #[snafu(display("RPC {operation} failed for {target}: {source}"))]
    ClientRpc {
        operation: String,
        target: String,
        source: anyhow::Error,
    },

    /// Failed to spawn a child process.
    #[snafu(display("failed to spawn {binary}: {source}"))]
    ProcessSpawn { binary: String, source: std::io::Error },

    /// A managed node process exited unexpectedly.
    #[snafu(display("node (pid {pid}) crashed: {stderr}"))]
    NodeCrash { pid: u32, stderr: String },

    /// State file I/O error.
    #[snafu(display("{operation} state file {path}: {source}"))]
    StateFile {
        operation: String,
        path: String,
        source: std::io::Error,
    },

    /// State file deserialization error.
    #[snafu(display("corrupt state file {path}: {source}"))]
    StateDeserialize { path: String, source: serde_json::Error },

    /// State file serialization error.
    #[snafu(display("serializing state file: {source}"))]
    StateSerialize { source: serde_json::Error },

    /// An operation exceeded its timeout.
    #[snafu(display("{operation} timed out after {timeout_secs}s"))]
    Timeout { operation: String, timeout_secs: u64 },

    /// Health check failed after exhausting retries.
    #[snafu(display("health check failed for node at {target}: {reason}"))]
    HealthCheck { target: String, reason: String },

    /// CI pipeline did not succeed.
    #[snafu(display("CI pipeline {run_id} {status}: {detail}"))]
    CiPipeline {
        run_id: String,
        status: String,
        detail: String,
    },

    /// Deployment did not succeed.
    #[snafu(display("deployment failed: {reason}"))]
    DeployFailed { reason: String },

    /// Forge operation failed.
    #[snafu(display("forge {operation}: {reason}"))]
    Forge { operation: String, reason: String },

    /// Dogfood receipt operation failed.
    #[snafu(display("receipt {operation}: {reason}"))]
    Receipt { operation: String, reason: String },

    /// Federation orchestration failed.
    #[snafu(display("federation {operation}: {reason}"))]
    Federation { operation: String, reason: String },

    /// Git push subprocess failed.
    #[snafu(display("git push failed (exit {exit_code}): {stderr}"))]
    GitPush { exit_code: i32, stderr: String },

    /// No state file found (cluster not started).
    #[snafu(display("no running cluster — run `aspen-dogfood start` first"))]
    NoCluster,

    /// Process signal/kill error (used for future multi-node stop).
    /// Currently nodes are stopped via `libc::kill` in `node::stop_nodes_by_pids`.
    #[snafu(display("stopping node pid {pid}: {source}"))]
    #[allow(dead_code)]
    StopNode { pid: u32, source: std::io::Error },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_client_rpc() {
        let err = DogfoodError::ClientRpc {
            operation: "GetHealth".to_string(),
            target: "node1".to_string(),
            source: anyhow::anyhow!("connection refused"),
        };
        let msg = err.to_string();
        assert!(msg.contains("GetHealth"));
        assert!(msg.contains("node1"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn error_display_process_spawn() {
        let err = DogfoodError::ProcessSpawn {
            binary: "/nix/store/.../aspen-node".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "No such file"),
        };
        let msg = err.to_string();
        assert!(msg.contains("aspen-node"));
        assert!(msg.contains("No such file"));
    }

    #[test]
    fn error_display_timeout() {
        let err = DogfoodError::Timeout {
            operation: "CI pipeline abc123".to_string(),
            timeout_secs: 600,
        };
        let msg = err.to_string();
        assert!(msg.contains("CI pipeline abc123"));
        assert!(msg.contains("600s"));
    }

    #[test]
    fn error_display_no_cluster() {
        let err = DogfoodError::NoCluster;
        let msg = err.to_string();
        assert!(msg.contains("aspen-dogfood start"));
    }

    #[test]
    fn error_display_health_check() {
        let err = DogfoodError::HealthCheck {
            target: "[REDACTED ticket; bytes=42]".to_string(),
            reason: "not healthy after 30s".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("[REDACTED ticket; bytes=42]"));
        assert!(msg.contains("not healthy"));
    }
}
