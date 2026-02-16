//! Jobs operation types.
//!
//! Request/response types for job scheduling, management, and worker coordination.

pub use aspen_jobs_protocol::*;
use serde::Deserialize;
use serde::Serialize;

/// Jobs domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobsRequest {
    /// Submit a new job to the job queue system.
    JobSubmit {
        job_type: String,
        payload: String,
        priority: Option<u8>,
        timeout_ms: Option<u64>,
        max_retries: Option<u32>,
        retry_delay_ms: Option<u64>,
        schedule: Option<String>,
        tags: Vec<String>,
    },
    /// Get job status and details.
    JobGet { job_id: String },
    /// List jobs with optional filtering.
    JobList {
        status: Option<String>,
        job_type: Option<String>,
        tags: Vec<String>,
        limit: Option<u32>,
        continuation_token: Option<String>,
    },
    /// Cancel a job.
    JobCancel { job_id: String, reason: Option<String> },
    /// Update job progress (for workers).
    JobUpdateProgress {
        job_id: String,
        progress: u8,
        message: Option<String>,
    },
    /// Get job queue statistics.
    JobQueueStats,
    /// Get worker pool status.
    WorkerStatus,
    /// Register a worker.
    WorkerRegister {
        worker_id: String,
        capabilities: Vec<String>,
        capacity: u32,
    },
    /// Worker heartbeat.
    WorkerHeartbeat {
        worker_id: String,
        active_jobs: Vec<String>,
    },
    /// Deregister a worker.
    WorkerDeregister { worker_id: String },
    /// Poll for available jobs.
    WorkerPollJobs {
        worker_id: String,
        job_types: Vec<String>,
        max_jobs: usize,
        visibility_timeout_secs: u64,
    },
    /// Complete a job and report the result.
    WorkerCompleteJob {
        worker_id: String,
        job_id: String,
        receipt_handle: String,
        execution_token: String,
        is_success: bool,
        error_message: Option<String>,
        output_data: Option<Vec<u8>>,
        processing_time_ms: u64,
    },
}

impl JobsRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::JobSubmit { .. }
            | Self::JobCancel { .. }
            | Self::JobUpdateProgress { .. }
            | Self::WorkerRegister { .. }
            | Self::WorkerHeartbeat { .. }
            | Self::WorkerDeregister { .. } => Some(Operation::Write {
                key: "_jobs:".to_string(),
                value: vec![],
            }),
            Self::JobGet { .. } | Self::JobList { .. } | Self::JobQueueStats | Self::WorkerStatus => {
                Some(Operation::Read {
                    key: "_jobs:".to_string(),
                })
            }
            Self::WorkerPollJobs { worker_id, .. } => Some(Operation::Read {
                key: format!("__worker:{worker_id}:jobs"),
            }),
            Self::WorkerCompleteJob { worker_id, .. } => Some(Operation::Write {
                key: format!("__worker:{worker_id}:complete"),
                value: vec![],
            }),
        }
    }
}
