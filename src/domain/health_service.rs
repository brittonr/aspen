//! Health service for system health checks
//!
//! Provides domain-level health check operations, encapsulating the business
//! logic for determining system health status.

use std::sync::Arc;
use anyhow::Result;

use crate::repositories::StateRepository;
use crate::domain::types::HealthStatus;

/// Domain service for health check operations
///
/// This service provides a domain-level API for health checks, isolating
/// handlers from direct infrastructure access.
pub struct HealthService {
    state_repo: Arc<dyn StateRepository>,
}

impl HealthService {
    /// Create a new health service
    pub fn new(state_repo: Arc<dyn StateRepository>) -> Self {
        Self { state_repo }
    }

    /// Check database cluster health
    ///
    /// Returns health information about the distributed database cluster,
    /// including node count, leader status, and overall health.
    pub async fn check_database_health(&self) -> Result<HealthStatus> {
        self.state_repo.health_check().await
    }

    /// Check if the database cluster is healthy
    ///
    /// Convenience method that returns a simple boolean health status.
    pub async fn is_database_healthy(&self) -> bool {
        match self.state_repo.health_check().await {
            Ok(health) => health.is_healthy && health.has_leader,
            Err(_) => false,
        }
    }

    /// Get database cluster summary
    ///
    /// Returns a human-readable summary of cluster health.
    pub async fn database_health_summary(&self) -> String {
        match self.state_repo.health_check().await {
            Ok(health) if health.is_healthy && health.has_leader => {
                format!(
                    "Healthy - {} nodes, leader elected",
                    health.node_count
                )
            }
            Ok(health) if health.is_healthy && !health.has_leader => {
                format!(
                    "Degraded - {} nodes, no leader",
                    health.node_count
                )
            }
            Ok(health) => {
                format!(
                    "Unhealthy - {} nodes, no leader",
                    health.node_count
                )
            }
            Err(e) => format!("Error: {}", e),
        }
    }
}
