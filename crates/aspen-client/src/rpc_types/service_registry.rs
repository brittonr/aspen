// Service registry response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegisterResultResponse {
    pub success: bool,
    pub fencing_token: Option<u64>,
    pub deadline_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDeregisterResultResponse {
    pub success: bool,
    pub was_registered: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoverResultResponse {
    pub success: bool,
    pub instances: Vec<ServiceInstanceResponse>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceListResultResponse {
    pub success: bool,
    pub services: Vec<String>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceGetInstanceResultResponse {
    pub success: bool,
    pub found: bool,
    pub instance: Option<ServiceInstanceResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHeartbeatResultResponse {
    pub success: bool,
    pub new_deadline_ms: Option<u64>,
    pub health_status: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateHealthResultResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceUpdateMetadataResultResponse {
    pub success: bool,
    pub error: Option<String>,
}
