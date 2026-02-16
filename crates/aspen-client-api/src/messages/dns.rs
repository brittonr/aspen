//! DNS operation types.
//!
//! Request/response types for DNS record and zone management operations.

use serde::Deserialize;
use serde::Serialize;

/// DNS domain request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DnsRequest {
    /// Set a DNS record.
    DnsSetRecord {
        domain: String,
        record_type: String,
        ttl_seconds: u32,
        data_json: String,
    },
    /// Get a DNS record.
    DnsGetRecord { domain: String, record_type: String },
    /// Get all DNS records for a domain.
    DnsGetRecords { domain: String },
    /// Delete a DNS record.
    DnsDeleteRecord { domain: String, record_type: String },
    /// Resolve a domain (with wildcard matching).
    DnsResolve { domain: String, record_type: String },
    /// Scan DNS records by prefix.
    DnsScanRecords { prefix: String, limit: u32 },
    /// Create or update a DNS zone.
    DnsSetZone {
        name: String,
        #[serde(rename = "enabled")]
        is_enabled: bool,
        default_ttl_secs: u32,
        description: Option<String>,
    },
    /// Get a DNS zone.
    DnsGetZone { name: String },
    /// List all DNS zones.
    DnsListZones,
    /// Delete a DNS zone.
    DnsDeleteZone {
        name: String,
        #[serde(rename = "delete_records")]
        should_delete_records: bool,
    },
}

impl DnsRequest {
    /// Convert to an authorization operation.
    pub fn to_operation(&self) -> Option<aspen_auth::Operation> {
        use aspen_auth::Operation;
        match self {
            Self::DnsSetRecord { domain, .. }
            | Self::DnsDeleteRecord { domain, .. }
            | Self::DnsSetZone { name: domain, .. }
            | Self::DnsDeleteZone { name: domain, .. } => Some(Operation::Write {
                key: format!("_dns:{domain}"),
                value: vec![],
            }),
            Self::DnsGetRecord { domain, .. }
            | Self::DnsGetRecords { domain }
            | Self::DnsResolve { domain, .. }
            | Self::DnsGetZone { name: domain } => Some(Operation::Read {
                key: format!("_dns:{domain}"),
            }),
            Self::DnsScanRecords { prefix, .. } => Some(Operation::Read {
                key: format!("_dns:{prefix}"),
            }),
            Self::DnsListZones => Some(Operation::Read {
                key: "_dns:".to_string(),
            }),
        }
    }
}

/// DNS record response structure.
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
    pub is_success: bool,
    /// Whether a record was found (for get operations).
    pub was_found: bool,
    /// The record (if found or created).
    pub record: Option<DnsRecordResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS records operation result (multiple records).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordsResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
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
    pub is_success: bool,
    /// Whether the record existed and was deleted.
    pub was_deleted: bool,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS zone response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZoneResponse {
    /// Zone name (e.g., "example.com").
    pub name: String,
    /// Whether the zone is enabled.
    #[serde(rename = "enabled")]
    pub is_enabled: bool,
    /// Default TTL for records in this zone (seconds).
    #[serde(rename = "default_ttl")]
    pub default_ttl_secs: u32,
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
    pub is_success: bool,
    /// Whether a zone was found (for get operations).
    pub was_found: bool,
    /// The zone (if found or created).
    pub zone: Option<DnsZoneResponse>,
    /// Error message if the operation failed.
    pub error: Option<String>,
}

/// DNS zones list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZonesResultResponse {
    /// Whether the operation succeeded.
    pub is_success: bool,
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
    pub is_success: bool,
    /// Whether the zone existed and was deleted.
    pub was_deleted: bool,
    /// Number of records deleted (if delete_records was true).
    pub records_deleted: u32,
    /// Error message if the operation failed.
    pub error: Option<String>,
}
