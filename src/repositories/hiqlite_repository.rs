//! Hiqlite-based implementation of StateRepository

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::domain::types::HealthStatus;
use crate::repositories::StateRepository;
use crate::services::traits::DatabaseHealth;

/// StateRepository implementation backed by DatabaseHealth trait
///
/// This implementation uses the DatabaseHealth trait for better testability
/// and decoupling from concrete database implementations.
#[derive(Clone)]
pub struct HiqliteStateRepository {
    db_health: Arc<dyn DatabaseHealth>,
}

impl HiqliteStateRepository {
    /// Create a new state repository with database health service
    pub fn new(db_health: Arc<dyn DatabaseHealth>) -> Self {
        Self { db_health }
    }
}

#[async_trait]
impl StateRepository for HiqliteStateRepository {
    async fn health_check(&self) -> Result<HealthStatus> {
        let cluster_health = self.db_health.health_check().await?;

        // Map infrastructure ClusterHealth to domain HealthStatus
        Ok(HealthStatus {
            is_healthy: cluster_health.is_healthy,
            node_count: cluster_health.node_count,
            has_leader: cluster_health.has_leader,
        })
    }
}
