//! Mock repository implementations for testing

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::hiqlite_service::ClusterHealth;
use crate::repositories::{StateRepository, WorkRepository};
use crate::work_queue::{WorkItem, WorkQueueStats, WorkStatus};

/// Mock implementation of StateRepository for testing
#[derive(Clone)]
pub struct MockStateRepository {
    health: Arc<Mutex<ClusterHealth>>,
}

impl MockStateRepository {
    /// Create a new mock repository with default healthy cluster
    pub fn new() -> Self {
        Self {
            health: Arc::new(Mutex::new(ClusterHealth {
                is_healthy: true,
                node_count: 3,
                has_leader: true,
            })),
        }
    }

    /// Set the cluster health to return
    pub async fn set_health(&self, health: ClusterHealth) {
        *self.health.lock().await = health;
    }
}

#[async_trait]
impl StateRepository for MockStateRepository {
    async fn health_check(&self) -> Result<ClusterHealth> {
        Ok(self.health.lock().await.clone())
    }
}

/// Mock implementation of WorkRepository for testing
#[derive(Clone)]
pub struct MockWorkRepository {
    work_items: Arc<Mutex<Vec<WorkItem>>>,
    stats: Arc<Mutex<WorkQueueStats>>,
}

impl MockWorkRepository {
    /// Create a new mock repository with empty queue
    pub fn new() -> Self {
        Self {
            work_items: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(WorkQueueStats {
                pending: 0,
                claimed: 0,
                in_progress: 0,
                completed: 0,
                failed: 0,
            })),
        }
    }

    /// Add work items for testing
    pub async fn add_work_items(&self, items: Vec<WorkItem>) {
        let mut work_items = self.work_items.lock().await;
        work_items.extend(items);
    }

    /// Set the stats to return
    pub async fn set_stats(&self, stats: WorkQueueStats) {
        *self.stats.lock().await = stats;
    }
}

#[async_trait]
impl WorkRepository for MockWorkRepository {
    async fn publish_work(&self, job_id: String, payload: JsonValue) -> Result<()> {
        let work_item = WorkItem {
            job_id,
            status: WorkStatus::Pending,
            claimed_by: None,
            completed_by: None,
            created_at: 0,
            updated_at: 0,
            payload,
        };
        self.work_items.lock().await.push(work_item);
        Ok(())
    }

    async fn claim_work(&self) -> Result<Option<WorkItem>> {
        let mut items = self.work_items.lock().await;
        Ok(items.iter_mut()
            .find(|item| item.status == WorkStatus::Pending)
            .map(|item| {
                item.status = WorkStatus::Claimed;
                item.clone()
            }))
    }

    async fn update_status(&self, job_id: &str, status: WorkStatus) -> Result<()> {
        let mut items = self.work_items.lock().await;
        if let Some(item) = items.iter_mut().find(|item| item.job_id == job_id) {
            item.status = status;
        }
        Ok(())
    }

    async fn list_work(&self) -> Result<Vec<WorkItem>> {
        Ok(self.work_items.lock().await.clone())
    }

    async fn stats(&self) -> WorkQueueStats {
        self.stats.lock().await.clone()
    }
}
