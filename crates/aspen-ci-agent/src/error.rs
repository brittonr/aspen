//! Error types for the CI agent.

use snafu::Snafu;

/// Errors that can occur in the CI agent.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum AgentError {
    /// Failed to bind vsock listener.
    #[snafu(display("failed to bind vsock listener on port {port}: {source}"))]
    BindVsock {
        port: u32,
        source: std::io::Error,
    },

    /// Failed to accept vsock connection.
    #[snafu(display("failed to accept vsock connection: {source}"))]
    AcceptConnection {
        source: std::io::Error,
    },

    /// Failed to read from vsock stream.
    #[snafu(display("failed to read from vsock: {source}"))]
    ReadVsock {
        source: std::io::Error,
    },

    /// Failed to write to vsock stream.
    #[snafu(display("failed to write to vsock: {source}"))]
    WriteVsock {
        source: std::io::Error,
    },

    /// Message exceeded maximum size.
    #[snafu(display("message size {size} exceeds maximum {max}"))]
    MessageTooLarge {
        size: u32,
        max: u32,
    },

    /// Failed to deserialize message.
    #[snafu(display("failed to deserialize message: {source}"))]
    DeserializeMessage {
        source: serde_json::Error,
    },

    /// Failed to serialize message.
    #[snafu(display("failed to serialize message: {source}"))]
    SerializeMessage {
        source: serde_json::Error,
    },

    /// Failed to spawn process.
    #[snafu(display("failed to spawn process '{command}': {source}"))]
    SpawnProcess {
        command: String,
        source: std::io::Error,
    },

    /// Process execution timed out.
    #[snafu(display("process execution timed out after {timeout_secs} seconds"))]
    ExecutionTimeout {
        timeout_secs: u64,
    },

    /// Invalid working directory.
    #[snafu(display("invalid working directory: {path}"))]
    InvalidWorkingDir {
        path: String,
    },

    /// Working directory must be under /workspace.
    #[snafu(display("working directory must be under /workspace, got: {path}"))]
    WorkingDirNotUnderWorkspace {
        path: String,
    },

    /// Command not found.
    #[snafu(display("command not found: {command}"))]
    CommandNotFound {
        command: String,
    },

    /// Failed to send cancellation signal.
    #[snafu(display("failed to send signal to process: {source}"))]
    SignalProcess {
        source: std::io::Error,
    },

    /// Job not found for cancellation.
    #[snafu(display("job not found: {id}"))]
    JobNotFound {
        id: String,
    },
}

/// Result type for agent operations.
pub type Result<T> = std::result::Result<T, AgentError>;
