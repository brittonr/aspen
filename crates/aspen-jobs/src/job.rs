//! Core job types and structures.

use std::collections::HashMap;
use std::time::Duration;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::dependency_tracker::DependencyFailurePolicy;
use crate::dependency_tracker::DependencyState;
use crate::error::Result;
use crate::types::Priority;
use crate::types::RetryPolicy;
use crate::types::Schedule;

/// Unique identifier for a job.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(String);

impl JobId {
    /// Create a new unique job ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create a job ID from a string.
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Parse a job ID from a string.
    pub fn parse(id: &str) -> Result<Self, std::string::ParseError> {
        Ok(Self(id.to_string()))
    }

    /// Get the string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a job in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is waiting to be scheduled.
    Pending,
    /// Job is scheduled and waiting in queue.
    Scheduled,
    /// Job is currently being executed by a worker.
    Running,
    /// Job completed successfully.
    Completed,
    /// Job failed after all retry attempts.
    Failed,
    /// Job was cancelled.
    Cancelled,
    /// Job is being retried.
    Retrying,
    /// Job is in the dead letter queue.
    DeadLetter,
}

impl JobStatus {
    /// Check if the job is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled | Self::DeadLetter)
    }

    /// Check if the job is active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Running | Self::Retrying)
    }
}

/// Result of a job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResult {
    /// Job completed successfully.
    Success(JobOutput),
    /// Job failed with an error.
    Failure(JobFailure),
    /// Job was cancelled.
    Cancelled,
}

impl JobResult {
    /// Create a success result.
    pub fn success<T: Into<serde_json::Value>>(output: T) -> Self {
        Self::Success(JobOutput {
            data: output.into(),
            metadata: HashMap::new(),
        })
    }

    /// Create a failure result.
    pub fn failure<S: Into<String>>(reason: S) -> Self {
        Self::Failure(JobFailure {
            reason: reason.into(),
            is_retryable: true,
            error_code: None,
        })
    }

    /// Check if the result is successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
}

/// Output from a successful job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOutput {
    /// Output data from the job.
    pub data: serde_json::Value,
    /// Additional metadata about the execution.
    pub metadata: HashMap<String, String>,
}

impl JobOutput {
    /// Create a simple success output.
    pub fn success<S: Into<String>>(message: S) -> Self {
        Self {
            data: serde_json::json!({ "message": message.into() }),
            metadata: HashMap::new(),
        }
    }
}

/// Details about a job failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobFailure {
    /// Reason for the failure.
    pub reason: String,
    /// Whether the job can be retried.
    pub is_retryable: bool,
    /// Optional error code.
    pub error_code: Option<String>,
}

/// Configuration for a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobConfig {
    /// Priority of the job.
    pub priority: Priority,
    /// Retry policy for failures.
    pub retry_policy: RetryPolicy,
    /// Maximum execution time.
    pub timeout: Option<Duration>,
    /// Dependencies on other jobs.
    pub dependencies: Vec<JobId>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Whether to save result after completion.
    pub save_result: bool,
    /// Time-to-live for the job record after completion.
    pub ttl_after_completion: Option<Duration>,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            priority: Priority::Normal,
            retry_policy: RetryPolicy::default(),
            timeout: Some(Duration::from_secs(300)), // 5 minutes default
            dependencies: Vec::new(),
            tags: Vec::new(),
            save_result: true,
            ttl_after_completion: Some(Duration::from_secs(86400)), // 24 hours
        }
    }
}

/// Specification for creating a new job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Type of job (used to route to correct worker).
    pub job_type: String,
    /// Payload for the job.
    pub payload: serde_json::Value,
    /// Job configuration.
    pub config: JobConfig,
    /// Optional schedule for the job.
    pub schedule: Option<Schedule>,
    /// Optional unique key for deduplication.
    pub idempotency_key: Option<String>,
    /// Metadata for the job (tracing, etc.)
    pub metadata: HashMap<String, String>,
}

