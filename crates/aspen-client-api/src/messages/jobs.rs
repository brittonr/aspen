//! Jobs operation types.
//!
//! Request/response types for job scheduling, management, and worker coordination.

use alloc::string::String;
#[cfg(feature = "auth")]
use alloc::string::ToString;
use alloc::vec::Vec;

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
        capacity_jobs: u32,
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

#[cfg(feature = "auth")]
impl JobsRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth_core::Operation> {
        use aspen_auth_core::Operation;
        match self {
            Self::JobSubmit { job_type, .. } => Some(Operation::JobsWrite {
                resource: format!("type:{job_type}"),
            }),
            Self::JobCancel { job_id, .. } | Self::JobUpdateProgress { job_id, .. } => Some(Operation::JobsWrite {
                resource: format!("job:{job_id}"),
            }),
            Self::WorkerRegister { worker_id, .. }
            | Self::WorkerHeartbeat { worker_id, .. }
            | Self::WorkerDeregister { worker_id } => Some(Operation::JobsWrite {
                resource: format!("worker:{worker_id}"),
            }),
            Self::JobGet { job_id } => Some(Operation::JobsRead {
                resource: format!("job:{job_id}"),
            }),
            Self::JobList { job_type, .. } => Some(Operation::JobsRead {
                resource: match job_type.as_deref() {
                    Some(job_type) => format!("type:{job_type}"),
                    None => "job:".to_string(),
                },
            }),
            Self::JobQueueStats => Some(Operation::JobsRead {
                resource: "queue:stats".to_string(),
            }),
            Self::WorkerStatus => Some(Operation::JobsRead {
                resource: "worker:".to_string(),
            }),
            Self::WorkerPollJobs { worker_id, .. } => Some(Operation::JobsRead {
                resource: format!("worker:{worker_id}:jobs"),
            }),
            Self::WorkerCompleteJob { worker_id, .. } => Some(Operation::JobsWrite {
                resource: format!("worker:{worker_id}:complete"),
            }),
        }
    }
}
