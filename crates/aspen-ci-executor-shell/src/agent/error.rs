//! Error types for the CI agent.

use snafu::Snafu;

/// Errors that can occur in the CI agent.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AgentError {
    /// Failed to bind vsock listener.
    #[snafu(display("failed to bind vsock listener on port {port}: {source}"))]
    BindVsock { port: u32, source: std::io::Error },

    /// Failed to accept vsock connection.
    #[snafu(display("failed to accept vsock connection: {source}"))]
    AcceptConnection { source: std::io::Error },

    /// Failed to read from vsock stream.
    #[snafu(display("failed to read from vsock: {source}"))]
    ReadVsock { source: std::io::Error },

    /// Failed to write to vsock stream.
    #[snafu(display("failed to write to vsock: {source}"))]
    WriteVsock { source: std::io::Error },

    /// Message exceeded maximum size.
    #[snafu(display("message size {size} exceeds maximum {max}"))]
    MessageTooLarge { size: u32, max: u32 },

    /// Failed to deserialize message.
    #[snafu(display("failed to deserialize message: {source}"))]
    DeserializeMessage { source: serde_json::Error },

    /// Failed to serialize message.
    #[snafu(display("failed to serialize message: {source}"))]
    SerializeMessage { source: serde_json::Error },

    /// Failed to spawn process.
    #[snafu(display("failed to spawn process '{command}': {source}"))]
    SpawnProcess { command: String, source: std::io::Error },

    /// Process execution timed out.
    #[snafu(display("process execution timed out after {timeout_secs} seconds"))]
    ExecutionTimeout { timeout_secs: u64 },

    /// Invalid working directory.
    #[snafu(display("invalid working directory: {path}"))]
    InvalidWorkingDir { path: String },

    /// Working directory must be under /workspace.
    #[snafu(display("working directory must be under /workspace, got: {path}"))]
    WorkingDirNotUnderWorkspace { path: String },

    /// Command not found.
    #[snafu(display("command not found: {command}"))]
    CommandNotFound { command: String },

    /// Failed to send cancellation signal.
    #[snafu(display("failed to send signal to process: {source}"))]
    SignalProcess { source: std::io::Error },

    /// Job not found for cancellation.
    #[snafu(display("job not found: {id}"))]
    JobNotFound { id: String },
}

/// Result type for agent operations.
pub type Result<T> = std::result::Result<T, AgentError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_bind_vsock() {
        let err = AgentError::BindVsock {
            port: 5000,
            source: std::io::Error::new(std::io::ErrorKind::AddrInUse, "address in use"),
        };
        let msg = err.to_string();
        assert!(msg.contains("5000"));
        assert!(msg.contains("bind"));
    }

    #[test]
    fn test_error_display_message_too_large() {
        let err = AgentError::MessageTooLarge {
            size: 20_000_000,
            max: 16_000_000,
        };
        let msg = err.to_string();
        assert!(msg.contains("20000000"));
        assert!(msg.contains("16000000"));
    }

    #[test]
    fn test_error_display_execution_timeout() {
        let err = AgentError::ExecutionTimeout { timeout_secs: 3600 };
        let msg = err.to_string();
        assert!(msg.contains("3600"));
        assert!(msg.contains("timed out"));
    }

    #[test]
    fn test_error_display_working_dir_not_under_workspace() {
        let err = AgentError::WorkingDirNotUnderWorkspace {
            path: "/tmp/evil".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("/tmp/evil"));
        assert!(msg.contains("/workspace"));
    }

    #[test]
    fn test_error_display_job_not_found() {
        let err = AgentError::JobNotFound {
            id: "job-12345".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("job-12345"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_error_display_spawn_process() {
        let err = AgentError::SpawnProcess {
            command: "/bin/nonexistent".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let msg = err.to_string();
        assert!(msg.contains("/bin/nonexistent"));
        assert!(msg.contains("spawn"));
    }

    #[test]
    fn test_error_is_debug() {
        let err = AgentError::CommandNotFound {
            command: "foo".to_string(),
        };
        // Verify Debug is implemented
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("CommandNotFound"));
    }
}
