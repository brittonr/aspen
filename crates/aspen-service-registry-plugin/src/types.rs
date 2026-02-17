//! Standalone service registry types for the WASM guest.
//!
//! These mirror `aspen_coordination::registry::types` but are self-contained
//! so the plugin has zero dependency on aspen crates.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

/// Health status of a service instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl HealthStatus {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "healthy" => Self::Healthy,
            "unhealthy" => Self::Unhealthy,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Unhealthy => "unhealthy",
            Self::Unknown => "unknown",
        }
    }
}

/// Metadata for a service instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstanceMetadata {
    pub version: String,
    pub tags: Vec<String>,
    pub weight: u32,
    pub custom: HashMap<String, String>,
}

impl Default for ServiceInstanceMetadata {
    fn default() -> Self {
        Self {
            version: String::new(),
            tags: Vec::new(),
            weight: 100,
            custom: HashMap::new(),
        }
    }
}

/// A service instance stored in KV as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub instance_id: String,
    pub service_name: String,
    pub address: String,
    pub health_status: HealthStatus,
    pub metadata: ServiceInstanceMetadata,
    pub registered_at_ms: u64,
    pub last_heartbeat_ms: u64,
    pub deadline_ms: u64,
    pub ttl_ms: u64,
    pub lease_id: Option<u64>,
    pub fencing_token: u64,
}

/// Response-format service instance for client RPC.
#[derive(Debug, Serialize)]
pub struct ServiceInstanceResponse {
    pub instance_id: String,
    pub service_name: String,
    pub address: String,
    pub health_status: String,
    pub version: String,
    pub tags: Vec<String>,
    pub weight: u32,
    pub custom_metadata: String,
    pub registered_at_ms: u64,
    pub last_heartbeat_ms: u64,
    pub deadline_ms: u64,
    pub lease_id: Option<u64>,
    pub fencing_token: u64,
}

impl ServiceInstance {
    /// Convert to the client RPC response format.
    pub fn to_response(&self) -> ServiceInstanceResponse {
        let custom_metadata = serde_json::to_string(&self.metadata.custom).unwrap_or_else(|_| "{}".to_string());

        ServiceInstanceResponse {
            instance_id: self.instance_id.clone(),
            service_name: self.service_name.clone(),
            address: self.address.clone(),
            health_status: self.health_status.as_str().to_string(),
            version: self.metadata.version.clone(),
            tags: self.metadata.tags.clone(),
            weight: self.metadata.weight,
            custom_metadata,
            registered_at_ms: self.registered_at_ms,
            last_heartbeat_ms: self.last_heartbeat_ms,
            deadline_ms: self.deadline_ms,
            lease_id: self.lease_id,
            fencing_token: self.fencing_token,
        }
    }
}
