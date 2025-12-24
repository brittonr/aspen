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

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use anyhow::bail;
use serde::Deserialize;
use serde::Serialize;
use tracing::debug;

use crate::api::KeyValueStore;
use crate::api::KeyValueStoreError;
use crate::api::ReadRequest;
use crate::api::WriteCommand;
use crate::api::WriteRequest;
use crate::coordination::types::now_unix_ms;
use crate::raft::constants::DEFAULT_SERVICE_TTL_MS;
use crate::raft::constants::MAX_SERVICE_DISCOVERY_RESULTS;
use crate::raft::constants::MAX_SERVICE_TTL_MS;
use crate::raft::constants::SERVICE_CLEANUP_BATCH;

/// Service registry key prefix.
const SERVICE_PREFIX: &str = "__service:";

/// Health status of a service instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HealthStatus {
    /// Instance is healthy and ready to receive traffic.
    #[default]
    Healthy,
    /// Instance is running but not healthy.
    Unhealthy,
    /// Health status is not known (no recent heartbeat).
    Unknown,
}

impl HealthStatus {
    /// Parse from string representation.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "healthy" => Some(Self::Healthy),
            "unhealthy" => Some(Self::Unhealthy),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }

    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }
}

/// Metadata for a service instance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceInstanceMetadata {
    /// Version string (e.g., "1.2.3").
    pub version: String,
    /// Tags for filtering/routing (e.g., ["canary", "region:us-east"]).
    pub tags: Vec<String>,
    /// Weight for load balancing (higher = more traffic, default 100).
    pub weight: u32,
    /// Custom metadata as key-value pairs.
    pub custom: HashMap<String, String>,
}

/// A service instance registered with the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    /// Unique instance identifier.
    pub instance_id: String,
    /// Service name this instance belongs to.
    pub service_name: String,
    /// Network address (e.g., "192.168.1.100:8080").
    pub address: String,
    /// Current health status.
    pub health_status: HealthStatus,
    /// Instance metadata.
    pub metadata: ServiceInstanceMetadata,
    /// Registration time (Unix ms).
    pub registered_at_ms: u64,
    /// Last heartbeat time (Unix ms).
    pub last_heartbeat_ms: u64,
    /// TTL deadline in Unix ms (0 = no expiration).
    pub deadline_ms: u64,
    /// TTL duration in ms (for heartbeat renewal).
    pub ttl_ms: u64,
    /// Optional lease ID for integration with lease system.
    pub lease_id: Option<u64>,
    /// Fencing token to prevent stale updates.
    pub fencing_token: u64,
}

impl ServiceInstance {
    /// Check if this instance has expired.
    pub fn is_expired(&self) -> bool {
        self.deadline_ms > 0 && now_unix_ms() > self.deadline_ms
    }

    /// Get remaining TTL in milliseconds.
    pub fn remaining_ttl_ms(&self) -> u64 {
        if self.deadline_ms == 0 {
            return u64::MAX;
        }
        self.deadline_ms.saturating_sub(now_unix_ms())
    }
}

/// Service-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceMetadata {
    /// Service name.
    pub name: String,
    /// Number of registered instances.
    pub instance_count: u32,
    /// Creation time (Unix ms).
    pub created_at_ms: u64,
    /// Fencing token for service-level operations.
    pub fencing_token: u64,
}

/// Options for service registration.
#[derive(Debug, Clone, Default)]
pub struct RegisterOptions {
    /// Instance TTL in milliseconds (0 = use default 30s).
    pub ttl_ms: Option<u64>,
    /// Initial health status.
    pub initial_status: Option<HealthStatus>,
    /// Attach to an existing lease instead of using TTL.
    pub lease_id: Option<u64>,
}

/// Filter for service discovery.
#[derive(Debug, Clone, Default)]
pub struct DiscoveryFilter {
    /// Only return healthy instances.
    pub healthy_only: bool,
    /// Filter by tags (all must match).
    pub tags: Vec<String>,
    /// Filter by version prefix.
    pub version_prefix: Option<String>,
    /// Maximum instances to return.
    pub limit: Option<u32>,
}

