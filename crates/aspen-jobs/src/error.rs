//! Error types for the job queue system.

use std::time::Duration;

use snafu::Snafu;

/// Result type for job operations.
pub type Result<T, E = JobError> = std::result::Result<T, E>;

/// Errors that can occur in the job system.
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum JobError {
    /// Job not found.
    #[snafu(display("Job not found: {id}"))]
    JobNotFound {
        /// Job ID that was not found.
        id: String,
    },

    /// Worker not found.
    #[snafu(display("Worker not found: {id}"))]
    WorkerNotFound {
        /// Worker ID that was not found.
        id: String,
    },

    /// Job already exists.
    #[snafu(display("Job already exists: {id}"))]
    JobAlreadyExists {
        /// Job ID that already exists.
        id: String,
    },

    /// Job is in an invalid state for the operation.
    #[snafu(display("Invalid job state: {state} for operation: {operation}"))]
    InvalidJobState {
        /// Current job state.
        state: String,
        /// Operation that was attempted.
        operation: String,
    },

    /// Job execution failed.
    #[snafu(display("Job execution failed: {reason}"))]
    ExecutionFailed {
        /// Failure reason.
        reason: String,
    },

    /// Job timed out.
    #[snafu(display("Job timed out after {:?}", timeout))]
    JobTimeout {
        /// Timeout duration.
        timeout: Duration,
    },

    /// Worker is unavailable.
    #[snafu(display("No workers available for job type: {job_type}"))]
    NoWorkersAvailable {
        /// Job type that could not be scheduled.
        job_type: String,
    },

    /// Serialization error.
    #[snafu(display("Serialization error: {}", source))]
    SerializationError {
        /// Source error.
        source: serde_json::Error,
    },

    /// Storage error.
    #[snafu(display("Storage error: {}", source))]
    StorageError {
        /// Source error.
        source: aspen_core::KeyValueStoreError,
    },

    /// Queue error.
    #[snafu(display("Queue error: {}", source))]
    QueueError {
        /// Source error.
        source: anyhow::Error,
    },

    /// Invalid job specification.
    #[snafu(display("Invalid job specification: {reason}"))]
    InvalidJobSpec {
        /// Reason the spec is invalid.
        reason: String,
    },

    /// Dependency not satisfied.
    #[snafu(display("Job dependency not satisfied: {dependency}"))]
    DependencyNotSatisfied {
        /// Missing dependency.
        dependency: String,
    },

    /// Rate limit exceeded.
    #[snafu(display("Rate limit exceeded for job type: {job_type}"))]
    RateLimitExceeded {
        /// Job type that hit the rate limit.
        job_type: String,
    },

    /// Worker registration failed.
    #[snafu(display("Worker registration failed: {reason}"))]
    WorkerRegistrationFailed {
        /// Failure reason.
        reason: String,
    },

    /// Worker communication failed.
    #[snafu(display("Worker communication failed: {reason}"))]
    WorkerCommunicationFailed {
        /// Failure reason.
        reason: String,
    },

    /// Job cancelled.
    #[snafu(display("Job was cancelled: {id}"))]
    JobCancelled {
        /// Cancelled job ID.
        id: String,
    },

    /// Build failed.
    #[snafu(display("Build failed: {reason}"))]
    BuildFailed {
        /// Failure reason.
        reason: String,
    },

    /// Binary too large.
    #[snafu(display("Binary too large: {size} bytes (max: {max} bytes)"))]
    BinaryTooLarge {
        /// Actual size.
        size: usize,
        /// Maximum allowed size.
        max: usize,
    },

    /// VM execution failed.
    #[snafu(display("VM execution failed: {reason}"))]
    VmExecutionFailed {
        /// Failure reason.
        reason: String,
    },
}

/// Error kinds for categorizing errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobErrorKind {
    /// Temporary error that can be retried.
    Temporary,
    /// Permanent error that should not be retried.
    Permanent,
    /// Resource exhaustion error.
    ResourceExhausted,
    /// Invalid input or configuration.
    InvalidInput,
}

impl JobError {
    /// Get the error kind for retry logic.
    pub fn kind(&self) -> JobErrorKind {
        match self {
            Self::JobNotFound { .. } => JobErrorKind::Permanent,
            Self::WorkerNotFound { .. } => JobErrorKind::Temporary,
            Self::JobAlreadyExists { .. } => JobErrorKind::Permanent,
            Self::InvalidJobState { .. } => JobErrorKind::InvalidInput,
            Self::ExecutionFailed { .. } => JobErrorKind::Temporary,
            Self::JobTimeout { .. } => JobErrorKind::Temporary,
            Self::NoWorkersAvailable { .. } => JobErrorKind::ResourceExhausted,
            Self::SerializationError { .. } => JobErrorKind::Permanent,
            Self::StorageError { .. } => JobErrorKind::Temporary,
            Self::QueueError { .. } => JobErrorKind::Temporary,
            Self::InvalidJobSpec { .. } => JobErrorKind::InvalidInput,
            Self::DependencyNotSatisfied { .. } => JobErrorKind::Temporary,
            Self::RateLimitExceeded { .. } => JobErrorKind::ResourceExhausted,
            Self::WorkerRegistrationFailed { .. } => JobErrorKind::Temporary,
            Self::WorkerCommunicationFailed { .. } => JobErrorKind::Temporary,
            Self::JobCancelled { .. } => JobErrorKind::Permanent,
            Self::BuildFailed { .. } => JobErrorKind::Temporary,
            Self::BinaryTooLarge { .. } => JobErrorKind::Permanent,
            Self::VmExecutionFailed { .. } => JobErrorKind::Temporary,
        }
    }

    /// Check if the error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(self.kind(), JobErrorKind::Temporary | JobErrorKind::ResourceExhausted)
    }
}

// Automatic conversions for common error types
impl From<serde_json::Error> for JobError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError { source: err }
    }
}

impl From<aspen_core::KeyValueStoreError> for JobError {
    fn from(err: aspen_core::KeyValueStoreError) -> Self {
        Self::StorageError { source: err }
    }
}

impl From<anyhow::Error> for JobError {
    fn from(err: anyhow::Error) -> Self {
        Self::QueueError { source: err }
    }
}
