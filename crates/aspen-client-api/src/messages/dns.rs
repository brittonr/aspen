//! DNS response types.
//!
//! Response types for DNS record and zone management operations.

use serde::{Deserialize, Serialize};

/// DNS record response structure.
///
/// Contains a single DNS record in JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResponse {
    /// Domain name.
    pub domain: String,
    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, NS, SOA, PTR, CAA).
    pub record_type: String,
    /// TTL in seconds.
    pub ttl_seconds: u32,
    /// Record data as JSON string (format depends on type).
    pub data_json: String,
    /// Unix timestamp when record was last updated (milliseconds).
    pub updated_at_ms: u64,
}

/// DNS record operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether a record was found (for get operations).
    pub found: bool,
    /// The record (if found or created).
    pub record: Option<DnsRecordResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS records operation result (multiple records).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordsResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of records.
    pub records: Vec<DnsRecordResponse>,
    /// Number of records returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS delete record result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteRecordResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the record existed and was deleted.
    pub deleted: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS zone response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResponse {
    /// Zone name (e.g., "example.com").
    pub name: String,
    /// Whether the zone is enabled.
    pub enabled: bool,
    /// Default TTL for records in this zone.
    pub default_ttl: u32,
    /// SOA serial number.
    pub serial: u32,
    /// Unix timestamp of last modification (milliseconds).
    pub last_modified_ms: u64,
    /// Optional description.
    pub description: Option<String>,
}

/// DNS zone operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether a zone was found (for get operations).
    pub found: bool,
    /// The zone (if found or created).
    pub zone: Option<DnsZoneResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS zones list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZonesResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// List of zones.
    pub zones: Vec<DnsZoneResponse>,
    /// Number of zones returned.
    pub count: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS delete zone result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsDeleteZoneResultResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Whether the zone existed and was deleted.
    pub deleted: bool,
    /// Number of records deleted (if delete_records was true).
    pub records_deleted: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}