impl JobSpec {
    /// Create a new job specification.
    pub fn new<S: Into<String>>(job_type: S) -> Self {
        Self {
            job_type: job_type.into(),
            payload: serde_json::Value::Null,
            config: JobConfig::default(),
            schedule: None,
            idempotency_key: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the job payload.
    pub fn payload<T: Serialize>(mut self, payload: T) -> Result<Self> {
        self.payload =
            serde_json::to_value(payload).map_err(|e| crate::error::JobError::SerializationError { source: e })?;
        Ok(self)
    }

    /// Set the job priority.
    pub fn priority(mut self, priority: Priority) -> Self {
        self.config.priority = priority;
        self
    }

    /// Set the retry policy.
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.config.retry_policy = policy;
        self
    }

    /// Set the job timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Add a job dependency.
    pub fn depends_on(mut self, job_id: JobId) -> Self {
        self.config.dependencies.push(job_id);
        self
    }

    /// Add a tag to the job.
    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.config.tags.push(tag.into());
        self
    }

    /// Set the job schedule.
    pub fn schedule(mut self, schedule: Schedule) -> Self {
        self.schedule = Some(schedule);
        self
    }

    /// Set an idempotency key for deduplication.
    pub fn idempotency_key<S: Into<String>>(mut self, key: S) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Set whether the job requires isolation (VM execution).
    pub fn with_isolation(mut self, required: bool) -> Self {
        if required {
            self.config.tags.push("requires_isolation".to_string());
        }
        self
    }

    /// Helper method to create a job with a blob-stored binary.
    /// The binary MUST be uploaded to the blob store first.
    #[cfg(feature = "vm-executor")]
    pub fn with_blob_binary(hash: impl Into<String>, size: u64, format: impl Into<String>) -> Self {
        use crate::vm_executor::JobPayload;
        let payload = JobPayload::blob_binary(hash, size, format);
        Self::new("vm_execute")
            .payload(payload)
            .unwrap_or_else(|_| Self::new("vm_execute"))
            .with_isolation(true)
    }

    /// Helper method to create a job with a Nix flake.
    #[cfg(feature = "vm-executor")]
    pub fn with_nix_flake(flake_url: &str, attribute: &str) -> Self {
        use crate::vm_executor::JobPayload;
        let payload = JobPayload::nix_flake(flake_url, attribute);
        Self::new("vm_execute")
            .payload(payload)
            .unwrap_or_else(|_| Self::new("vm_execute"))
            .with_isolation(true)
    }

    /// Helper method to create a job with an inline Nix expression.
    #[cfg(feature = "vm-executor")]
    pub fn with_nix_expr(nix_code: &str) -> Self {
        use crate::vm_executor::JobPayload;
        let payload = JobPayload::nix_derivation(nix_code);
        Self::new("vm_execute")
            .payload(payload)
            .unwrap_or_else(|_| Self::new("vm_execute"))
            .with_isolation(true)
    }
}

impl JobSpec {
    /// Schedule the job at a specific time.
    pub fn schedule_at(mut self, time: DateTime<Utc>) -> Self {
        self.schedule = Some(Schedule::Once(time));
        self
    }

    /// Schedule the job after a delay.
    pub fn schedule_after(mut self, delay: Duration) -> Self {
        let time = Utc::now() + chrono::Duration::from_std(delay).unwrap();
        self.schedule = Some(Schedule::Once(time));
        self
    }
}

