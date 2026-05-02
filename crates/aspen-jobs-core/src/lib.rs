//! Reusable job contracts and deterministic helpers.
//!
//! This crate intentionally owns only portable job model and policy logic. It
//! does not own worker pools, storage, schedulers, process execution, VM/Nix
//! executors, transport, handlers, or Aspen node integration.

#![warn(missing_docs)]

use std::collections::HashMap;
use std::time::Duration;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// Errors returned by pure jobs-core helpers.
#[derive(Debug, thiserror::Error)]
pub enum JobsCoreError {
    /// Serialization into a job payload failed.
    #[error("failed to serialize job payload: {source}")]
    SerializePayload {
        /// Source JSON serialization error.
        source: serde_json::Error,
    },
}

/// Unique identifier for a job.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(String);

impl JobId {
    /// Create a new unique job ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create a job ID from a string.
    #[must_use]
    pub fn from_string(id: String) -> Self {
        Self(id)
    }

    /// Parse a job ID from a string.
    pub fn parse(id: &str) -> Result<Self, std::string::ParseError> {
        Ok(Self(id.to_string()))
    }

    /// Borrow the string representation.
    #[must_use]
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
        f.write_str(&self.0)
    }
}

/// Priority level for job execution.
#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize
)]
pub enum Priority {
    /// Lowest priority.
    Low = 0,
    /// Normal priority.
    #[default]
    Normal = 1,
    /// High priority.
    High = 2,
    /// Critical priority.
    Critical = 3,
}

impl Priority {
    /// Queue name for this priority level.
    #[must_use]
    pub fn queue_name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Numeric wire representation used by client RPC job submission.
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Decode the numeric wire representation used by client RPC job submission.
    ///
    /// Unknown values fall back to `Normal`, matching historical server behavior.
    #[must_use]
    pub const fn from_u8_or_normal(value: u8) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::High,
            3 => Self::Critical,
            _ => Self::Normal,
        }
    }

    /// Priorities ordered from highest to lowest.
    #[must_use]
    pub fn all_ordered() -> Vec<Self> {
        vec![Self::Critical, Self::High, Self::Normal, Self::Low]
    }
}

/// Status of a job in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is waiting to be scheduled.
    Pending,
    /// Job is scheduled and waiting in queue.
    Scheduled,
    /// Job is being executed by a worker.
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
    /// Job status is unknown after leader failover or crash.
    Unknown,
}

impl JobStatus {
    /// Whether this status is terminal.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled | Self::DeadLetter)
    }

    /// Whether this status is active.
    #[must_use]
    pub fn is_active(self) -> bool {
        matches!(self, Self::Running | Self::Retrying)
    }

    /// Whether this status needs recovery after failover.
    #[must_use]
    pub fn needs_recovery(self) -> bool {
        matches!(self, Self::Unknown | Self::Running | Self::Retrying)
    }
    /// Stable lowercase wire representation used by client RPC job APIs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Scheduled => "scheduled",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Retrying => "retrying",
            Self::DeadLetter => "dead_letter",
            Self::Unknown => "unknown",
        }
    }

    /// Parse a stable lowercase wire representation used by client RPC job APIs.
    ///
    /// Accepts both `dead_letter` and the older `deadletter` spelling for
    /// compatibility with historical clients.
    #[must_use]
    pub fn from_wire_str(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "scheduled" => Some(Self::Scheduled),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            "retrying" => Some(Self::Retrying),
            "dead_letter" | "deadletter" => Some(Self::DeadLetter),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Whether this status is unknown and needs investigation.
    #[must_use]
    pub fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Retry policy for failed jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryPolicy {
    /// No retries.
    None,
    /// Fixed delay between attempts.
    Fixed {
        /// Maximum attempts including the initial attempt.
        max_attempts: u32,
        /// Delay between attempts.
        delay: Duration,
    },
    /// Exponential backoff.
    Exponential {
        /// Maximum attempts including the initial attempt.
        max_attempts: u32,
        /// Initial delay.
        initial_delay: Duration,
        /// Multiplier for each retry.
        multiplier: f64,
        /// Maximum delay between attempts.
        max_delay: Option<Duration>,
    },
    /// Custom retry delays.
    Custom {
        /// Delay schedule for retry attempts.
        delays: Vec<Duration>,
        /// Maximum attempts. If absent, `delays.len()` is used.
        max_attempts: Option<u32>,
    },
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::exponential(3)
    }
}

