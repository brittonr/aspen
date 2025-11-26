//! Domain types for job management
//!
//! These types are owned by the domain layer and independent of infrastructure.
//! They represent the business concepts without coupling to storage or transport.
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use super::job_metadata::JobMetadata;
use super::job_requirements::JobRequirements;
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

    // === Backward compatibility fields (infrastructure concerns) ===
    // These will eventually be moved to JobAssignment in infrastructure layer
    /// Worker node that claimed this job (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    /// Worker ID assigned to execute this job (from worker registry)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_worker_id: Option<String>,
    /// Worker node that completed this job (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_by: Option<String>,
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
            claimed_by: None,
            assigned_worker_id: None,
            completed_by: None,
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
/// Aggregate statistics for the job queue
///
/// Domain representation of queue health and activity metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Total number of jobs in the queue
    pub total: usize,
    /// Number of jobs pending (available for claim)
    pub pending: usize,
    /// Number of jobs claimed by workers
    pub claimed: usize,
    /// Number of jobs in progress
    pub in_progress: usize,
    /// Number of completed jobs
    pub completed: usize,
    /// Number of failed jobs
    pub failed: usize,
}
impl QueueStats {
    /// Create empty statistics
    pub fn empty() -> Self {
        Self {
            total: 0,
            pending: 0,
            claimed: 0,
            in_progress: 0,
            completed: 0,
            failed: 0,
        }
    }
    /// Calculate from a collection of jobs
    pub fn from_jobs(jobs: &[Job]) -> Self {
        let mut stats = Self::empty();
        stats.total = jobs.len();
        for job in jobs {
            match job.status {
                JobStatus::Pending => stats.pending += 1,
                JobStatus::Claimed => stats.claimed += 1,
                JobStatus::InProgress => stats.in_progress += 1,
                JobStatus::Completed => stats.completed += 1,
                JobStatus::Failed => stats.failed += 1,
            }
        }
        stats
    }
}
/// Database cluster health status
///
/// Domain representation of distributed database health, independent of
/// infrastructure implementation (hiqlite, postgres, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthStatus {
    /// Whether the database cluster is operating normally
    pub is_healthy: bool,
    /// Number of nodes in the database cluster
    pub node_count: usize,
    /// Whether the cluster has an elected leader
    pub has_leader: bool,
}

// =============================================================================
// WORKER DOMAIN TYPES
// =============================================================================
/// Type of worker backend for job execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkerType {
    /// Firecracker microVM-based worker
    Firecracker,
    /// WASM workflow-based worker (Flawless)
    Wasm,
}

impl std::fmt::Display for WorkerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerType::Firecracker => write!(f, "firecracker"),
            WorkerType::Wasm => write!(f, "wasm"),
        }
    }
}

impl std::str::FromStr for WorkerType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "firecracker" => Ok(WorkerType::Firecracker),
            "wasm" => Ok(WorkerType::Wasm),
            _ => Err(anyhow::anyhow!("Invalid worker type: {}", s)),
        }
    }
}

/// Worker operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkerStatus {
    /// Worker is online and accepting jobs
    Online,
    /// Worker is draining (finishing current jobs, no new assignments)
    Draining,
    /// Worker is offline (marked dead by health monitor)
    Offline,
}

impl std::fmt::Display for WorkerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkerStatus::Online => write!(f, "online"),
            WorkerStatus::Draining => write!(f, "draining"),
            WorkerStatus::Offline => write!(f, "offline"),
        }
    }
}

impl std::str::FromStr for WorkerStatus {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "online" => Ok(WorkerStatus::Online),
            "draining" => Ok(WorkerStatus::Draining),
            "offline" => Ok(WorkerStatus::Offline),
            _ => Err(anyhow::anyhow!("Invalid worker status: {}", s)),
        }
    }
}
/// Worker node in the distributed orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    /// Unique worker identifier
    pub id: String,
    /// Type of worker backend
    pub worker_type: WorkerType,
    /// Current operational status
    pub status: WorkerStatus,
    /// Iroh endpoint ID for P2P communication
    pub endpoint_id: String,
    /// Timestamp when worker registered (Unix epoch seconds)
    pub registered_at: i64,
    /// Timestamp of last heartbeat (Unix epoch seconds)
    pub last_heartbeat: i64,
    /// Number of CPU cores available
    pub cpu_cores: Option<u32>,
    /// Memory in megabytes available
    pub memory_mb: Option<u64>,
    /// Number of currently active jobs
    pub active_jobs: u32,
    /// Total number of jobs completed by this worker
    pub total_jobs_completed: u64,
    /// Additional metadata (version, hostname, etc.)
    pub metadata: JsonValue,
}

impl Worker {
    /// Check if worker is healthy (heartbeat within threshold)
    pub fn is_healthy(&self, heartbeat_timeout_secs: i64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs() as i64;
        let age = now - self.last_heartbeat;
        age < heartbeat_timeout_secs
    }

    /// Get time since last heartbeat in seconds
    pub fn heartbeat_age_seconds(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_secs() as i64;
        now - self.last_heartbeat
    }

    /// Check if worker can accept new jobs
    pub fn can_accept_jobs(&self) -> bool {
        matches!(self.status, WorkerStatus::Online)
    }
}

/// Worker registration request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistration {
    /// Type of worker backend
    pub worker_type: WorkerType,
    /// Iroh endpoint ID
    pub endpoint_id: String,
    /// Number of CPU cores available
    pub cpu_cores: Option<u32>,
    /// Memory in megabytes available
    pub memory_mb: Option<u64>,
    /// Additional metadata
    pub metadata: JsonValue,
}

/// Worker heartbeat update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeat {
    /// Worker identifier
    pub worker_id: String,
    /// Number of currently active jobs
    pub active_jobs: u32,
    /// Updated CPU cores (optional)
    pub cpu_cores: Option<u32>,
    /// Updated memory (optional)
    pub memory_mb: Option<u64>,
}

/// Aggregate statistics for worker pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    /// Total number of registered workers
    pub total: usize,
    /// Number of online workers
    pub online: usize,
    /// Number of draining workers
    pub draining: usize,
    /// Number of offline workers
    pub offline: usize,
    /// Total active jobs across all workers
    pub total_active_jobs: u32,
    /// Total completed jobs across all workers
    pub total_completed_jobs: u64,
}

impl WorkerStats {
    /// Create empty statistics
    pub fn empty() -> Self {
        Self {
            total: 0,
            online: 0,
            draining: 0,
            offline: 0,
            total_active_jobs: 0,
            total_completed_jobs: 0,
        }
    }

    /// Calculate from a collection of workers
    pub fn from_workers(workers: &[Worker]) -> Self {
        let mut stats = Self::empty();
        stats.total = workers.len();
        for worker in workers {
            match worker.status {
                WorkerStatus::Online => stats.online += 1,
                WorkerStatus::Draining => stats.draining += 1,
                WorkerStatus::Offline => stats.offline += 1,
            }
            stats.total_active_jobs += worker.active_jobs;
            stats.total_completed_jobs += worker.total_jobs_completed;
        }
        stats
    }
}
