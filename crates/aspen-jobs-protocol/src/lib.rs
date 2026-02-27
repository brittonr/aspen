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
    /// Maximum concurrent jobs (capacity in job count).
    pub capacity_jobs: u32,
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
    /// Total capacity across all workers (in job slots).
    pub total_capacity_jobs: u32,
    /// Currently used capacity (in job slots).
    pub used_capacity_jobs: u32,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: postcard roundtrip.
    fn postcard_roundtrip<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(val: &T) {
        let bytes = postcard::to_stdvec(val).expect("postcard serialize");
        let _: T = postcard::from_bytes(&bytes).expect("postcard deserialize");
    }

    /// Helper: JSON roundtrip.
    fn json_roundtrip<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(val: &T) {
        let json = serde_json::to_string(val).expect("json serialize");
        let _: T = serde_json::from_str(&json).expect("json deserialize");
    }

    /// Helper: run both roundtrips.
    fn roundtrip<T: Serialize + for<'de> Deserialize<'de> + std::fmt::Debug>(val: &T) {
        postcard_roundtrip(val);
        json_roundtrip(val);
    }

    fn sample_job_details() -> JobDetails {
        JobDetails {
            job_id: "job-001".into(),
            job_type: "build".into(),
            status: "running".into(),
            priority: 2,
            progress: 50,
            progress_message: Some("compiling...".into()),
            payload: r#"{"target":"x86_64"}"#.into(),
            tags: vec!["ci".into(), "nightly".into()],
            submitted_at: "2025-01-15T10:00:00Z".into(),
            started_at: Some("2025-01-15T10:00:05Z".into()),
            completed_at: None,
            worker_id: Some("worker-3".into()),
            attempts: 1,
            result: None,
            error_message: None,
        }
    }

    // =========================================================================
    // Job lifecycle
    // =========================================================================

    #[test]
    fn job_submit_result_roundtrip() {
        roundtrip(&JobSubmitResultResponse {
            is_success: true,
            job_id: Some("job-001".into()),
            error: None,
        });
        roundtrip(&JobSubmitResultResponse {
            is_success: false,
            job_id: None,
            error: Some("queue full".into()),
        });
    }

    #[test]
    fn job_details_roundtrip() {
        roundtrip(&sample_job_details());
        // Completed job with all optionals populated
        roundtrip(&JobDetails {
            completed_at: Some("2025-01-15T10:05:00Z".into()),
            progress: 100,
            progress_message: None,
            result: Some(r#"{"status":"ok"}"#.into()),
            error_message: None,
            ..sample_job_details()
        });
        // Failed job
        roundtrip(&JobDetails {
            status: "failed".into(),
            progress: 25,
            result: None,
            error_message: Some("OOM killed".into()),
            ..sample_job_details()
        });
    }

    #[test]
    fn job_details_boundary_values() {
        roundtrip(&JobDetails {
            priority: 0,
            progress: 0,
            tags: vec![],
            ..sample_job_details()
        });
        roundtrip(&JobDetails {
            priority: u8::MAX,
            progress: 100,
            ..sample_job_details()
        });
    }

    #[test]
    fn job_get_result_roundtrip() {
        roundtrip(&JobGetResultResponse {
            was_found: true,
            job: Some(sample_job_details()),
            error: None,
        });
        roundtrip(&JobGetResultResponse {
            was_found: false,
            job: None,
            error: None,
        });
    }

    #[test]
    fn job_list_result_roundtrip() {
        roundtrip(&JobListResultResponse {
            jobs: vec![sample_job_details()],
            total_count: 1,
            continuation_token: Some("page2".into()),
            error: None,
        });
        roundtrip(&JobListResultResponse {
            jobs: vec![],
            total_count: 0,
            continuation_token: None,
            error: None,
        });
    }

    #[test]
    fn job_cancel_result_roundtrip() {
        roundtrip(&JobCancelResultResponse {
            is_success: true,
            previous_status: Some("running".into()),
            error: None,
        });
    }

    #[test]
    fn job_update_progress_result_roundtrip() {
        roundtrip(&JobUpdateProgressResultResponse {
            is_success: true,
            error: None,
        });
    }

    #[test]
    fn job_queue_stats_roundtrip() {
        roundtrip(&JobQueueStatsResultResponse {
            pending_count: 10,
            scheduled_count: 5,
            running_count: 3,
            completed_count: 100,
            failed_count: 2,
            cancelled_count: 1,
            priority_counts: vec![PriorityCount { priority: 0, count: 5 }, PriorityCount {
                priority: 2,
                count: 8,
            }],
            type_counts: vec![TypeCount {
                job_type: "build".into(),
                count: 13,
            }],
            error: None,
        });
        // Empty stats
        roundtrip(&JobQueueStatsResultResponse {
            pending_count: 0,
            scheduled_count: 0,
            running_count: 0,
            completed_count: 0,
            failed_count: 0,
            cancelled_count: 0,
            priority_counts: vec![],
            type_counts: vec![],
            error: None,
        });
    }

    // =========================================================================
    // Worker management
    // =========================================================================

    #[test]
    fn worker_status_result_roundtrip() {
        roundtrip(&WorkerStatusResultResponse {
            workers: vec![WorkerInfo {
                worker_id: "w-1".into(),
                status: "busy".into(),
                capabilities: vec!["build".into(), "test".into()],
                capacity_jobs: 4,
                active_jobs: 2,
                active_job_ids: vec!["job-001".into(), "job-002".into()],
                last_heartbeat: "2025-01-15T10:00:00Z".into(),
                total_processed: 150,
                total_failed: 3,
            }],
            total_workers: 1,
            idle_workers: 0,
            busy_workers: 1,
            offline_workers: 0,
            total_capacity_jobs: 4,
            used_capacity_jobs: 2,
            error: None,
        });
    }

    #[test]
    fn worker_register_result_roundtrip() {
        roundtrip(&WorkerRegisterResultResponse {
            is_success: true,
            worker_token: Some("tok-abc".into()),
            error: None,
        });
    }

    #[test]
    fn worker_heartbeat_result_roundtrip() {
        roundtrip(&WorkerHeartbeatResultResponse {
            is_success: true,
            jobs_to_process: vec!["job-003".into()],
            error: None,
        });
        roundtrip(&WorkerHeartbeatResultResponse {
            is_success: true,
            jobs_to_process: vec![],
            error: None,
        });
    }

    #[test]
    fn worker_deregister_result_roundtrip() {
        roundtrip(&WorkerDeregisterResultResponse {
            is_success: true,
            error: None,
        });
    }

    // =========================================================================
    // Worker job coordination
    // =========================================================================

    #[test]
    fn worker_poll_jobs_result_roundtrip() {
        roundtrip(&WorkerPollJobsResultResponse {
            is_success: true,
            worker_id: "w-1".into(),
            jobs: vec![WorkerJobInfo {
                job_id: "job-005".into(),
                job_type: "test".into(),
                job_spec_json: r#"{"command":"cargo nextest run"}"#.into(),
                priority: "high".into(),
                created_at_ms: 1_700_000_000_000,
                visibility_timeout_ms: 1_700_000_060_000,
                receipt_handle: "rh-xyz".into(),
                execution_token: "et-123".into(),
            }],
            error: None,
        });
        // No jobs available
        roundtrip(&WorkerPollJobsResultResponse {
            is_success: true,
            worker_id: "w-1".into(),
            jobs: vec![],
            error: None,
        });
    }

    #[test]
    fn worker_complete_job_result_roundtrip() {
        roundtrip(&WorkerCompleteJobResultResponse {
            is_success: true,
            worker_id: "w-1".into(),
            job_id: "job-005".into(),
            error: None,
        });
        roundtrip(&WorkerCompleteJobResultResponse {
            is_success: false,
            worker_id: "w-1".into(),
            job_id: "job-005".into(),
            error: Some("job already completed".into()),
        });
    }
}
