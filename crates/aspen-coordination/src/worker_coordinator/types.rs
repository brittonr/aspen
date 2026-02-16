//! Types for the distributed worker coordinator.

use std::collections::HashSet;

use serde::Deserialize;
use serde::Serialize;

use super::constants::STEAL_HINT_TTL_MS;
use crate::registry::HealthStatus;
use crate::types::now_unix_ms;

/// Worker information stored in the coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    /// Unique worker identifier.
    pub worker_id: String,
    /// Node ID hosting this worker.
    pub node_id: String,
    /// Iroh peer ID for P2P communication.
    pub peer_id: Option<String>,
    /// Worker capabilities (job types it can handle).
    pub capabilities: Vec<String>,
    /// Current load (0.0 = idle, 1.0 = fully loaded).
    pub load: f32,
    /// Number of jobs currently processing.
    pub active_jobs: u32,
    /// Maximum concurrent jobs.
    pub max_concurrent: u32,
    /// Queue depth at this worker.
    pub queue_depth: u32,
    /// Worker health status.
    pub health: HealthStatus,
    /// Custom tags for routing.
    pub tags: Vec<String>,
    /// Last heartbeat timestamp.
    pub last_heartbeat_ms: u64,
    /// Worker started timestamp.
    pub started_at_ms: u64,
    /// Total jobs processed.
    pub total_processed: u64,
    /// Total jobs failed.
    pub total_failed: u64,
    /// Average job processing time in ms.
    pub avg_processing_time_ms: u64,
    /// Worker group memberships.
    pub groups: HashSet<String>,
}

impl WorkerInfo {
    /// Calculate available capacity (0.0 = no capacity, 1.0 = full capacity).
    pub fn available_capacity(&self) -> f32 {
        crate::verified::calculate_available_capacity_f32(self.load, self.health == HealthStatus::Healthy)
    }

    /// Check if worker can handle a job type.
    pub fn can_handle(&self, job_type: &str) -> bool {
        crate::verified::can_handle_job(&self.capabilities, job_type)
    }

    /// Check if worker is alive based on heartbeat.
    pub fn is_alive(&self, timeout_ms: u64) -> bool {
        crate::verified::is_worker_alive(self.last_heartbeat_ms, now_unix_ms(), timeout_ms)
    }
}

/// Worker group for coordinated tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerGroup {
    /// Group identifier.
    pub group_id: String,
    /// Group description.
    pub description: String,
    /// Member worker IDs.
    pub members: HashSet<String>,
    /// Group leader worker ID (for coordination).
    pub leader: Option<String>,
    /// Required capabilities for group members.
    pub required_capabilities: Vec<String>,
    /// Minimum members needed for group to be active.
    pub min_members: u32,
    /// Maximum members allowed.
    pub max_members: u32,
    /// Group creation timestamp.
    pub created_at_ms: u64,
    /// Group state.
    pub state: GroupState,
}

/// State of a worker group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupState {
    /// Group is forming, waiting for members.
    Forming,
    /// Group is active and ready for tasks.
    Active,
    /// Group is executing a coordinated task.
    Executing,
    /// Group is disbanding.
    Disbanding,
}

/// A work stealing hint stored in the KV store.
///
/// Hints are coordination signals from the coordinator to workers,
/// indicating that a target worker should attempt to steal work
/// from a source worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealHint {
    /// Target worker ID (the worker that should steal).
    pub target_worker_id: String,
    /// Source worker ID (the worker to steal from).
    pub source_worker_id: String,
    /// Suggested batch size for stealing.
    pub batch_size: u32,
    /// When this hint was created (Unix ms).
    pub created_at_ms: u64,
    /// When this hint expires (Unix ms).
    pub expires_at_ms: u64,
    /// Round-robin index used for source selection (for debugging).
    pub source_index: u32,
}

impl StealHint {
    /// Create a new steal hint with TTL.
    pub fn new(target_worker_id: String, source_worker_id: String, batch_size: u32, source_index: u32) -> Self {
        let now = now_unix_ms();
        Self {
            target_worker_id,
            source_worker_id,
            batch_size,
            created_at_ms: now,
            expires_at_ms: now + STEAL_HINT_TTL_MS,
            source_index,
        }
    }

    /// Check if this hint has expired.
    pub fn is_expired(&self) -> bool {
        crate::verified::is_steal_hint_expired(self.expires_at_ms, now_unix_ms())
    }

    /// Get remaining TTL in milliseconds.
    pub fn remaining_ttl_ms(&self) -> u64 {
        crate::verified::steal_hint_remaining_ttl(self.expires_at_ms, now_unix_ms())
    }
}

/// Load balancing strategy for work distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingStrategy {
    /// Simple round-robin distribution.
    RoundRobin,
    /// Route to least loaded worker.
    LeastLoaded,
    /// Route based on worker affinity.
    Affinity,
    /// Consistent hashing for deterministic routing.
    ConsistentHash,
    /// Enable work stealing.
    WorkStealing,
}

/// Configuration for the distributed worker coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCoordinatorConfig {
    /// Load balancing strategy.
    pub strategy: LoadBalancingStrategy,
    /// Worker heartbeat timeout in milliseconds.
    pub heartbeat_timeout_ms: u64,
    /// Worker heartbeat interval in milliseconds.
    pub heartbeat_interval_ms: u64,
    /// Enable work stealing.
    pub enable_work_stealing: bool,
    /// Work stealing check interval in milliseconds.
    pub steal_check_interval_ms: u64,
    /// Load threshold for stealing (steal if load < threshold).
    pub steal_load_threshold: f32,
    /// Queue depth threshold for stealing source.
    pub steal_queue_threshold: u32,
    /// Enable automatic failover.
    pub enable_failover: bool,
    /// Failover check interval in milliseconds.
    pub failover_check_interval_ms: u64,
    /// Maximum workers to track.
    pub max_workers: u32,
    /// Maximum groups to manage.
    pub max_groups: u32,
}

impl Default for WorkerCoordinatorConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::LeastLoaded,
            heartbeat_timeout_ms: 30_000,  // 30 seconds
            heartbeat_interval_ms: 10_000, // 10 seconds
            enable_work_stealing: true,
            steal_check_interval_ms: 5_000, // 5 seconds
            steal_load_threshold: 0.2,      // Steal if load < 20%
            steal_queue_threshold: 10,      // Source must have > 10 jobs
            enable_failover: true,
            failover_check_interval_ms: 15_000, // 15 seconds
            max_workers: super::constants::MAX_WORKERS,
            max_groups: super::constants::MAX_GROUPS,
        }
    }
}

/// Filter for querying workers.
#[derive(Debug, Clone, Default)]
pub struct WorkerFilter {
    /// Filter by health status.
    pub health: Option<HealthStatus>,
    /// Filter by capability.
    pub capability: Option<String>,
    /// Filter by node ID.
    pub node_id: Option<String>,
    /// Filter by tags.
    pub tags: Option<Vec<String>>,
    /// Filter by maximum load.
    pub max_load: Option<f32>,
}

/// Worker statistics for heartbeat updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStats {
    /// Current load.
    pub load: f32,
    /// Active jobs.
    pub active_jobs: u32,
    /// Queue depth.
    pub queue_depth: u32,
    /// Total processed.
    pub total_processed: u64,
    /// Total failed.
    pub total_failed: u64,
    /// Average processing time.
    pub avg_processing_time_ms: u64,
    /// Health status.
    pub health: HealthStatus,
}
