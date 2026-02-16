// DNS response types.

use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResponse {
    pub domain: String,
    pub record_type: String,
    pub ttl_seconds: u32,
    pub data_json: String,
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    #[serde(rename = "found")]
    pub was_found: bool,
    pub record: Option<DnsRecordResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordsResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub records: Vec<DnsRecordResponse>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteRecordResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    #[serde(rename = "deleted")]
    pub was_deleted: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResponse {
    pub name: String,
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    #[serde(rename = "default_ttl")]
    pub default_ttl_secs: u32,
    pub serial: u32,
    pub last_modified_ms: u64,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    #[serde(rename = "found")]
    pub was_found: bool,
    pub zone: Option<DnsZoneResponse>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZonesResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    pub zones: Vec<DnsZoneResponse>,
    pub count: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteZoneResultResponse {
    #[serde(rename = "success")]
    pub is_success: bool,
    #[serde(rename = "deleted")]
    pub was_deleted: bool,
    pub records_deleted: u32,
    pub error: Option<String>,
}
