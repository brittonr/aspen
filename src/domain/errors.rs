//! Domain Errors Module
//!
//! This module defines typed errors for the domain layer, replacing generic anyhow::Result
//! with specific, actionable error types that can be handled appropriately.

use crate::domain::types::{JobStatus, WorkerType};
use std::fmt;

/// Result type for domain operations
pub type DomainResult<T> = Result<T, DomainError>;

/// Main domain error type
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// Job-related errors
    #[error("Job error: {0}")]
    Job(#[from] JobError),

    /// Worker-related errors
    #[error("Worker error: {0}")]
    Worker(#[from] WorkerError),

    /// State machine errors
    #[error("State machine error: {0}")]
    StateMachine(#[from] StateError),

    /// Validation errors
    #[error("Validation failed: {0}")]
    Validation(#[from] ValidationError),

    /// Resource errors
    #[error("Resource error: {0}")]
    Resource(#[from] ResourceError),

    /// Infrastructure errors (wrapped)
    #[error("Infrastructure error: {0}")]
    Infrastructure(String),

    /// Generic domain error
    #[error("{0}")]
    Generic(String),
}

/// Job-specific errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum JobError {
    #[error("Job not found: {id}")]
    NotFound { id: String },

    #[error("Job already exists: {id}")]
    AlreadyExists { id: String },

    #[error("Job already claimed: {id}")]
    AlreadyClaimed { id: String },

    #[error("Job in invalid state: {id} is {status:?}")]
    InvalidState { id: String, status: JobStatus },

    #[error("Job incompatible with worker: {job_id} cannot be claimed by {worker_type:?}")]
    IncompatibleWorker {
        job_id: String,
        worker_type: WorkerType,
    },

    #[error("Job execution failed: {id} - {reason}")]
    ExecutionFailed { id: String, reason: String },

    #[error("Job retry limit exceeded: {id} (retries: {retry_count})")]
    RetryLimitExceeded { id: String, retry_count: u32 },
}

/// Worker-specific errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum WorkerError {
    #[error("Worker not found: {id}")]
    NotFound { id: String },

    #[error("Worker already registered: {id}")]
    AlreadyRegistered { id: String },

    #[error("Worker not available: {id} is {reason}")]
    NotAvailable { id: String, reason: String },

    #[error("Worker capacity exceeded: {id} (current: {current}, max: {max})")]
    CapacityExceeded {
        id: String,
        current: usize,
        max: usize,
    },

    #[error("Worker heartbeat timeout: {id} (last seen: {last_seen_secs}s ago)")]
    HeartbeatTimeout { id: String, last_seen_secs: i64 },

    #[error("Worker type mismatch: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        expected: WorkerType,
        actual: WorkerType,
    },
}

/// State machine errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum StateError {
    #[error("Invalid state transition: {from:?} → {to:?}")]
    InvalidTransition { from: JobStatus, to: JobStatus },

    #[error("Terminal state reached: cannot transition from {state:?}")]
    TerminalState { state: JobStatus },

    #[error("State precondition failed: {reason}")]
    PreconditionFailed { reason: String },

    #[error("State invariant violated: {invariant}")]
    InvariantViolated { invariant: String },
}

/// Validation errors
#[derive(Debug, Clone)]
pub enum ValidationError {
    InvalidJobId { reason: String },
    InvalidPayload { reason: String },
    MissingField { field: String },
    InvalidTimestamp { field: String, reason: String },
    InvalidWorkerConstraints { reason: String },
    Multiple(Vec<ValidationError>),
}

impl ValidationError {
    /// Combine multiple validation errors into one
    pub fn combine(errors: Vec<ValidationError>) -> Self {
        match errors.len() {
            0 => panic!("Cannot combine empty error list"),
            1 => errors.into_iter().next().expect("Vector has exactly one element"),
            _ => ValidationError::Multiple(errors),
        }
    }
}

/// Resource-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResourceError {
    #[error("Resource exhausted: {resource} - {reason}")]
    Exhausted { resource: String, reason: String },

    #[error("Resource not available: {resource}")]
    NotAvailable { resource: String },

    #[error("Resource allocation failed: {resource} - {reason}")]
    AllocationFailed { resource: String, reason: String },

    #[error("Resource limit exceeded: {resource} (requested: {requested}, available: {available})")]
    LimitExceeded {
        resource: String,
        requested: usize,
        available: usize,
    },

    #[error("Resource locked: {resource} (locked by: {owner})")]
    Locked { resource: String, owner: String },
}

/// Conversion from anyhow::Error to DomainError
impl From<anyhow::Error> for DomainError {
    fn from(err: anyhow::Error) -> Self {
        // Try to downcast to known error types
        if let Some(job_err) = err.downcast_ref::<JobError>() {
            return DomainError::Job(job_err.clone());
        }
        if let Some(worker_err) = err.downcast_ref::<WorkerError>() {
            return DomainError::Worker(worker_err.clone());
        }
        if let Some(state_err) = err.downcast_ref::<StateError>() {
            return DomainError::StateMachine(state_err.clone());
        }
        if let Some(val_err) = err.downcast_ref::<ValidationError>() {
            return DomainError::Validation(val_err.clone());
        }
        if let Some(res_err) = err.downcast_ref::<ResourceError>() {
            return DomainError::Resource(res_err.clone());
        }

        // Fall back to infrastructure error
        DomainError::Infrastructure(err.to_string())
    }
}

