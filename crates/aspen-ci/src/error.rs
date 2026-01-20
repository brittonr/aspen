//! Error types for the CI/CD system.

use std::path::PathBuf;

use snafu::Snafu;

/// Result type for CI operations.
pub type Result<T> = std::result::Result<T, CiError>;

/// CI/CD system errors.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CiError {
    /// Pipeline configuration file not found.
    #[snafu(display("Pipeline config not found: {}", path.display()))]
    ConfigNotFound {
        /// Path that was searched.
        path: PathBuf,
    },

    /// Pipeline configuration file too large.
    #[snafu(display("Pipeline config too large: {size} bytes (max: {max})"))]
    ConfigTooLarge {
        /// Actual size.
        size: u64,
        /// Maximum allowed.
        max: u64,
    },

    /// Failed to read configuration file.
    #[snafu(display("Failed to read config file {}: {source}", path.display()))]
    ReadConfig {
        /// Path to the file.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },

    /// Nickel evaluation error.
    #[snafu(display("Nickel evaluation error: {message}"))]
    NickelEvaluation {
        /// Error message from Nickel.
        message: String,
    },

    /// Configuration deserialization error.
    #[snafu(display("Config deserialization error: {message}"))]
    Deserialization {
        /// Error message.
        message: String,
    },

    /// Invalid pipeline configuration.
    #[snafu(display("Invalid pipeline config: {reason}"))]
    InvalidConfig {
        /// Reason for invalidity.
        reason: String,
    },

    /// Stage not found in pipeline.
    #[snafu(display("Stage not found: {stage}"))]
    StageNotFound {
        /// Stage name.
        stage: String,
    },

    /// Job not found in pipeline.
    #[snafu(display("Job not found: {job}"))]
    JobNotFound {
        /// Job name.
        job: String,
    },

    /// Circular dependency detected in pipeline.
    #[snafu(display("Circular dependency: {path}"))]
    CircularDependency {
        /// Dependency path showing the cycle.
        path: String,
    },

    /// Pipeline execution failed.
    #[snafu(display("Pipeline execution failed: {reason}"))]
    ExecutionFailed {
        /// Failure reason.
        reason: String,
    },

    /// Job execution failed.
    #[snafu(display("Job '{job}' failed: {reason}"))]
    JobFailed {
        /// Job name.
        job: String,
        /// Failure reason.
        reason: String,
    },

    /// Nix build failed.
    #[snafu(display("Nix build failed for {flake}: {reason}"))]
    NixBuildFailed {
        /// Flake URL.
        flake: String,
        /// Failure reason.
        reason: String,
    },

    /// Artifact storage error.
    #[snafu(display("Artifact storage error: {reason}"))]
    ArtifactStorage {
        /// Error reason.
        reason: String,
    },

    /// Trigger subscription error.
    #[snafu(display("Trigger subscription failed: {reason}"))]
    TriggerSubscription {
        /// Error reason.
        reason: String,
    },

    /// Forge operation error.
    #[snafu(display("Forge operation failed: {reason}"))]
    ForgeOperation {
        /// Error reason.
        reason: String,
    },

    /// Job system error.
    #[snafu(display("Job system error: {reason}"))]
    JobSystem {
        /// Error reason.
        reason: String,
    },

    /// Workflow error.
    #[snafu(display("Workflow error: {reason}"))]
    Workflow {
        /// Error reason.
        reason: String,
    },

    /// Timeout exceeded.
    #[snafu(display("Operation timed out after {timeout_secs} seconds"))]
    Timeout {
        /// Timeout duration in seconds.
        timeout_secs: u64,
    },

    /// Pipeline cancelled.
    #[snafu(display("Pipeline cancelled: {reason}"))]
    Cancelled {
        /// Cancellation reason.
        reason: String,
    },
}

impl From<aspen_nickel::NickelConfigError> for CiError {
    fn from(err: aspen_nickel::NickelConfigError) -> Self {
        CiError::NickelEvaluation {
            message: err.to_string(),
        }
    }
}