impl RetryPolicy {
    /// Create a no-retry policy.
    #[must_use]
    pub fn none() -> Self {
        Self::None
    }

    /// Create a fixed retry policy.
    #[must_use]
    pub fn fixed(max_attempts: u32, delay: Duration) -> Self {
        Self::Fixed { max_attempts, delay }
    }

    /// Create an exponential policy with Aspen defaults.
    #[must_use]
    pub fn exponential(max_attempts: u32) -> Self {
        Self::Exponential {
            max_attempts,
            initial_delay: Duration::from_secs(1),
            multiplier: 2.0,
            max_delay: Some(Duration::from_secs(300)),
        }
    }

    /// Create an exponential backoff policy with custom parameters.
    #[must_use]
    pub fn exponential_custom(
        max_attempts: u32,
        initial_delay: Duration,
        multiplier: f64,
        max_delay: Option<Duration>,
    ) -> Self {
        Self::Exponential {
            max_attempts,
            initial_delay,
            multiplier,
            max_delay,
        }
    }

    /// Create a custom retry policy with specific delays.
    #[must_use]
    pub fn custom(delays: Vec<Duration>) -> Self {
        let max_attempts = delays.len() as u32;
        Self::Custom {
            delays,
            max_attempts: Some(max_attempts),
        }
    }

    /// Return the maximum attempts including the initial attempt.
    #[must_use]
    pub fn max_attempts(&self) -> u32 {
        match self {
            Self::None => 1,
            Self::Fixed { max_attempts, .. } | Self::Exponential { max_attempts, .. } => *max_attempts,
            Self::Custom { max_attempts, delays } => max_attempts.unwrap_or(delays.len() as u32),
        }
    }

    /// Return whether another attempt is allowed after `attempts_completed`.
    #[must_use]
    pub fn allows_retry(&self, attempts_completed: u32) -> bool {
        attempts_completed < self.max_attempts()
    }
}

/// Schedule descriptor for job execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Schedule {
    /// Execute once at a specific time.
    Once(DateTime<Utc>),
    /// Execute on a cron expression.
    Recurring(String),
    /// Execute at fixed intervals.
    Interval {
        /// Duration between executions.
        every: Duration,
        /// Optional start time.
        start_at: Option<DateTime<Utc>>,
    },
    /// Rate-limited execution.
    RateLimit {
        /// Maximum executions per hour.
        max_per_hour: u32,
        /// Current hour's execution count.
        #[serde(skip)]
        current_hour_count: u32,
        /// Current hour timestamp.
        #[serde(skip)]
        current_hour: Option<DateTime<Utc>>,
    },
    /// Business-hours execution.
    BusinessHours {
        /// Days of week to run, Monday = 1 and Sunday = 7.
        days: Vec<u32>,
        /// Start hour, 0-23.
        start_hour: u8,
        /// End hour, 0-23.
        end_hour: u8,
        /// Optional timezone name.
        timezone: Option<String>,
    },
    /// Exponential backoff schedule.
    Exponential {
        /// Base delay between executions.
        base_delay: Duration,
        /// Maximum delay between executions.
        max_delay: Duration,
        /// Current multiplier.
        #[serde(skip)]
        current_multiplier: f64,
    },
}

impl Schedule {
    /// Create a one-time schedule.
    #[must_use]
    pub fn once(time: DateTime<Utc>) -> Self {
        Self::Once(time)
    }

