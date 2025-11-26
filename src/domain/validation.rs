//! Job Validation Module
//!
//! This module contains all business logic for validating jobs.
//! It provides a trait-based system for extensible validation rules.
use crate::domain::types::{Job, JobStatus};
use std::sync::Arc;
/// Result type for validation operations
pub type ValidationResult = Result<(), ValidationError>;
/// Validation errors that can occur during job validation
#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid job ID: {reason}")]
    InvalidJobId { reason: String },
    #[error("Invalid payload: {reason}")]
    InvalidPayload { reason: String },
    #[error("Invalid job status: {status:?}")]
    InvalidStatus { status: JobStatus },
    #[error("Missing required field: {field}")]
    MissingField { field: String },
    #[error("Invalid worker type constraints: {reason}")]
    InvalidWorkerConstraints { reason: String },
    #[error("Validation failed: {reason}")]
    Generic { reason: String },
}
/// Trait for job validators
///
/// Implement this trait to add custom validation logic for jobs.
pub trait JobValidator: Send + Sync {
    /// Validate a job
    ///
    /// Returns `Ok(())` if the job is valid, or a `ValidationError` otherwise.
    fn validate(&self, job: &Job) -> ValidationResult;
    /// Get the name of this validator for logging
    fn name(&self) -> &str;
    /// Priority for this validator (higher = runs first)
    fn priority(&self) -> u32 {
        100
    }
}
/// Composite validator that runs multiple validators in sequence
pub struct CompositeValidator {
    validators: Vec<Arc<dyn JobValidator>>,
}
impl CompositeValidator {
    /// Create a new composite validator
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }
    /// Add a validator to the composite
    pub fn add_validator(mut self, validator: Arc<dyn JobValidator>) -> Self {
        self.validators.push(validator);
        // Sort by priority (highest first)
        self.validators.sort_by_key(|v| std::cmp::Reverse(v.priority()));
        self
    }
    /// Validate a job using all registered validators
    pub fn validate_job(&self, job: &Job) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        for validator in &self.validators {
            if let Err(e) = validator.validate(job) {
                tracing::debug!(
                    validator = validator.name(),
                    error = %e,
                    job_id = %job.id,
                    "Validation failed"
                );
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
impl Default for CompositeValidator {
    fn default() -> Self {
        Self::new()
    }
}
// === Built-in Validators ===
/// Validator that checks job ID format
pub struct JobIdValidator {
    min_length: usize,
    max_length: usize,
}
impl JobIdValidator {
    pub fn new() -> Self {
        Self {
            min_length: 1,
            max_length: 255,
        }
    }
    pub fn with_length_constraints(min_length: usize, max_length: usize) -> Self {
        Self {
            min_length,
            max_length,
        }
    }
}
impl JobValidator for JobIdValidator {
    fn validate(&self, job: &Job) -> ValidationResult {
        let id_len = job.id.len();
        if id_len < self.min_length {
            return Err(ValidationError::InvalidJobId {
                reason: format!("ID too short (minimum {} characters)", self.min_length),
            });
        }
        if id_len > self.max_length {
            return Err(ValidationError::InvalidJobId {
                reason: format!("ID too long (maximum {} characters)", self.max_length),
            });
        }
        // Check for valid characters (alphanumeric, dash, underscore)
        if !job.id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(ValidationError::InvalidJobId {
                reason: "ID contains invalid characters".to_string(),
            });
        }
        Ok(())
    }
    fn name(&self) -> &str {
        "JobIdValidator"
    }
    fn priority(&self) -> u32 {
        200 // Higher priority - check ID first
    }
}
/// Validator that checks job payload structure
pub struct PayloadValidator {
    max_size_bytes: Option<usize>,
}
impl PayloadValidator {
    pub fn new() -> Self {
        Self {
            max_size_bytes: Some(1024 * 1024), // 1MB default
        }
    }
    pub fn with_max_size(max_size_bytes: Option<usize>) -> Self {
        Self { max_size_bytes }
    }
}
impl JobValidator for PayloadValidator {
    fn validate(&self, job: &Job) -> ValidationResult {
        // Check payload size if limit is set
        if let Some(max_size) = self.max_size_bytes {
            let payload_str = job.payload.to_string();
            if payload_str.len() > max_size {
                return Err(ValidationError::InvalidPayload {
                    reason: format!("Payload exceeds maximum size of {} bytes", max_size),
                });
            }
        }
        // Ensure payload is not null (but can be empty object)
        if job.payload.is_null() {
            return Err(ValidationError::InvalidPayload {
                reason: "Payload cannot be null".to_string(),
            });
        }
        Ok(())
    }
    fn name(&self) -> &str {
        "PayloadValidator"
    }

    fn priority(&self) -> u32 {
        150
    }
}

/// Validator for worker type constraints
pub struct WorkerConstraintValidator;
impl JobValidator for WorkerConstraintValidator {
    fn validate(&self, job: &Job) -> ValidationResult {
        // Check for duplicate worker types in constraints
        let mut seen = std::collections::HashSet::new();
        for worker_type in &job.compatible_worker_types {
            if !seen.insert(worker_type) {
                return Err(ValidationError::InvalidWorkerConstraints {
                    reason: format!("Duplicate worker type: {:?}", worker_type),
                });
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "WorkerConstraintValidator"
    }

    fn priority(&self) -> u32 {
        100
    }
}

/// Validator that checks job timestamps
pub struct TimestampValidator;
impl JobValidator for TimestampValidator {
    fn validate(&self, job: &Job) -> ValidationResult {
        // Ensure created_at is set
        if job.created_at <= 0 {
            return Err(ValidationError::MissingField {
                field: "created_at".to_string(),
            });
        }
        // Ensure updated_at >= created_at
        if job.updated_at < job.created_at {
            return Err(ValidationError::Generic {
                reason: "updated_at cannot be before created_at".to_string(),
            });
        }
        // If started_at is set, ensure it's >= created_at
        if let Some(started_at) = job.started_at {
            if started_at < job.created_at {
                return Err(ValidationError::Generic {
                    reason: "started_at cannot be before created_at".to_string(),
                });
            }
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "TimestampValidator"
    }

    fn priority(&self) -> u32 {
        50
    }
}

/// Create a default validator with all built-in validators
pub fn create_default_validator() -> CompositeValidator {
    CompositeValidator::new()
        .add_validator(Arc::new(JobIdValidator::new()))
        .add_validator(Arc::new(PayloadValidator::new()))
        .add_validator(Arc::new(WorkerConstraintValidator))
        .add_validator(Arc::new(TimestampValidator))
}
