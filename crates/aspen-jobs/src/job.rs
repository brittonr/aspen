//! Core job types and structures.

use std::time::Duration;

use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::dependency_tracker::DependencyFailurePolicy;
use crate::dependency_tracker::DependencyState;
use crate::types::RetryPolicy;
use crate::types::ScheduleExt;

#[allow(unknown_lints)]
#[allow(ambient_clock, reason = "job timestamps need an explicit UTC boundary helper")]
fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

fn chrono_delay_from_duration(delay_duration: Duration) -> chrono::Duration {
    chrono::Duration::from_std(delay_duration).unwrap_or(chrono::Duration::MAX)
}

fn u32_to_usize(value: u32) -> Option<usize> {
    usize::try_from(value).ok()
}

pub use aspen_jobs_core::JobConfig;
pub use aspen_jobs_core::JobFailure;
pub use aspen_jobs_core::JobId;
pub use aspen_jobs_core::JobOutput;
pub use aspen_jobs_core::JobResult;
pub use aspen_jobs_core::JobSpec;
pub use aspen_jobs_core::JobStatus;

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
    pub dlq_metadata: Option<DlqMetadata>,
    /// Dependency state
    pub dependency_state: DependencyState,
    /// Jobs currently blocking this one
    pub blocked_by: Vec<JobId>,
    /// Jobs waiting on this one
    pub blocking: Vec<JobId>,
    /// Dependency failure policy
    pub dependency_failure_policy: DependencyFailurePolicy,
    /// Execution token for lease-based ownership.
    ///
    /// A unique token generated each time mark_started is called. This token
    /// must be provided to ack_job/nack_job to prevent race conditions where:
    /// 1. Worker A starts job, gets token T1
    /// 2. Visibility timeout expires, job re-enqueued
    /// 3. Worker B dequeues but fails mark_started (job already Running)
    /// 4. Worker A completes with T1 - succeeds
    ///
    /// Or in crash recovery:
    /// 1. Worker A starts job, gets token T1, crashes
    /// 2. Visibility timeout expires, job re-enqueued
    /// 3. Worker B successfully restarts job, gets NEW token T2
    /// 4. Worker A recovers, tries to complete with T1 - rejected (stale token)
    #[serde(default)]
    pub execution_token: Option<String>,
}

/// Dead Letter Queue metadata for a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlqMetadata {
    /// Reason the job was moved to DLQ.
    pub reason: DlqReason,
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
pub enum DlqReason {
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
        let now = utc_now();
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
            dependency_failure_policy: DependencyFailurePolicy::FailCascade,
            execution_token: None,
        }
    }

    /// Check if the job can be executed now.
    pub fn can_execute_now(&self) -> bool {
        // Terminal and Running jobs cannot be executed
        if self.status.is_terminal() || self.status == JobStatus::Running {
            return false;
        }

        // Retrying jobs can only proceed if their retry time has passed
        if self.status == JobStatus::Retrying {
            return match self.next_retry_at {
                Some(next_retry_at) => next_retry_at <= utc_now(),
                None => true,
            };
        }

        // Check dependency state
        if !self.dependency_state.is_ready() {
            return false;
        }

        // Check if scheduled time has passed
        if let Some(scheduled_at) = self.scheduled_at {
            if scheduled_at > utc_now() {
                return false;
            }
        }

        // Check if retry time has passed
        if let Some(next_retry_at) = self.next_retry_at {
            if next_retry_at > utc_now() {
                return false;
            }
        }

        true
    }

    /// Update progress for a running job.
    pub fn update_progress(&mut self, progress: u8, message: Option<String>) {
        self.progress = Some(progress.min(100));
        self.progress_message = message;
        self.updated_at = utc_now();
        self.version += 1;
    }

    /// Mark job as started and generate an execution token.
    ///
    /// Returns the execution token that must be provided to ack_job/nack_job
    /// to prove ownership of this execution attempt.
    pub fn mark_started(&mut self, worker_id: String) -> String {
        let token = Uuid::new_v4().to_string();
        self.status = JobStatus::Running;
        self.worker_id = Some(worker_id);
        self.started_at = Some(utc_now());
        self.attempts = self.attempts.saturating_add(1);
        self.updated_at = utc_now();
        self.version += 1;
        self.execution_token = Some(token.clone());
        token
    }

    /// Mark job as completed.
    pub fn mark_completed(&mut self, result: JobResult) {
        self.status = if result.is_success() {
            JobStatus::Completed
        } else {
            JobStatus::Failed
        };
        self.result = Some(result);
        self.completed_at = Some(utc_now());
        self.updated_at = utc_now();
        self.worker_id = None;
        self.version += 1;
    }

    /// Mark job for retry.
    pub fn mark_retry(&mut self, next_retry_at: DateTime<Utc>, error: String) {
        self.status = JobStatus::Retrying;
        self.next_retry_at = Some(next_retry_at);
        self.last_error = Some(error);
        self.updated_at = utc_now();
        self.worker_id = None;
        self.version += 1;
    }

    /// Mark job as cancelled.
    pub fn mark_cancelled(&mut self) {
        self.status = JobStatus::Cancelled;
        self.result = Some(JobResult::Cancelled);
        self.completed_at = Some(utc_now());
        self.updated_at = utc_now();
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

        let retry_delay = match &self.spec.config.retry_policy {
            RetryPolicy::None => return None,
            RetryPolicy::Fixed { delay, .. } => *delay,
            RetryPolicy::Exponential {
                initial_delay,
                multiplier,
                max_delay,
                ..
            } => {
                let mut retry_delay = *initial_delay;
                for _ in 1..self.attempts {
                    retry_delay = Duration::from_secs_f64(retry_delay.as_secs_f64() * multiplier);
                    if let Some(max_delay_duration) = max_delay {
                        retry_delay = retry_delay.min(*max_delay_duration);
                    }
                }
                retry_delay
            }
            RetryPolicy::Custom { delays, .. } => {
                let retry_index = self.attempts.saturating_sub(1);
                let retry_index = u32_to_usize(retry_index)?;
                delays.get(retry_index).copied().unwrap_or(Duration::from_secs(60))
            }
        };

        Some(utc_now() + chrono_delay_from_duration(retry_delay))
    }

    /// Mark job as moved to Dead Letter Queue.
    pub fn mark_dlq(&mut self, reason: DlqReason, error: String) {
        self.status = JobStatus::DeadLetter;
        self.dlq_metadata = Some(DlqMetadata {
            reason,
            entered_at: utc_now(),
            redriven_at: None,
            original_job_id: None,
            redrive_count: 0,
            final_error: error.clone(),
        });
        self.last_error = Some(error);
        self.updated_at = utc_now();
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
            dlq_meta.redriven_at = Some(utc_now());
            dlq_meta.redrive_count = dlq_meta.redrive_count.saturating_add(1);
        }
        self.status = JobStatus::Pending;
        self.attempts = 0; // Reset attempts for redrive
        self.last_error = None;
        self.next_retry_at = None;
        self.updated_at = utc_now();
        self.version += 1;
    }
}