    /// Create a one-time schedule after a delay.
    #[must_use]
    #[allow(unknown_lints)]
    #[allow(
        ambient_clock,
        reason = "relative schedule construction needs an explicit UTC boundary"
    )]
    pub fn after(delay: Duration) -> Self {
        let chrono_delay = chrono::Duration::from_std(delay).unwrap_or(chrono::Duration::MAX);
        Self::Once(Utc::now() + chrono_delay)
    }

    /// Create a recurring schedule descriptor.
    #[must_use]
    pub fn cron(expression: impl Into<String>) -> Self {
        Self::Recurring(expression.into())
    }

    /// Common schedule: every minute.
    #[must_use]
    pub fn every_minute() -> Self {
        Self::cron("* * * * *")
    }

    /// Common schedule: every hour.
    #[must_use]
    pub fn every_hour() -> Self {
        Self::cron("0 * * * *")
    }

    /// Common schedule: daily at midnight.
    #[must_use]
    pub fn daily() -> Self {
        Self::cron("0 0 * * *")
    }

    /// Common schedule: weekly on Sunday at midnight.
    #[must_use]
    pub fn weekly() -> Self {
        Self::cron("0 0 * * SUN")
    }

    /// Common schedule: monthly on the first day at midnight.
    #[must_use]
    pub fn monthly() -> Self {
        Self::cron("0 0 1 * *")
    }

    /// Create an interval schedule descriptor.
    #[must_use]
    pub fn interval(every: Duration) -> Self {
        Self::Interval { every, start_at: None }
    }

    /// Create an interval schedule with a specific start time.
    #[must_use]
    pub fn interval_starting_at(every: Duration, start: DateTime<Utc>) -> Self {
        Self::Interval {
            every,
            start_at: Some(start),
        }
    }

    /// Create a rate-limited schedule.
    #[must_use]
    pub fn rate_limited(max_per_hour: u32) -> Self {
        Self::RateLimit {
            max_per_hour,
            current_hour_count: 0,
            current_hour: None,
        }
    }

    /// Create a business-hours schedule.
    #[must_use]
    pub fn business_hours(days: Vec<u32>, start_hour: u8, end_hour: u8) -> Self {
        Self::BusinessHours {
            days,
            start_hour,
            end_hour,
            timezone: None,
        }
    }

    /// Create an exponential backoff schedule.
    #[must_use]
    pub fn exponential_backoff(base_delay: Duration, max_delay: Duration) -> Self {
        Self::Exponential {
            base_delay,
            max_delay,
            current_multiplier: 1.0,
        }
    }
}

/// State of dependencies for a job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyState {
    /// Dependencies are ready.
    Ready,
    /// The job is waiting on dependencies.
    Waiting(Vec<JobId>),
    /// A dependency failed.
    Failed(JobId),
    /// Dependencies were skipped.
    Skipped,
}

/// Policy for handling dependency failures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyFailurePolicy {
    /// Fail this job when a dependency fails.
    #[default]
    FailJob,
    /// Skip this job when a dependency fails.
    SkipJob,
    /// Continue even if dependencies fail.
    Continue,
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
    /// Whether to save the result after completion.
    pub save_result: bool,
    /// Time-to-live for the record after completion.
    pub ttl_after_completion: Option<Duration>,
}

impl Default for JobConfig {
    fn default() -> Self {
        Self {
            priority: Priority::Normal,
            retry_policy: RetryPolicy::default(),
            timeout: Some(Duration::from_secs(300)),
            dependencies: Vec::new(),
            tags: Vec::new(),
            save_result: true,
            ttl_after_completion: Some(Duration::from_secs(86_400)),
        }
    }
}

/// Payload types for VM-executed jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum JobPayload {
    /// Blob-stored binary (ELF, WASM, or other executable format).
    /// All VM binaries MUST be stored in iroh-blobs first.
    BlobBinary {
        /// BLAKE3 hash of the binary (hex string).
        hash: String,
        /// Size of the binary in bytes (for validation).
        size: u64,
        /// Binary format hint (e.g., "elf", "wasm", "unknown").
        format: String,
    },

    /// Build from a Nix flake.
    NixExpression {
        /// Flake URL (e.g., "github:user/repo#package").
        flake_url: String,
        /// Attribute path within the flake (e.g., "jobs.dataProcessor").
        attribute: String,
    },

    /// Build from an inline Nix derivation.
    NixDerivation {
        /// Nix expression as a string.
        content: String,
    },

    /// WASM Component stored in blob store.
    WasmComponent {
        /// BLAKE3 hash of the .wasm component (hex string).
        hash: String,
        /// Size of the component in bytes (for validation).
        size: u64,
        /// Fuel limit for execution (None = default).
        fuel_limit: Option<u64>,
        /// Memory limit in bytes (None = default).
        memory_limit: Option<u64>,
    },
}

