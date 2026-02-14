//! Type definitions for the service registry.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::types::now_unix_ms;

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
        crate::verified::is_instance_expired(self.deadline_ms, now_unix_ms())
    }

    /// Get remaining TTL in milliseconds.
    pub fn remaining_ttl_ms(&self) -> u64 {
        crate::verified::instance_remaining_ttl(self.deadline_ms, now_unix_ms())
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