/// Helper trait for converting infrastructure errors to domain errors
pub trait IntoDomainError<T> {
    fn into_domain_error(self) -> Result<T, DomainError>;
}

impl<T, E> IntoDomainError<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn into_domain_error(self) -> Result<T, DomainError> {
        self.map_err(|e| DomainError::Infrastructure(e.to_string()))
    }
}

/// Error severity for logging and monitoring
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Informational - normal operation, user error
    Info,
    /// Warning - recoverable error, retry possible
    Warning,
    /// Error - operation failed, manual intervention may be needed
    Error,
    /// Critical - system integrity at risk
    Critical,
}

impl DomainError {
    /// Get the severity of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            DomainError::Job(JobError::NotFound { .. }) => ErrorSeverity::Info,
            DomainError::Job(JobError::AlreadyClaimed { .. }) => ErrorSeverity::Info,
            DomainError::Worker(WorkerError::NotAvailable { .. }) => ErrorSeverity::Warning,
            DomainError::Worker(WorkerError::HeartbeatTimeout { .. }) => ErrorSeverity::Warning,
            DomainError::StateMachine(StateError::InvalidTransition { .. }) => ErrorSeverity::Warning,
            DomainError::Validation(_) => ErrorSeverity::Info,
            DomainError::Resource(ResourceError::Exhausted { .. }) => ErrorSeverity::Error,
            DomainError::Infrastructure(_) => ErrorSeverity::Error,
            _ => ErrorSeverity::Error,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            DomainError::Worker(WorkerError::NotAvailable { .. })
                | DomainError::Resource(ResourceError::Exhausted { .. })
                | DomainError::Resource(ResourceError::Locked { .. })
                | DomainError::Infrastructure(_)
        )
    }

    /// Get a user-friendly error code
    pub fn error_code(&self) -> &str {
        match self {
            DomainError::Job(JobError::NotFound { .. }) => "JOB_NOT_FOUND",
            DomainError::Job(JobError::AlreadyExists { .. }) => "JOB_EXISTS",
            DomainError::Job(JobError::AlreadyClaimed { .. }) => "JOB_CLAIMED",
            DomainError::Job(JobError::InvalidState { .. }) => "JOB_INVALID_STATE",
            DomainError::Job(JobError::IncompatibleWorker { .. }) => "JOB_INCOMPATIBLE",
            DomainError::Job(JobError::ExecutionFailed { .. }) => "JOB_FAILED",
            DomainError::Job(JobError::RetryLimitExceeded { .. }) => "JOB_RETRY_EXCEEDED",
            DomainError::Worker(WorkerError::NotFound { .. }) => "WORKER_NOT_FOUND",
            DomainError::Worker(WorkerError::AlreadyRegistered { .. }) => "WORKER_EXISTS",
            DomainError::Worker(WorkerError::NotAvailable { .. }) => "WORKER_UNAVAILABLE",
            DomainError::Worker(WorkerError::CapacityExceeded { .. }) => "WORKER_CAPACITY",
            DomainError::Worker(WorkerError::HeartbeatTimeout { .. }) => "WORKER_TIMEOUT",
            DomainError::Worker(WorkerError::TypeMismatch { .. }) => "WORKER_TYPE_MISMATCH",
            DomainError::StateMachine(StateError::InvalidTransition { .. }) => "INVALID_TRANSITION",
            DomainError::StateMachine(StateError::TerminalState { .. }) => "TERMINAL_STATE",
            DomainError::StateMachine(StateError::PreconditionFailed { .. }) => "PRECONDITION_FAILED",
            DomainError::StateMachine(StateError::InvariantViolated { .. }) => "INVARIANT_VIOLATED",
            DomainError::Validation(ValidationError::Multiple(_)) => "VALIDATION_ERRORS",
            DomainError::Validation(_) => "VALIDATION_ERROR",
            DomainError::Resource(ResourceError::Exhausted { .. }) => "RESOURCE_EXHAUSTED",
            DomainError::Resource(ResourceError::NotAvailable { .. }) => "RESOURCE_UNAVAILABLE",
            DomainError::Resource(ResourceError::AllocationFailed { .. }) => "ALLOCATION_FAILED",
            DomainError::Resource(ResourceError::LimitExceeded { .. }) => "LIMIT_EXCEEDED",
            DomainError::Resource(ResourceError::Locked { .. }) => "RESOURCE_LOCKED",
            DomainError::Infrastructure(_) => "INFRASTRUCTURE_ERROR",
            DomainError::Generic(_) => "DOMAIN_ERROR",
        }
    }
}

/// Display implementation for ValidationError with Multiple handling

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidJobId { reason } => write!(f, "Invalid job ID: {}", reason),
            ValidationError::InvalidPayload { reason } => write!(f, "Invalid payload: {}", reason),
            ValidationError::MissingField { field } => write!(f, "Missing required field: {}", field),
            ValidationError::InvalidTimestamp { field, reason } => {
                write!(f, "Invalid timestamp: {} - {}", field, reason)
            }
            ValidationError::InvalidWorkerConstraints { reason } => {
                write!(f, "Invalid worker constraints: {}", reason)
            }
            ValidationError::Multiple(errors) => {
                write!(f, "Multiple validation errors: ")?;
                for (i, err) in errors.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", err)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ValidationError {}