impl JobPayload {
    /// Create a blob-stored binary payload.
    /// The binary must already be uploaded to the blob store.
    #[must_use]
    pub fn blob_binary(hash: impl Into<String>, size: u64, format: impl Into<String>) -> Self {
        Self::BlobBinary {
            hash: hash.into(),
            size,
            format: format.into(),
        }
    }

    /// Create a Nix flake payload.
    #[must_use]
    pub fn nix_flake(flake_url: impl Into<String>, attribute: impl Into<String>) -> Self {
        Self::NixExpression {
            flake_url: flake_url.into(),
            attribute: attribute.into(),
        }
    }

    /// Create an inline Nix derivation payload.
    #[must_use]
    pub fn nix_derivation(content: impl Into<String>) -> Self {
        Self::NixDerivation {
            content: content.into(),
        }
    }

    /// Create a WASM component payload.
    /// The component must already be uploaded to the blob store.
    #[must_use]
    pub fn wasm_component(hash: impl Into<String>, size: u64) -> Self {
        Self::WasmComponent {
            hash: hash.into(),
            size,
            fuel_limit: None,
            memory_limit: None,
        }
    }

    /// Create a WASM component payload with resource limits.
    #[must_use]
    pub fn wasm_component_with_limits(
        hash: impl Into<String>,
        size: u64,
        fuel_limit: Option<u64>,
        memory_limit: Option<u64>,
    ) -> Self {
        Self::WasmComponent {
            hash: hash.into(),
            size,
            fuel_limit,
            memory_limit,
        }
    }
}

/// Specification for creating a new job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSpec {
    /// Type of job used for routing.
    pub job_type: String,
    /// JSON payload.
    pub payload: serde_json::Value,
    /// Job configuration.
    pub config: JobConfig,
    /// Optional schedule descriptor.
    pub schedule: Option<Schedule>,
    /// Optional unique key for deduplication.
    pub idempotency_key: Option<String>,
    /// Metadata for tracing or categorization.
    pub metadata: HashMap<String, String>,
}

impl JobSpec {
    /// Create a new job specification.
    #[must_use]
    pub fn new(job_type: impl Into<String>) -> Self {
        Self {
            job_type: job_type.into(),
            payload: serde_json::Value::Null,
            config: JobConfig::default(),
            schedule: None,
            idempotency_key: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the JSON-serializable payload.
    pub fn payload<T: Serialize>(mut self, payload: T) -> Result<Self, JobsCoreError> {
        self.payload = serde_json::to_value(payload).map_err(|source| JobsCoreError::SerializePayload { source })?;
        Ok(self)
    }

    /// Set priority.
    #[must_use]
    pub fn priority(mut self, priority: Priority) -> Self {
        self.config.priority = priority;
        self
    }

    /// Set the retry policy.
    #[must_use]
    pub fn retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.config.retry_policy = policy;
        self
    }

    /// Set the job timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    /// Add a job dependency.
    #[must_use]
    pub fn depends_on(mut self, job_id: JobId) -> Self {
        self.config.dependencies.push(job_id);
        self
    }

    /// Add a tag to the job.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.config.tags.push(tag.into());
        self
    }

    /// Set the job schedule.
    #[must_use]
    pub fn schedule(mut self, schedule: Schedule) -> Self {
        self.schedule = Some(schedule);
        self
    }

    /// Set an idempotency key for deduplication.
    #[must_use]
    pub fn idempotency_key(mut self, key: impl Into<String>) -> Self {
        self.idempotency_key = Some(key.into());
        self
    }

    /// Set whether the job requires isolation.
    #[must_use]
    pub fn with_isolation(mut self, required: bool) -> Self {
        if required {
            self.config.tags.push("requires_isolation".to_string());
        }
        self
    }

    /// Schedule the job at a specific time.
    #[must_use]
    pub fn schedule_at(mut self, time: DateTime<Utc>) -> Self {
        self.schedule = Some(Schedule::Once(time));
        self
    }