/// A job in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job ID.
    pub id: JobId,
    /// Job specification.
    pub spec: JobSpec,
    /// Current status.
    pub status: JobStatus,
    /// Number of execution attempts.
    pub attempts: u32,
    /// Last error if any.
    pub last_error: Option<String>,
    /// Job result if completed.
    pub result: Option<JobResult>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Scheduled execution time.
    pub scheduled_at: Option<DateTime<Utc>>,
    /// Started execution time.
    pub started_at: Option<DateTime<Utc>>,
    /// Completion time.
    pub completed_at: Option<DateTime<Utc>>,
    /// Worker ID processing this job.
    pub worker_id: Option<String>,
    /// Next retry time if applicable.
    pub next_retry_at: Option<DateTime<Utc>>,
    /// Progress percentage (0-100).
    pub progress: Option<u8>,
    /// Progress message.
    pub progress_message: Option<String>,
    /// Version number for optimistic concurrency control.
    pub version: u64,
    /// DLQ metadata
    pub dlq_metadata: Option<DLQMetadata>,
    /// Dependency state
    pub dependency_state: DependencyState,
    /// Jobs currently blocking this one
    pub blocked_by: Vec<JobId>,
    /// Jobs waiting on this one
    pub blocking: Vec<JobId>,
    /// Dependency failure policy
    pub dependency_failure_policy: DependencyFailurePolicy,
}

/// Dead Letter Queue metadata for a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DLQMetadata {
    /// Reason the job was moved to DLQ.
    pub reason: DLQReason,
    /// Time when job entered DLQ.
    pub entered_at: DateTime<Utc>,
    /// Time when job was redriven from DLQ (if applicable).
    pub redriven_at: Option<DateTime<Utc>>,
    /// Original job ID if this is a redriven job.
    pub original_job_id: Option<JobId>,
    /// Number of times this job has been redriven.
    pub redrive_count: u32,
    /// Final error that caused DLQ entry.
    pub final_error: String,
}

/// Reason for moving a job to the Dead Letter Queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DLQReason {
    /// Exceeded maximum retry attempts.
    MaxRetriesExceeded,
    /// Explicitly moved to DLQ by worker or system.
    ExplicitlyRejected,
    /// Job expired before completion.
    Expired,
    /// Processing timeout exceeded.
    ProcessingTimeout,
    /// Unrecoverable error occurred.
    UnrecoverableError,
}

impl Job {
    /// Create a new job from a specification.
    pub fn from_spec(spec: JobSpec) -> Self {
        let now = Utc::now();
        let scheduled_at = match &spec.schedule {
            Some(schedule) => schedule.next_execution(),
            None => None,
        };

        // Clone dependencies before moving spec
        let dependencies = spec.config.dependencies.clone();
        let dependency_state = if dependencies.is_empty() {
            DependencyState::Ready
        } else {
            DependencyState::Waiting(dependencies.clone())
        };

        Self {
            id: JobId::new(),
            spec,
            status: if scheduled_at.is_some() {
                JobStatus::Scheduled
            } else {
                JobStatus::Pending
            },
            attempts: 0,
            last_error: None,
            result: None,
            created_at: now,
            updated_at: now,
            scheduled_at,
            started_at: None,
            completed_at: None,
            worker_id: None,
            next_retry_at: None,
            progress: None,
            progress_message: None,
            version: 0,
            dlq_metadata: None,
            dependency_state,
            blocked_by: dependencies,
            blocking: Vec::new(),
            dependency_failure_policy: DependencyFailurePolicy::default(),
        }
    }

    /// Check if the job can be executed now.
    pub fn can_execute_now(&self) -> bool {
        if self.status.is_terminal() || self.status.is_active() {
            return false;
        }

        // Check dependency state
        if !self.dependency_state.is_ready() {
            return false;
        }

        // Check if scheduled time has passed
        if let Some(scheduled_at) = self.scheduled_at {
            if scheduled_at > Utc::now() {
                return false;
            }
        }

        // Check if retry time has passed
        if let Some(next_retry_at) = self.next_retry_at {
            if next_retry_at > Utc::now() {
                return false;
            }
        }

        true
    }

