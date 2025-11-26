//! Cluster status business logic
//!
//! Encapsulates logic for determining cluster health, worker status,
//! and node information by aggregating data from multiple services.

use std::collections::HashSet;
use std::sync::Arc;
use anyhow::Result;

use crate::repositories::{StateRepository, WorkRepository};
use crate::domain::types::Job;

/// Aggregated cluster health information
///
/// Combines database cluster health (from HealthStatus) with worker activity metrics.
#[derive(Debug, Clone)]
pub struct AggregatedClusterHealth {
    pub is_healthy: bool,
    pub node_count: usize,
    pub has_leader: bool,
    pub active_worker_count: usize,
}

/// Individual worker statistics
#[derive(Debug, Clone)]
pub struct WorkerStats {
    pub node_id: String,
    pub active_jobs: usize,
    pub completed_jobs: usize,
    pub last_seen_timestamp: i64,
    pub is_active: bool,
}

/// Control plane node information
#[derive(Debug, Clone)]
pub struct ControlPlaneNode {
    pub node_number: usize,
    pub is_leader: bool,
    pub is_active: bool,
}

/// Domain service for cluster status operations
pub struct ClusterStatusService {
    state_repo: Arc<dyn StateRepository>,
    work_repo: Arc<dyn WorkRepository>,
}

impl ClusterStatusService {
    /// Create a new cluster status service
    pub fn new(state_repo: Arc<dyn StateRepository>, work_repo: Arc<dyn WorkRepository>) -> Self {
        Self { state_repo, work_repo }
    }

    /// Get aggregated cluster health status
    pub async fn get_cluster_health(&self) -> Result<AggregatedClusterHealth> {
        // Get control plane health from state repository
        let health_check = self.state_repo.health_check().await?;

        // Count active workers from recent job activity
        let jobs = self.work_repo.list_work().await?;
        let active_workers = Self::count_active_workers(&jobs);

        Ok(AggregatedClusterHealth {
            is_healthy: health_check.is_healthy,
            node_count: health_check.node_count,
            has_leader: health_check.has_leader,
            active_worker_count: active_workers.len(),
        })
    }

    /// Get detailed statistics for all workers
    pub async fn get_worker_stats(&self) -> Result<Vec<WorkerStats>> {
        let jobs = self.work_repo.list_work().await?;
        let stats_map = Self::aggregate_worker_stats(&jobs);

        let now = Self::current_timestamp();
        let mut stats: Vec<WorkerStats> = stats_map
            .into_iter()
            .map(|(node_id, (active, completed, last_seen))| {
                let seconds_ago = now - last_seen;
                WorkerStats {
                    node_id,
                    active_jobs: active,
                    completed_jobs: completed,
                    last_seen_timestamp: last_seen,
                    is_active: seconds_ago < 30, // Active if seen in last 30 seconds
                }
            })
            .collect();

        // Sort by most recent activity
        stats.sort_by(|a, b| b.last_seen_timestamp.cmp(&a.last_seen_timestamp));

        Ok(stats)
    }

    /// Get control plane node information
    pub async fn get_control_plane_nodes(&self) -> Result<Vec<ControlPlaneNode>> {
        let health_check = self.state_repo.health_check().await?;

        let mut nodes = Vec::new();
        for i in 1..=health_check.node_count {
            // Simplified assumption: node 1 is leader if there is one
            let is_leader = health_check.has_leader && i == 1;
            nodes.push(ControlPlaneNode {
                node_number: i,
                is_leader,
                is_active: true, // All nodes from health check are active
            });
        }

        Ok(nodes)
    }

    /// Count unique active workers from jobs
    fn count_active_workers(jobs: &[Job]) -> HashSet<String> {
        jobs
            .iter()
            .filter_map(|job| job.claimed_by.clone())
            .collect()
    }

    /// Aggregate worker statistics from jobs
    /// Returns: HashMap<node_id, (active_jobs, completed_jobs, last_seen_timestamp)>
    fn aggregate_worker_stats(
        jobs: &[Job],
    ) -> std::collections::HashMap<String, (usize, usize, i64)> {
        use crate::domain::types::JobStatus;

        let mut stats: std::collections::HashMap<String, (usize, usize, i64)> =
            std::collections::HashMap::new();

        for job in jobs {
            if let Some(node_id) = &job.claimed_by {
                let entry = stats.entry(node_id.clone()).or_insert((0, 0, 0));

                match job.status {
                    JobStatus::Completed => entry.1 += 1,
                    JobStatus::InProgress | JobStatus::Claimed => entry.0 += 1,
                    _ => {}
                }

                // Track most recent activity
                if job.updated_at > entry.2 {
                    entry.2 = job.updated_at;
                }
            }
        }

        stats
    }

    /// Get current Unix timestamp
    fn current_timestamp() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}
