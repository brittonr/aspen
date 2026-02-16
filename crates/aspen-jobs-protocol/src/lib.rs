//! Job execution response types.
//!
//! Response types for high-level job scheduling and worker management operations.

use serde::Deserialize;
use serde::Serialize;

/// Job submit result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmitResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
    /// Job ID assigned to the submitted job.
    pub job_id: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job details for get/list operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDetails {
    /// Job ID.
    pub job_id: String,
    /// Job type.
    pub job_type: String,
    /// Job status.
    pub status: String,
    /// Priority level.
    pub priority: u8,
    /// Progress percentage (0-100).
    pub progress: u8,
    /// Progress message.
    pub progress_message: Option<String>,
    /// Job payload (JSON-encoded string).
    pub payload: String,
    /// Tags associated with the job.
    pub tags: Vec<String>,
    /// Submission time (ISO 8601).
    pub submitted_at: String,
    /// Start time (ISO 8601).
    pub started_at: Option<String>,
    /// Completion time (ISO 8601).
    pub completed_at: Option<String>,
    /// Worker ID processing this job.
    pub worker_id: Option<String>,
    /// Number of retry attempts.
    pub attempts: u32,
    /// Job result (if completed, JSON-encoded string).
    pub result: Option<String>,
    /// Error message (if failed).
    pub error_message: Option<String>,
}

/// Job get result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobGetResultResponse {
    /// Whether the job was found.
    pub was_found: bool,
    /// Job details if found.
    pub job: Option<JobDetails>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job list result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobListResultResponse {
    /// List of jobs matching the filter.
    pub jobs: Vec<JobDetails>,
    /// Total count of matching jobs.
    pub total_count: u32,
    /// Continuation token for pagination.
    pub continuation_token: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job cancel result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCancelResultResponse {
    /// Whether the cancellation succeeded.
    pub is_success: bool,
    /// Previous status of the job.
    pub previous_status: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job update progress result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobUpdateProgressResultResponse {
    /// Whether the update succeeded.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Job queue statistics response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobQueueStatsResultResponse {
    /// Number of pending jobs.
    pub pending_count: u64,
    /// Number of scheduled jobs.
    pub scheduled_count: u64,
    /// Number of running jobs.
    pub running_count: u64,
    /// Number of completed jobs (recent).
    pub completed_count: u64,
    /// Number of failed jobs (recent).
    pub failed_count: u64,
    /// Number of cancelled jobs (recent).
    pub cancelled_count: u64,
    /// Jobs per priority level.
    pub priority_counts: Vec<PriorityCount>,
    /// Jobs per type.
    pub type_counts: Vec<TypeCount>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Priority level job count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityCount {
    /// Priority level (0=Low, 1=Normal, 2=High, 3=Critical).
    pub priority: u8,
    /// Number of jobs at this priority.
    pub count: u64,
}

/// Job type count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCount {
    /// Job type name.
    pub job_type: String,
    /// Number of jobs of this type.
    pub count: u64,
}

/// Worker information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Worker ID.
    pub worker_id: String,
    /// Worker status: idle, busy, offline.
    pub status: String,
    /// Job types this worker can handle.
    pub capabilities: Vec<String>,
    /// Maximum concurrent jobs.
    pub capacity: u32,
    /// Currently active job count.
    pub active_jobs: u32,
    /// Job IDs currently being processed.
    pub active_job_ids: Vec<String>,
    /// Last heartbeat time (ISO 8601).
    pub last_heartbeat: String,
    /// Total jobs processed.
    pub total_processed: u64,
    /// Total jobs failed.
    pub total_failed: u64,
}

/// Worker status result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatusResultResponse {
    /// List of registered workers.
    pub workers: Vec<WorkerInfo>,
    /// Total worker count.
    pub total_workers: u32,
    /// Number of idle workers.
    pub idle_workers: u32,
    /// Number of busy workers.
    pub busy_workers: u32,
    /// Number of offline workers.
    pub offline_workers: u32,
    /// Total capacity across all workers.
    pub total_capacity: u32,
    /// Currently used capacity.
    pub used_capacity: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Worker register result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegisterResultResponse {
    /// Whether registration succeeded.
    pub is_success: bool,
    /// Assigned worker token for authentication.
    pub worker_token: Option<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Worker heartbeat result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeatResultResponse {
    /// Whether heartbeat was accepted.
    pub is_success: bool,
    /// Jobs to dequeue (job IDs).
    pub jobs_to_process: Vec<String>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// Worker deregister result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerDeregisterResultResponse {
    /// Whether deregistration succeeded.
    pub is_success: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

// =============================================================================
// Worker Job Coordination Response Types
// =============================================================================

/// Job information returned by worker polling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerJobInfo {
    /// Job identifier.
    pub job_id: String,
    /// Job type.
    pub job_type: String,
    /// Job specification data (JSON-encoded JobSpec).
    pub job_spec_json: String,
    /// Job priority.
    pub priority: String,
    /// When the job was created (Unix timestamp ms).
    pub created_at_ms: u64,
    /// Visibility timeout for this job (Unix timestamp ms).
    pub visibility_timeout_ms: u64,
    /// Receipt handle for acknowledging job completion (from queue).
    pub receipt_handle: String,
    /// Execution token for job completion (from job manager).
    pub execution_token: String,
}

/// Worker job polling result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerPollJobsResultResponse {
    /// Whether polling was successful.
    pub is_success: bool,
    /// Worker ID that polled for jobs.
    pub worker_id: String,
    /// Jobs assigned to this worker.
    pub jobs: Vec<WorkerJobInfo>,
    /// Error message if polling failed.
    pub error: Option<String>,
}

/// Worker job completion result response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCompleteJobResultResponse {
    /// Whether the job completion was recorded successfully.
    pub is_success: bool,
    /// Worker ID that completed the job.
    pub worker_id: String,
    /// Job ID that was completed.
    pub job_id: String,
    /// Error message if completion recording failed.
    pub error: Option<String>,
}
