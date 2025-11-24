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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::{MockStateRepository, MockWorkRepository};
    use crate::domain::types::{Job, JobStatus, HealthStatus};

    #[tokio::test]
    async fn test_get_cluster_health_aggregates_data() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        // Set up hiqlite cluster health
        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 3,
            has_leader: true,
        }).await;

        // Add jobs with worker claims
        work_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({"url": "https://example.com"}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-2".to_string()),
                completed_by: Some("worker-2".to_string()),
                created_at: 1000,
                updated_at: 1020,
                payload: serde_json::json!({"url": "https://example.org"}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
        ]).await;

        let service = ClusterStatusService::new(state_repo, work_repo);

        // Act
        let health = service.get_cluster_health().await.unwrap();

        // Assert
        assert!(health.is_healthy);
        assert_eq!(health.node_count, 3);
        assert!(health.has_leader);
        assert_eq!(health.active_worker_count, 2); // Two unique workers
    }

    #[tokio::test]
    async fn test_get_cluster_health_counts_unique_workers() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 1,
            has_leader: true,
        }).await;

        // Same worker claims multiple jobs
        work_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Claimed,
                claimed_by: Some("worker-1".to_string()), // Same worker
                completed_by: None,
                created_at: 1000,
                updated_at: 1015,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
        ]).await;

        let service = ClusterStatusService::new(state_repo, work_repo);

        // Act
        let health = service.get_cluster_health().await.unwrap();

        // Assert
        assert_eq!(health.active_worker_count, 1); // Only one unique worker
    }

    #[tokio::test]
    async fn test_get_worker_stats_aggregates_correctly() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        work_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: now - 100,
                updated_at: now - 10, // Recent activity
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: now - 200,
                updated_at: now - 20,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-2".to_string()),
                completed_by: Some("worker-2".to_string()),
                created_at: now - 150,
                updated_at: now - 100, // Old activity
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
        ]).await;

        let service = ClusterStatusService::new(state_repo, work_repo);

        // Act
        let stats = service.get_worker_stats().await.unwrap();

        // Assert
        assert_eq!(stats.len(), 2);

        // Find worker-1 stats
        let worker1 = stats.iter().find(|s| s.node_id == "worker-1").unwrap();
        assert_eq!(worker1.active_jobs, 1); // InProgress
        assert_eq!(worker1.completed_jobs, 1);
        assert!(worker1.is_active); // Recent activity within 30 seconds

        // Find worker-2 stats
        let worker2 = stats.iter().find(|s| s.node_id == "worker-2").unwrap();
        assert_eq!(worker2.active_jobs, 0);
        assert_eq!(worker2.completed_jobs, 1);
        assert!(!worker2.is_active); // Activity more than 30 seconds ago
    }

    #[tokio::test]
    async fn test_get_worker_stats_sorted_by_recent_activity() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        work_repo.add_jobs(vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-old".to_string()),
                completed_by: Some("worker-old".to_string()),
                created_at: 1000,
                updated_at: 1100, // Older
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-new".to_string()),
                completed_by: None,
                created_at: 2000,
                updated_at: 2500, // More recent
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
        ]).await;

        let service = ClusterStatusService::new(state_repo, work_repo);

        // Act
        let stats = service.get_worker_stats().await.unwrap();

        // Assert
        assert_eq!(stats.len(), 2);
        // First worker should be the most recently active
        assert_eq!(stats[0].node_id, "worker-new");
        assert_eq!(stats[1].node_id, "worker-old");
    }

    #[tokio::test]
    async fn test_get_control_plane_nodes_when_leader_exists() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 3,
            has_leader: true,
        }).await;

        let service = ClusterStatusService::new(state_repo, work_repo);

        // Act
        let nodes = service.get_control_plane_nodes().await.unwrap();

        // Assert
        assert_eq!(nodes.len(), 3);

        // Node 1 should be leader
        assert_eq!(nodes[0].node_number, 1);
        assert!(nodes[0].is_leader);
        assert!(nodes[0].is_active);

        // Other nodes should not be leader
        assert_eq!(nodes[1].node_number, 2);
        assert!(!nodes[1].is_leader);
        assert!(nodes[1].is_active);
    }

    #[tokio::test]
    async fn test_get_control_plane_nodes_when_no_leader() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        let work_repo = Arc::new(MockWorkRepository::new());

        state_repo.set_health(HealthStatus {
            is_healthy: false,
            node_count: 2,
            has_leader: false, // No leader
        }).await;

        let service = ClusterStatusService::new(state_repo, work_repo);

        // Act
        let nodes = service.get_control_plane_nodes().await.unwrap();

        // Assert
        assert_eq!(nodes.len(), 2);
        // No node should be marked as leader
        assert!(!nodes[0].is_leader);
        assert!(!nodes[1].is_leader);
    }

    #[tokio::test]
    async fn test_count_active_workers_ignores_unclaimed_jobs() {
        // Arrange
        let jobs = vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::Pending,
                claimed_by: None, // Not claimed
                completed_by: None,
                created_at: 1000,
                updated_at: 1000,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
        ];

        // Act
        let workers = ClusterStatusService::count_active_workers(&jobs);

        // Assert
        assert_eq!(workers.len(), 1);
        assert!(workers.contains("worker-1"));
    }

    #[tokio::test]
    async fn test_aggregate_worker_stats_counts_by_status() {
        // Arrange
        let jobs = vec![
            Job {
                id: "job-1".to_string(),
                status: JobStatus::InProgress,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1010,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-2".to_string(),
                status: JobStatus::Claimed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1015,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-3".to_string(),
                status: JobStatus::Completed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: Some("worker-1".to_string()),
                created_at: 1000,
                updated_at: 1020,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
            Job {
                id: "job-4".to_string(),
                status: JobStatus::Failed,
                claimed_by: Some("worker-1".to_string()),
                completed_by: None,
                created_at: 1000,
                updated_at: 1025,
                payload: serde_json::json!({}),
                started_at: None,
                error_message: None,
                retry_count: 0,
                assigned_worker_id: None,
                compatible_worker_types: Vec::new(),
            },
        ];

        // Act
        let stats = ClusterStatusService::aggregate_worker_stats(&jobs);

        // Assert
        let worker1_stats = stats.get("worker-1").unwrap();
        assert_eq!(worker1_stats.0, 2); // Active: InProgress + Claimed
        assert_eq!(worker1_stats.1, 1); // Completed: 1
        assert_eq!(worker1_stats.2, 1025); // Most recent timestamp
    }
}
