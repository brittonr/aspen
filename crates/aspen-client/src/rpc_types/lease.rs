// Lease operation response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseGrantResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub lease_id: Option<u64>,
    pub ttl_seconds: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRevokeResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub keys_deleted: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseKeepaliveResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub lease_id: Option<u64>,
    pub ttl_seconds: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseTimeToLiveResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub lease_id: Option<u64>,
    pub granted_ttl_seconds: Option<u32>,
    pub remaining_ttl_seconds: Option<u32>,
    pub keys: Option<Vec<String>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseListResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub leases: Option<Vec<LeaseInfo>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseInfo {
    pub lease_id: u64,
    pub granted_ttl_seconds: u32,
    pub remaining_ttl_seconds: u32,
    pub attached_keys: u32,
}