    /// Update progress for a running job.
    pub fn update_progress(&mut self, progress: u8, message: Option<String>) {
        self.progress = Some(progress.min(100));
        self.progress_message = message;
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Mark job as started.
    pub fn mark_started(&mut self, worker_id: String) {
        self.status = JobStatus::Running;
        self.worker_id = Some(worker_id);
        self.started_at = Some(Utc::now());
        self.attempts += 1;
        self.updated_at = Utc::now();
        self.version += 1;
    }

    /// Mark job as completed.
    pub fn mark_completed(&mut self, result: JobResult) {
        self.status = if result.is_success() {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };
        self.result = Some(result);
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.worker_id = None;
        self.version += 1;
    }

    /// Mark job for retry.
    pub fn mark_retry(&mut self, next_retry_at: DateTime<Utc>, error: String) {
        self.status = JobStatus::Retrying;
        self.next_retry_at = Some(next_retry_at);
        self.last_error = Some(error);
        self.updated_at = Utc::now();
        self.worker_id = None;
        self.version += 1;
    }

    /// Mark job as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = JobStatus::Cancelled;
        self.result = Some(JobResult::Cancelled);
        self.completed_at = Some(Utc::now());
        self.updated_at = Utc::now();
        self.worker_id = None;
        self.version += 1;
    }

    /// Check if the job has exceeded its retry limit.
    pub fn exceeded_retry_limit(&self) -> bool {
        match &self.spec.config.retry_policy {
            RetryPolicy::None => self.attempts > 1,
            RetryPolicy::Fixed { max_attempts, .. } => self.attempts >= *max_attempts,
            RetryPolicy::Exponential { max_attempts, .. } => self.attempts >= *max_attempts,
            RetryPolicy::Custom { max_attempts, .. } => max_attempts.is_some_and(|max| self.attempts >= max),
        }
    }

    /// Calculate the next retry time based on the retry policy.
    pub fn calculate_next_retry(&self) -> Option<DateTime<Utc>> {
        if self.exceeded_retry_limit() {
            return None;
        }

        let delay = match &self.spec.config.retry_policy {
            RetryPolicy::None => return None,
            RetryPolicy::Fixed { delay, .. } => *delay,
            RetryPolicy::Exponential {
                initial_delay,
                multiplier,
                max_delay,
                ..
            } => {
                let mut delay = *initial_delay;
                for _ in 1..self.attempts {
                    delay = Duration::from_secs_f64(delay.as_secs_f64() * multiplier);
                    if let Some(max) = max_delay {
                        delay = delay.min(*max);
                    }
                }
                delay
            }
            RetryPolicy::Custom { delays, .. } => {
                let index = (self.attempts - 1) as usize;
                delays.get(index).copied().unwrap_or(Duration::from_secs(60))
            }
        };

        Some(Utc::now() + chrono::Duration::from_std(delay).unwrap())
    }

    /// Mark job as moved to Dead Letter Queue.
    pub fn mark_dlq(&mut self, reason: DLQReason, error: String) {
        self.status = JobStatus::DeadLetter;
        self.dlq_metadata = Some(DLQMetadata {
            reason,
            entered_at: Utc::now(),
            redriven_at: None,
            original_job_id: None,
            redrive_count: 0,
            final_error: error.clone(),
        });
        self.last_error = Some(error);
        self.updated_at = Utc::now();
        self.worker_id = None;
        self.version += 1;
    }

    /// Check if job is in Dead Letter Queue.
    pub fn is_in_dlq(&self) -> bool {
        self.status == JobStatus::DeadLetter
    }

    /// Prepare job for redrive from DLQ.
    pub fn prepare_for_redrive(&mut self) {
        if let Some(ref mut dlq_meta) = self.dlq_metadata {
            dlq_meta.redriven_at = Some(Utc::now());
            dlq_meta.redrive_count += 1;
        }
        self.status = JobStatus::Pending;
        self.attempts = 0; // Reset attempts for redrive
        self.last_error = None;
        self.next_retry_at = None;
        self.updated_at = Utc::now();
        self.version += 1;
    }
}
