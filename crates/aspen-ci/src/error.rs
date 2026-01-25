//! Error types for the CI/CD system.
//!
//! This module provides structured error types using `snafu` with source error
//! chains preserved for better debugging and actionable error messages.
//!
//! # Tiger Style
//!
//! - All errors preserve source chains where applicable
//! - Errors include contextual information for debugging
//! - String-only reason fields are minimized in favor of structured context

use std::path::PathBuf;

use snafu::Snafu;

/// Result type for CI operations.
pub type Result<T> = std::result::Result<T, CiError>;

/// CI/CD system errors.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum CiError {
    // ========================================================================
    // Configuration Errors
    // ========================================================================
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

    // ========================================================================
    // Execution Errors
    // ========================================================================
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

    // ========================================================================
    // Forge Errors (with source chain preservation)
    // ========================================================================
    /// Failed to load git tree from Forge.
    #[snafu(display("Failed to load tree {tree_hash}: {source}"))]
    LoadTreeFailed {
        /// Tree hash that was being loaded.
        tree_hash: String,
        /// Underlying Forge error.
        source: aspen_forge::ForgeError,
    },

    /// Failed to load git blob from Forge.
    #[snafu(display("Failed to load blob {blob_hash}: {source}"))]
    LoadBlobFailed {
        /// Blob hash that was being loaded.
        blob_hash: String,
        /// Underlying Forge error.
        source: aspen_forge::ForgeError,
    },

    /// Forge operation error (generic, for backwards compatibility).
    #[snafu(display("Forge operation failed: {reason}"))]
    ForgeOperation {
        /// Error reason.
        reason: String,
    },

    // ========================================================================
    // Checkout Errors (with source chain preservation)
    // ========================================================================
    /// Failed to create checkout directory.
    #[snafu(display("Failed to create checkout directory {}: {source}", path.display()))]
    CreateCheckoutDir {
        /// Path where directory creation failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },

    /// Failed to write file during checkout.
    #[snafu(display("Failed to write file {}: {source}", path.display()))]
    WriteCheckoutFile {
        /// Path where write failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },

    /// Failed to set file permissions during checkout.
    #[snafu(display("Failed to set permissions on {}: {source}", path.display()))]
    SetCheckoutPermissions {
        /// Path where permission setting failed.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },

    /// Failed to clean up checkout directory.
    #[snafu(display("Failed to clean up checkout directory {}: {source}", path.display()))]
    CleanupCheckout {
        /// Path of checkout directory.
        path: PathBuf,
        /// Underlying IO error.
        source: std::io::Error,
    },

    /// Checkout resource limit exceeded.
    #[snafu(display("Checkout limit exceeded: {reason}"))]
    CheckoutLimitExceeded {
        /// Description of the limit that was exceeded.
        reason: String,
    },

    /// Repository checkout error (generic, for backwards compatibility).
    #[snafu(display("Checkout failed: {reason}"))]
    Checkout {
        /// Error reason.
        reason: String,
    },

    // ========================================================================
    // Artifact & Storage Errors
    // ========================================================================
    /// Artifact storage error.
    #[snafu(display("Artifact storage error: {reason}"))]
    ArtifactStorage {
        /// Error reason.
        reason: String,
    },

    /// Log write error during CI job execution.
    #[snafu(display("Failed to write CI log: {reason}"))]
    LogWrite {
        /// Error reason.
        reason: String,
    },

    /// Log serialization error.
    #[snafu(display("Failed to serialize CI log chunk: {reason}"))]
    LogSerialization {
        /// Error reason.
        reason: String,
    },

    // ========================================================================
    // Trigger & Workflow Errors
    // ========================================================================
    /// Trigger subscription error.
    #[snafu(display("Trigger subscription failed: {reason}"))]
    TriggerSubscription {
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

    // ========================================================================
    // Replication Errors
    // ========================================================================
    /// Object not yet replicated (transient failure, should retry).
    #[snafu(display("{object_type} {hash} not yet replicated (attempt {attempt}/{max_attempts})"))]
    ObjectNotReplicated {
        /// Type of object (commit, tree, blob).
        object_type: String,
        /// Hash of the object.
        hash: String,
        /// Current retry attempt.
        attempt: u32,
        /// Maximum retry attempts.
        max_attempts: u32,
    },

    /// Object permanently missing after all retries.
    #[snafu(display("{object_type} {hash} not found after {attempts} attempts"))]
    ObjectPermanentlyMissing {
        /// Type of object (commit, tree, blob).
        object_type: String,
        /// Hash of the object.
        hash: String,
        /// Number of attempts made.
        attempts: u32,
    },
}

impl From<aspen_nickel::NickelConfigError> for CiError {
    fn from(err: aspen_nickel::NickelConfigError) -> Self {
        CiError::NickelEvaluation {
            message: err.to_string(),
        }
    }
}