    /// Schedule the job after a delay from the current UTC time.
    #[must_use]
    #[allow(unknown_lints)]
    #[allow(ambient_clock, reason = "job schedule helpers need an explicit UTC boundary")]
    pub fn schedule_after(mut self, delay_duration: Duration) -> Self {
        let delay = chrono::Duration::from_std(delay_duration).unwrap_or(chrono::Duration::MAX);
        self.schedule = Some(Schedule::Once(Utc::now() + delay));
        self
    }

    /// Create a VM execution job for a blob-stored binary.
    #[must_use]
    pub fn with_blob_binary(hash: impl Into<String>, size: u64, format: impl Into<String>) -> Self {
        Self::new("vm_execute")
            .payload(JobPayload::blob_binary(hash, size, format))
            .unwrap_or_else(|_| Self::new("vm_execute"))
            .with_isolation(true)
    }

    /// Create a VM execution job for a Nix flake attribute.
    #[must_use]
    pub fn with_nix_flake(flake_url: impl Into<String>, attribute: impl Into<String>) -> Self {
        Self::new("vm_execute")
            .payload(JobPayload::nix_flake(flake_url, attribute))
            .unwrap_or_else(|_| Self::new("vm_execute"))
            .with_isolation(true)
    }

    /// Create a VM execution job for an inline Nix derivation expression.
    #[must_use]
    pub fn with_nix_expr(nix_code: impl Into<String>) -> Self {
        Self::new("vm_execute")
            .payload(JobPayload::nix_derivation(nix_code))
            .unwrap_or_else(|_| Self::new("vm_execute"))
            .with_isolation(true)
    }

    /// Create a WASM component job using a blob-stored component.
    #[must_use]
    pub fn with_wasm_component(hash: impl Into<String>, size: u64) -> Self {
        Self::new("wasm_component")
            .payload(JobPayload::wasm_component(hash, size))
            .unwrap_or_else(|_| Self::new("wasm_component"))
            .with_isolation(true)
    }
}

/// Output from a successful job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobOutput {
    /// Output data from the job.
    pub data: serde_json::Value,
    /// Additional metadata.
    pub metadata: HashMap<String, String>,
}

impl JobOutput {
    /// Create a simple success output with a message field.
    #[must_use]
    pub fn success(message: impl Into<String>) -> Self {
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

/// Result of job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobResult {
    /// Success with output.
    Success(JobOutput),
    /// Failure details.
    Failure(JobFailure),
    /// Job was cancelled.
    Cancelled,
}

impl JobResult {
    /// Create a success result from JSON-convertible output.
    #[must_use]
    pub fn success(output: impl Into<serde_json::Value>) -> Self {
        Self::Success(JobOutput {
            data: output.into(),
            metadata: HashMap::new(),
        })
    }

    /// Create a retryable failure result.
    #[must_use]
    pub fn failure(reason: impl Into<String>) -> Self {
        Self::Failure(JobFailure {
            reason: reason.into(),
            is_retryable: true,
            error_code: None,
        })
    }

    /// Whether the result is successful.
    #[must_use]
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success(_))
    }
}

/// Deterministic state transition input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobEvent {
    /// Enqueue the job.
    Enqueue,
    /// Start executing the job.
    Start,
    /// Complete the job successfully.
    Complete,
    /// Fail the job but retry if policy permits.
    FailRetryable,
    /// Fail the job permanently.
    FailPermanent,
    /// Cancel the job.
    Cancel,
    /// Recover an active job after failover.
    RecoverActive,
}

/// Compute the next job status without touching storage or runtime services.
#[must_use]
pub fn transition_status(
    current: JobStatus,
    event: JobEvent,
    policy: &RetryPolicy,
    attempts_completed: u32,
) -> JobStatus {
    match event {
        JobEvent::Enqueue if matches!(current, JobStatus::Pending) => JobStatus::Scheduled,
        JobEvent::Start if matches!(current, JobStatus::Scheduled | JobStatus::Retrying) => JobStatus::Running,
        JobEvent::Complete if matches!(current, JobStatus::Running) => JobStatus::Completed,
        JobEvent::FailRetryable if policy.allows_retry(attempts_completed) => JobStatus::Retrying,
        JobEvent::FailRetryable => JobStatus::Failed,
        JobEvent::FailPermanent => JobStatus::Failed,
        JobEvent::Cancel if !current.is_terminal() => JobStatus::Cancelled,
        JobEvent::RecoverActive if current.needs_recovery() => JobStatus::Unknown,
        _ => current,
    }
}