/// Manager for distributed service registry operations.
///
/// Provides service registration, discovery, and health management
/// built on CAS operations for linearizable consistency.
pub struct ServiceRegistry<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> ServiceRegistry<S> {
    /// Create a new service registry manager.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    // =========================================================================
    // Registration Operations
    // =========================================================================

    /// Register a service instance.
    ///
    /// Returns (fencing_token, deadline_ms) on success.
    /// If instance already exists, updates it and returns new token.
    pub async fn register(
        &self,
        service_name: &str,
        instance_id: &str,
        address: &str,
        metadata: ServiceInstanceMetadata,
        options: RegisterOptions,
    ) -> Result<(u64, u64)> {
        let now = now_unix_ms();
        let ttl_ms = options.ttl_ms.unwrap_or(DEFAULT_SERVICE_TTL_MS).min(MAX_SERVICE_TTL_MS);
        let deadline_ms = if options.lease_id.is_some() {
            // Lease-based: no TTL deadline, lease handles expiration
            0
        } else {
            now + ttl_ms
        };

        let key = Self::instance_key(service_name, instance_id);

        loop {
            // Read existing instance if any
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            let (fencing_token, registered_at_ms) = match &existing {
                Some(inst) => (inst.fencing_token + 1, inst.registered_at_ms),
                None => (1, now),
            };

            let instance = ServiceInstance {
                instance_id: instance_id.to_string(),
                service_name: service_name.to_string(),
                address: address.to_string(),
                health_status: options.initial_status.unwrap_or(HealthStatus::Healthy),
                metadata: metadata.clone(),
                registered_at_ms,
                last_heartbeat_ms: now,
                deadline_ms,
                ttl_ms,
                lease_id: options.lease_id,
                fencing_token,
            };

            let new_json = serde_json::to_string(&instance)?;

            let command = match &existing {
                Some(old) => {
                    let old_json = serde_json::to_string(old)?;
                    WriteCommand::CompareAndSwap {
                        key: key.clone(),
                        expected: Some(old_json),
                        new_value: new_json,
                    }
                }
                None => WriteCommand::Set {
                    key: key.clone(),
                    value: new_json,
                },
            };

            match self.store.write(WriteRequest { command }).await {
                Ok(_) => {
                    debug!(service_name, instance_id, fencing_token, deadline_ms, "instance registered");
                    return Ok((fencing_token, deadline_ms));
                }
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => {
                    // Retry on CAS failure
                    continue;
                }
                Err(e) => bail!("failed to register instance: {}", e),
            }
        }
    }

