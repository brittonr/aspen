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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::mocks::MockStateRepository;

    #[tokio::test]
    async fn test_check_database_health_returns_health_info() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 3,
            has_leader: true,
        }).await;

        let service = HealthService::new(state_repo);

        // Act
        let health = service.check_database_health().await.unwrap();

        // Assert
        assert!(health.is_healthy);
        assert_eq!(health.node_count, 3);
        assert!(health.has_leader);
    }

    #[tokio::test]
    async fn test_is_database_healthy_when_healthy() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 3,
            has_leader: true,
        }).await;

        let service = HealthService::new(state_repo);

        // Act
        let is_healthy = service.is_database_healthy().await;

        // Assert
        assert!(is_healthy);
    }

    #[tokio::test]
    async fn test_is_database_healthy_when_no_leader() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 2,
            has_leader: false, // No leader
        }).await;

        let service = HealthService::new(state_repo);

        // Act
        let is_healthy = service.is_database_healthy().await;

        // Assert
        assert!(!is_healthy); // Should be false without leader
    }

    #[tokio::test]
    async fn test_is_database_healthy_when_unhealthy() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        state_repo.set_health(HealthStatus {
            is_healthy: false,
            node_count: 1,
            has_leader: false,
        }).await;

        let service = HealthService::new(state_repo);

        // Act
        let is_healthy = service.is_database_healthy().await;

        // Assert
        assert!(!is_healthy);
    }

    #[tokio::test]
    async fn test_database_health_summary_when_healthy() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 3,
            has_leader: true,
        }).await;

        let service = HealthService::new(state_repo);

        // Act
        let summary = service.database_health_summary().await;

        // Assert
        assert_eq!(summary, "Healthy - 3 nodes, leader elected");
    }

    #[tokio::test]
    async fn test_database_health_summary_when_degraded() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        state_repo.set_health(HealthStatus {
            is_healthy: true,
            node_count: 2,
            has_leader: false,
        }).await;

        let service = HealthService::new(state_repo);

        // Act
        let summary = service.database_health_summary().await;

        // Assert
        assert_eq!(summary, "Degraded - 2 nodes, no leader");
    }

    #[tokio::test]
    async fn test_database_health_summary_when_unhealthy() {
        // Arrange
        let state_repo = Arc::new(MockStateRepository::new());
        state_repo.set_health(HealthStatus {
            is_healthy: false,
            node_count: 1,
            has_leader: false,
        }).await;

        let service = HealthService::new(state_repo);

        // Act
        let summary = service.database_health_summary().await;

        // Assert
        assert_eq!(summary, "Unhealthy - 1 nodes, no leader");
    }
}