/// Queue statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueueStats {
    /// Jobs by priority.
    pub by_priority: HashMap<Priority, u64>,
    /// Total jobs in queue.
    pub total_queued: u64,
    /// Jobs being processed.
    pub processing: u64,
    /// Queue depth.
    pub depth: u64,
    /// Oldest job age in seconds.
    pub oldest_job_age_sec: Option<u64>,
    /// Average wait time in seconds.
    pub avg_wait_time_sec: Option<u64>,
    /// Dead-letter queue stats.
    pub dlq_stats: DlqStats,
}

/// Dead-letter queue statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DlqStats {
    /// Total jobs in DLQ.
    pub total_count: u64,
    /// Jobs in DLQ by priority.
    pub by_priority: HashMap<Priority, u64>,
    /// Jobs in DLQ by job type.
    pub by_job_type: HashMap<String, u64>,
    /// Oldest DLQ entry age in seconds.
    pub oldest_entry_age_sec: Option<u64>,
    /// Total redriven jobs.
    pub total_redriven: u64,
}

/// Backwards-compatible alias matching the runtime crate's historical name.
pub type DLQStats = DlqStats;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_helpers_match_runtime_contract() {
        assert!(JobStatus::Completed.is_terminal());
        assert!(JobStatus::Running.is_active());
        assert!(JobStatus::Retrying.needs_recovery());
    }

    #[test]
    fn priority_wire_helpers_match_client_contract() {
        assert_eq!(Priority::Low.as_u8(), 0);
        assert_eq!(Priority::Normal.as_u8(), 1);
        assert_eq!(Priority::High.as_u8(), 2);
        assert_eq!(Priority::Critical.as_u8(), 3);
        assert_eq!(Priority::from_u8_or_normal(0), Priority::Low);
        assert_eq!(Priority::from_u8_or_normal(1), Priority::Normal);
        assert_eq!(Priority::from_u8_or_normal(2), Priority::High);
        assert_eq!(Priority::from_u8_or_normal(3), Priority::Critical);
        assert_eq!(Priority::from_u8_or_normal(99), Priority::Normal);
    }

    #[test]
    fn job_status_wire_helpers_match_client_contract() {
        assert_eq!(JobStatus::Pending.as_str(), "pending");
        assert_eq!(JobStatus::Scheduled.as_str(), "scheduled");
        assert_eq!(JobStatus::Running.as_str(), "running");
        assert_eq!(JobStatus::Completed.as_str(), "completed");
        assert_eq!(JobStatus::Failed.as_str(), "failed");
        assert_eq!(JobStatus::Cancelled.as_str(), "cancelled");
        assert_eq!(JobStatus::Retrying.as_str(), "retrying");
        assert_eq!(JobStatus::DeadLetter.as_str(), "dead_letter");
        assert_eq!(JobStatus::Unknown.as_str(), "unknown");

        assert_eq!(JobStatus::from_wire_str("dead_letter"), Some(JobStatus::DeadLetter));
        assert_eq!(JobStatus::from_wire_str("deadletter"), Some(JobStatus::DeadLetter));
        assert_eq!(JobStatus::from_wire_str("unknown"), Some(JobStatus::Unknown));
        assert_eq!(JobStatus::from_wire_str("bogus"), None);
    }

    #[test]
    fn retry_transition_respects_policy() {
        let policy = RetryPolicy::fixed(2, Duration::from_secs(1));
        assert_eq!(transition_status(JobStatus::Running, JobEvent::FailRetryable, &policy, 1), JobStatus::Retrying);
        assert_eq!(transition_status(JobStatus::Running, JobEvent::FailRetryable, &policy, 2), JobStatus::Failed);
    }

    #[test]
    fn job_spec_roundtrips_json() {
        let dependency = JobId::from_string("parent".to_string());
        let spec = JobSpec::new("build").priority(Priority::High).depends_on(dependency);
        let encoded = serde_json::to_string(&spec).expect("serialize job spec");
        let decoded: JobSpec = serde_json::from_str(&encoded).expect("deserialize job spec");
        assert_eq!(decoded.job_type, "build");
        assert_eq!(decoded.config.priority, Priority::High);
        assert_eq!(decoded.config.dependencies[0].as_str(), "parent");
    }

    #[test]
    fn job_payload_helpers_match_runtime_schema() {
        let blob = JobPayload::blob_binary("abc123", 42, "elf");
        let encoded = serde_json::to_value(&blob).expect("serialize blob payload");
        assert_eq!(encoded["type"], "BlobBinary");
        assert_eq!(encoded["hash"], "abc123");
        assert_eq!(encoded["size"], 42);
        assert_eq!(encoded["format"], "elf");

        let wasm = JobPayload::wasm_component_with_limits("def456", 128, Some(10_000), Some(64 * 1024));
        let encoded = serde_json::to_value(&wasm).expect("serialize wasm payload");
        assert_eq!(encoded["type"], "WasmComponent");
        assert_eq!(encoded["hash"], "def456");
        assert_eq!(encoded["fuel_limit"], 10_000);
        assert_eq!(encoded["memory_limit"], 64 * 1024);
    }

    #[test]
    fn job_spec_vm_builders_emit_typed_payloads() {
        let blob_spec = JobSpec::with_blob_binary("abc123", 42, "elf");
        let blob: JobPayload = serde_json::from_value(blob_spec.payload).expect("deserialize blob payload");
        match blob {
            JobPayload::BlobBinary { hash, size, format } => {
                assert_eq!(hash, "abc123");
                assert_eq!(size, 42);
                assert_eq!(format, "elf");
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        let nix_spec = JobSpec::with_nix_flake("github:example/repo", "packages.x86_64-linux.default");
        let nix: JobPayload = serde_json::from_value(nix_spec.payload).expect("deserialize nix payload");
        match nix {
            JobPayload::NixExpression { flake_url, attribute } => {
                assert_eq!(flake_url, "github:example/repo");
                assert_eq!(attribute, "packages.x86_64-linux.default");
            }
            other => panic!("unexpected payload: {other:?}"),
        }

        let wasm_spec = JobSpec::with_wasm_component("def456", 128);
        let wasm: JobPayload = serde_json::from_value(wasm_spec.payload).expect("deserialize wasm payload");
        match wasm {
            JobPayload::WasmComponent {
                hash,
                size,
                fuel_limit,
                memory_limit,
            } => {
                assert_eq!(hash, "def456");
                assert_eq!(size, 128);
                assert_eq!(fuel_limit, None);
                assert_eq!(memory_limit, None);
            }
            other => panic!("unexpected payload: {other:?}"),
        }
    }

    #[test]
    fn job_spec_builder_helpers_match_runtime_contract() {
        let dependency = JobId::from_string("parent".to_string());
        let schedule_time = Utc::now();
        let spec = JobSpec::new("build")
            .payload(serde_json::json!({ "step": "compile" }))
            .expect("payload serializes")
            .priority(Priority::Critical)
            .retry_policy(RetryPolicy::fixed(3, Duration::from_secs(2)))
            .timeout(Duration::from_secs(60))
            .depends_on(dependency)
            .tag("ci")
            .schedule_at(schedule_time)
            .idempotency_key("build-main")
            .with_isolation(true);

        assert_eq!(spec.config.priority, Priority::Critical);
        assert_eq!(spec.config.timeout, Some(Duration::from_secs(60)));
        assert_eq!(spec.config.dependencies[0].as_str(), "parent");
        assert_eq!(spec.config.tags, vec!["ci", "requires_isolation"]);
        assert_eq!(spec.schedule, Some(Schedule::Once(schedule_time)));
        assert_eq!(spec.idempotency_key.as_deref(), Some("build-main"));
    }
}
