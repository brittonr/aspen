//! Worker domain types
//!
//! Core types for worker management in the distributed orchestrator.
//! These types represent worker nodes and their operational state.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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
        let now = crate::common::current_timestamp_or_zero();
        let age = now - self.last_heartbeat;
        age < heartbeat_timeout_secs
    }

    /// Get time since last heartbeat in seconds
    pub fn heartbeat_age_seconds(&self) -> i64 {
        let now = crate::common::current_timestamp_or_zero();
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