    /// Deregister a service instance.
    ///
    /// Returns true if instance was found and removed.
    pub async fn deregister(&self, service_name: &str, instance_id: &str, fencing_token: u64) -> Result<bool> {
        let key = Self::instance_key(service_name, instance_id);

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => return Ok(false),
                Some(inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: String::new(), // Empty = delete
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, "instance deregistered");
                            return Ok(true);
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to deregister instance: {}", e),
                    }
                }
            }
        }
    }

    // =========================================================================
    // Discovery Operations
    // =========================================================================

    /// Discover all instances of a service.
    ///
    /// Automatically cleans up expired instances during discovery.
    pub async fn discover(&self, service_name: &str, filter: DiscoveryFilter) -> Result<Vec<ServiceInstance>> {
        // Cleanup expired instances first
        let _ = self.cleanup_expired(service_name).await;

        let prefix = format!("{}{}:", SERVICE_PREFIX, service_name);
        let limit = filter.limit.unwrap_or(MAX_SERVICE_DISCOVERY_RESULTS).min(MAX_SERVICE_DISCOVERY_RESULTS);

        let keys = self.scan_keys(&prefix, limit).await?;
        let mut instances = Vec::new();

        for key in keys {
            // Skip the service metadata key (no instance ID suffix)
            if key == format!("{}{}", SERVICE_PREFIX, service_name) {
                continue;
            }

            if let Some(instance) = self.read_json::<ServiceInstance>(&key).await?
                && !instance.is_expired()
            {
                // Apply filters
                if filter.healthy_only && instance.health_status != HealthStatus::Healthy {
                    continue;
                }

                if !filter.tags.is_empty() && !filter.tags.iter().all(|t| instance.metadata.tags.contains(t)) {
                    continue;
                }

                if let Some(ref prefix) = filter.version_prefix
                    && !instance.metadata.version.starts_with(prefix)
                {
                    continue;
                }

                instances.push(instance);

                if instances.len() >= limit as usize {
                    break;
                }
            }
        }

        Ok(instances)
    }

    /// Discover services by name prefix.
    ///
    /// Returns service names matching the prefix.
    pub async fn discover_services(&self, prefix: &str, limit: u32) -> Result<Vec<String>> {
        let full_prefix = format!("{}{}", SERVICE_PREFIX, prefix);
        let limit = limit.min(MAX_SERVICE_DISCOVERY_RESULTS);

        let keys = self.scan_keys(&full_prefix, limit).await?;

        // Extract unique service names from keys
        let mut services = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for key in keys {
            // Key format: __service:{name}:{instance_id}
            if let Some(rest) = key.strip_prefix(SERVICE_PREFIX)
                && let Some(colon_pos) = rest.find(':')
            {
                let service_name = &rest[..colon_pos];
                if seen.insert(service_name.to_string()) {
                    services.push(service_name.to_string());
                }
            }
        }

        Ok(services)
    }

    /// Get a specific service instance.
    pub async fn get_instance(&self, service_name: &str, instance_id: &str) -> Result<Option<ServiceInstance>> {
        let key = Self::instance_key(service_name, instance_id);

        if let Some(instance) = self.read_json::<ServiceInstance>(&key).await? {
            if instance.is_expired() {
                // Lazily delete expired instance
                let _ = self.delete_key(&key).await;
                return Ok(None);
            }
            return Ok(Some(instance));
        }

        Ok(None)
    }

    // =========================================================================
    // Health and Heartbeat Operations
    // =========================================================================

    /// Send heartbeat to renew instance TTL.
    ///
    /// Returns (new_deadline_ms, health_status) on success.
    pub async fn heartbeat(
        &self,
        service_name: &str,
        instance_id: &str,
        fencing_token: u64,
    ) -> Result<(u64, HealthStatus)> {
        let key = Self::instance_key(service_name, instance_id);
        let now = now_unix_ms();

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => bail!("instance not found: {}:{}", service_name, instance_id),
                Some(mut inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;

                    // Update heartbeat and deadline
                    inst.last_heartbeat_ms = now;
                    if inst.lease_id.is_none() {
                        // Only update deadline if not lease-based
                        inst.deadline_ms = now + inst.ttl_ms;
                    }

                    let new_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, new_deadline = inst.deadline_ms, "heartbeat sent");
                            return Ok((inst.deadline_ms, inst.health_status));
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to send heartbeat: {}", e),
                    }
                }
            }
        }
    }

    /// Update health status of an instance.
    pub async fn update_health(
        &self,
        service_name: &str,
        instance_id: &str,
        fencing_token: u64,
        status: HealthStatus,
    ) -> Result<()> {
        let key = Self::instance_key(service_name, instance_id);

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => bail!("instance not found: {}:{}", service_name, instance_id),
                Some(mut inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;
                    inst.health_status = status;
                    let new_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, status = status.as_str(), "health status updated");
                            return Ok(());
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to update health status: {}", e),
                    }
                }
            }
        }
    }

    /// Update instance metadata.
    pub async fn update_metadata(
        &self,
        service_name: &str,
        instance_id: &str,
        fencing_token: u64,
        metadata: ServiceInstanceMetadata,
    ) -> Result<()> {
        let key = Self::instance_key(service_name, instance_id);

        loop {
            let existing = self.read_json::<ServiceInstance>(&key).await?;

            match existing {
                None => bail!("instance not found: {}:{}", service_name, instance_id),
                Some(mut inst) => {
                    if inst.fencing_token != fencing_token {
                        bail!("fencing token mismatch: expected {}, got {}", inst.fencing_token, fencing_token);
                    }

                    let old_json = serde_json::to_string(&inst)?;
                    inst.metadata = metadata.clone();
                    let new_json = serde_json::to_string(&inst)?;

                    match self
                        .store
                        .write(WriteRequest {
                            command: WriteCommand::CompareAndSwap {
                                key: key.clone(),
                                expected: Some(old_json),
                                new_value: new_json,
                            },
                        })
                        .await
                    {
                        Ok(_) => {
                            debug!(service_name, instance_id, "metadata updated");
                            return Ok(());
                        }
                        Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                        Err(e) => bail!("failed to update metadata: {}", e),
                    }
                }
            }
        }
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    /// Cleanup expired instances for a service.
    async fn cleanup_expired(&self, service_name: &str) -> Result<u32> {
        let prefix = format!("{}{}:", SERVICE_PREFIX, service_name);
        let keys = self.scan_keys(&prefix, SERVICE_CLEANUP_BATCH).await?;

        let mut cleaned = 0u32;

        for key in keys {
            if let Some(instance) = self.read_json::<ServiceInstance>(&key).await?
                && instance.is_expired()
            {
                let _ = self.delete_key(&key).await;
                cleaned += 1;
            }
        }

        if cleaned > 0 {
            debug!(service_name, cleaned, "expired instances cleaned up");
        }

        Ok(cleaned)
    }

    /// Generate instance key.
    fn instance_key(service_name: &str, instance_id: &str) -> String {
        format!("{}{}:{}", SERVICE_PREFIX, service_name, instance_id)
    }

    /// Read JSON from key.
    async fn read_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        match self.store.read(ReadRequest::new(key.to_string())).await {
            Ok(result) => {
                let value = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value.is_empty() {
                    Ok(None)
                } else {
                    let parsed = serde_json::from_str(&value)?;
                    Ok(Some(parsed))
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => bail!("failed to read {}: {}", key, e),
        }
    }

    /// Scan keys with prefix.
    async fn scan_keys(&self, prefix: &str, limit: u32) -> Result<Vec<String>> {
        use crate::api::ScanRequest;

        match self
            .store
            .scan(ScanRequest {
                prefix: prefix.to_string(),
                limit: Some(limit),
                continuation_token: None,
            })
            .await
        {
            Ok(result) => Ok(result.entries.iter().map(|e| e.key.clone()).collect()),
            Err(e) => bail!("failed to scan {}: {}", prefix, e),
        }
    }

    /// Delete a key.
    async fn delete_key(&self, key: &str) -> Result<()> {
        self.store
            .write(WriteRequest {
                command: WriteCommand::Delete { key: key.to_string() },
            })
            .await
            .map_err(|e| anyhow::anyhow!("failed to delete {}: {}", key, e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeterministicKeyValueStore;

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
            .discover(
                "test-service",
                DiscoveryFilter {
                    healthy_only: true,
                    ..Default::default()
                },
            )
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
            .discover(
                "test-service",
                DiscoveryFilter {
                    tags: vec!["region:us-east".to_string()],
                    ..Default::default()
                },
            )
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
            .update_metadata(
                "test-service",
                "instance-1",
                token,
                ServiceInstanceMetadata {
                    version: "2.0.0".to_string(),
                    weight: 200,
                    tags: vec!["updated".to_string()],
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        // Verify update
        let instance = registry.get_instance("test-service", "instance-1").await.unwrap().unwrap();
        assert_eq!(instance.metadata.version, "2.0.0");
        assert_eq!(instance.metadata.weight, 200);
        assert!(instance.metadata.tags.contains(&"updated".to_string()));
    }
}
