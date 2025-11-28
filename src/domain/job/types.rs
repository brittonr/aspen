//! Job domain types
//!
//! Core types for job management in the distributed queue.
//! These types are owned by the domain layer and independent of infrastructure.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use super::super::job_metadata::JobMetadata;
use super::super::job_requirements::JobRequirements;
use crate::domain::worker::WorkerType;

/// Job status in the workflow lifecycle
///
/// This is the domain representation of job status, independent of how
/// it's stored or transmitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Job is available for claiming
    Pending,
    /// Job has been claimed by a worker
    Claimed,
    /// Job is being executed
    InProgress,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "Pending"),
            JobStatus::Claimed => write!(f, "Claimed"),
            JobStatus::InProgress => write!(f, "InProgress"),
            JobStatus::Completed => write!(f, "Completed"),
            JobStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// Job representing work in the distributed queue
///
/// This is the domain representation of a job, containing only business-relevant
/// information. Infrastructure concerns (like worker assignment, node tracking)
/// are kept in the infrastructure layer or accessed via compatibility fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    /// Unique job identifier
    pub id: String,
    /// Current status of the job
    pub status: JobStatus,
    /// Job payload (application-specific data)
    pub payload: JsonValue,
    /// Job requirements and constraints
    pub requirements: JobRequirements,
    /// Timing and audit metadata
    pub metadata: JobMetadata,
    /// Error message if job failed
    pub error_message: Option<String>,
}

impl Default for Job {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: JobStatus::Pending,
            payload: serde_json::json!({}),
            requirements: JobRequirements::default(),
            metadata: JobMetadata::default(),
            error_message: None,
        }
    }
}

impl Job {
    /// Calculate duration of job execution in seconds (delegates to metadata)
    pub fn duration_seconds(&self) -> i64 {
        self.metadata.duration_seconds()
    }

    /// Calculate duration of job execution in milliseconds (delegates to metadata)
    pub fn duration_ms(&self) -> i64 {
        self.metadata.duration_ms()
    }

    /// Calculate time since last update in seconds (delegates to metadata)
    pub fn time_since_update_seconds(&self) -> i64 {
        self.metadata.time_since_update_seconds()
    }

    /// Check if job is in a terminal state (completed or failed)
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, JobStatus::Completed | JobStatus::Failed)
    }

    /// Check if job is claimable (pending status)
    pub fn is_claimable(&self) -> bool {
        matches!(self.status, JobStatus::Pending)
    }

    /// Check if job is actively running
    pub fn is_running(&self) -> bool {
        matches!(self.status, JobStatus::InProgress)
    }

    // === Backward compatibility accessors ===
    // These provide access to fields that used to be at the top level

    /// Get created_at timestamp (backward compatibility)
    pub fn created_at(&self) -> i64 {
        self.metadata.created_at
    }

    /// Get updated_at timestamp (backward compatibility)
    pub fn updated_at(&self) -> i64 {
        self.metadata.updated_at
    }

    /// Get started_at timestamp (backward compatibility)
    pub fn started_at(&self) -> Option<i64> {
        self.metadata.started_at
    }

    /// Get retry_count (backward compatibility)
    pub fn retry_count(&self) -> u32 {
        self.metadata.retry_count
    }

    /// Get compatible_worker_types (backward compatibility)
    pub fn compatible_worker_types(&self) -> &[WorkerType] {
        &self.requirements.compatible_worker_types
    }
}
