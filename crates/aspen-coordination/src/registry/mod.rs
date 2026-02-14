//! Distributed service registry for service discovery.
//!
//! This module provides a service registry for registering and discovering
//! service instances with TTL-based expiration and health status tracking.
//!
//! ## Features
//!
//! - Service registration with TTL/lease
//! - Health status tracking (Healthy, Unhealthy, Unknown)
//! - Service discovery with filters (tags, version, health)
//! - Heartbeat for TTL renewal
//! - Metadata (version, tags, weight for load balancing)
//! - Optional lease integration for auto-cleanup
//!
//! ## Example
//!
//! ```ignore
//! use aspen::coordination::{ServiceRegistry, HealthStatus, RegisterOptions};
//!
//! let registry = ServiceRegistry::new(store);
//!
//! // Register a service instance
//! let (token, deadline) = registry.register(
//!     "api-gateway",
//!     "instance-1",
//!     "192.168.1.100:8080",
//!     ServiceInstanceMetadata::default(),
//!     RegisterOptions::default(),
//! ).await?;
//!
//! // Discover healthy instances
//! let instances = registry.discover(
//!     "api-gateway",
//!     DiscoveryFilter { healthy_only: true, ..Default::default() },
//! ).await?;
//!
//! // Send heartbeat to renew TTL
//! registry.heartbeat("api-gateway", "instance-1", token).await?;
//! ```

mod discovery;
mod health;
mod helpers;
mod registration;
pub mod types;

use std::sync::Arc;

use aspen_traits::KeyValueStore;
pub use types::DiscoveryFilter;
pub use types::HealthStatus;
pub use types::RegisterOptions;
pub use types::ServiceInstance;
pub use types::ServiceInstanceMetadata;
pub use types::ServiceMetadata;

/// Manager for distributed service registry operations.
///
/// Provides service registration, discovery, and health management
/// built on CAS operations for linearizable consistency.
#[derive(Clone)]
pub struct ServiceRegistry<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> ServiceRegistry<S> {
    /// Create a new service registry manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    #[tokio::test]
    async fn test_register_discover() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        // Register an instance
        let (token, deadline) = registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata {
                    version: "1.0.0".to_string(),
                    ..Default::default()
                },
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(token, 1);
        assert!(deadline > 0);

        // Discover the instance
        let instances = registry.discover("test-service", DiscoveryFilter::default()).await.unwrap();

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].instance_id, "instance-1");
        assert_eq!(instances[0].address, "127.0.0.1:8080");
        assert_eq!(instances[0].metadata.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_deregister_removes_instance() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        let (token, _) = registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        // Deregister
        let removed = registry.deregister("test-service", "instance-1", token).await.unwrap();
        assert!(removed);

        // Should not be discoverable
        let instances = registry.discover("test-service", DiscoveryFilter::default()).await.unwrap();
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_heartbeat_renews_ttl() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        let (token, initial_deadline) = registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata::default(),
                RegisterOptions {
                    ttl_ms: Some(1000),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Wait a bit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Send heartbeat
        let (new_deadline, status) = registry.heartbeat("test-service", "instance-1", token).await.unwrap();

        assert!(new_deadline > initial_deadline);
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_health_status_update() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        let (token, _) = registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        // Update health to unhealthy
        registry.update_health("test-service", "instance-1", token, HealthStatus::Unhealthy).await.unwrap();

        // Check instance
        let instance = registry.get_instance("test-service", "instance-1").await.unwrap().unwrap();
        assert_eq!(instance.health_status, HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn test_discover_healthy_only_filter() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        // Register healthy instance
        registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        // Register unhealthy instance
        let (token2, _) = registry
            .register(
                "test-service",
                "instance-2",
                "127.0.0.1:8081",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        registry.update_health("test-service", "instance-2", token2, HealthStatus::Unhealthy).await.unwrap();

        // Discover all - should get both
        let all = registry.discover("test-service", DiscoveryFilter::default()).await.unwrap();
        assert_eq!(all.len(), 2);

        // Discover healthy only - should get one
        let healthy = registry
            .discover("test-service", DiscoveryFilter {
                healthy_only: true,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0].instance_id, "instance-1");
    }

    #[tokio::test]
    async fn test_discover_tags_filter() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        // Register with tags
        registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata {
                    tags: vec!["region:us-east".to_string(), "env:prod".to_string()],
                    ..Default::default()
                },
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        registry
            .register(
                "test-service",
                "instance-2",
                "127.0.0.1:8081",
                ServiceInstanceMetadata {
                    tags: vec!["region:us-west".to_string(), "env:prod".to_string()],
                    ..Default::default()
                },
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        // Filter by tag
        let filtered = registry
            .discover("test-service", DiscoveryFilter {
                tags: vec!["region:us-east".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].instance_id, "instance-1");
    }

    #[tokio::test]
    async fn test_fencing_token_prevents_stale_update() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        let (token1, _) = registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        // Re-register (simulating another process)
        let (token2, _) = registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        assert_eq!(token2, token1 + 1);

        // Old token should fail
        let result = registry.heartbeat("test-service", "instance-1", token1).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("fencing token"));

        // New token should work
        let result = registry.heartbeat("test-service", "instance-1", token2).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_instances_same_service() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        // Register multiple instances
        for i in 1..=5 {
            registry
                .register(
                    "test-service",
                    &format!("instance-{}", i),
                    &format!("127.0.0.1:808{}", i),
                    ServiceInstanceMetadata::default(),
                    RegisterOptions::default(),
                )
                .await
                .unwrap();
        }

        // Discover all
        let instances = registry.discover("test-service", DiscoveryFilter::default()).await.unwrap();
        assert_eq!(instances.len(), 5);
    }

    #[tokio::test]
    async fn test_list_services_by_prefix() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        // Register instances for different services
        registry
            .register(
                "api-gateway",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        registry
            .register(
                "api-users",
                "instance-1",
                "127.0.0.1:8081",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        registry
            .register(
                "worker-jobs",
                "instance-1",
                "127.0.0.1:8082",
                ServiceInstanceMetadata::default(),
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        // List services with "api-" prefix
        let services = registry.discover_services("api-", 100).await.unwrap();
        assert_eq!(services.len(), 2);
        assert!(services.contains(&"api-gateway".to_string()));
        assert!(services.contains(&"api-users".to_string()));
    }

    #[tokio::test]
    async fn test_metadata_update() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let registry = ServiceRegistry::new(store);

        let (token, _) = registry
            .register(
                "test-service",
                "instance-1",
                "127.0.0.1:8080",
                ServiceInstanceMetadata {
                    version: "1.0.0".to_string(),
                    weight: 100,
                    ..Default::default()
                },
                RegisterOptions::default(),
            )
            .await
            .unwrap();

        // Update metadata
        registry
            .update_metadata("test-service", "instance-1", token, ServiceInstanceMetadata {
                version: "2.0.0".to_string(),
                weight: 200,
                tags: vec!["updated".to_string()],
                ..Default::default()
            })
            .await
            .unwrap();

        // Verify update
        let instance = registry.get_instance("test-service", "instance-1").await.unwrap().unwrap();
        assert_eq!(instance.metadata.version, "2.0.0");
        assert_eq!(instance.metadata.weight, 200);
        assert!(instance.metadata.tags.contains(&"updated".to_string()));
    }
}
