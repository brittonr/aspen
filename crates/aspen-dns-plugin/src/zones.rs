//! DNS zone types matching `aspen-dns` wire format.

use aspen_client_api::DnsZoneResponse;
use aspen_wasm_guest_sdk::host;
use serde::Deserialize;
use serde::Serialize;

// ============================================================================
// Zone Types
// ============================================================================

/// DNS zone metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneMetadata {
    pub serial: u32,
    pub last_modified_ms: u64,
    pub description: Option<String>,
}

/// A DNS zone stored in KV.
///
/// Serialization matches `aspen_dns::Zone` exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    pub is_enabled: bool,
    #[serde(rename = "default_ttl")]
    pub default_ttl_secs: u32,
    pub metadata: ZoneMetadata,
}

impl Zone {
    /// Create a new zone with current timestamp.
    pub fn new(name: String, is_enabled: bool, default_ttl_secs: u32, description: Option<String>) -> Self {
        let now = host::current_time_ms();
        Self {
            name,
            is_enabled,
            default_ttl_secs,
            metadata: ZoneMetadata {
                serial: 1,
                last_modified_ms: now,
                description,
            },
        }
    }

    /// Deserialize from JSON bytes.
    pub fn from_json_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }

    /// Convert to the client API response type.
    pub fn to_response(&self) -> DnsZoneResponse {
        DnsZoneResponse {
            name: self.name.clone(),
            is_enabled: self.is_enabled,
            default_ttl_secs: self.default_ttl_secs,
            serial: self.metadata.serial,
            last_modified_ms: self.metadata.last_modified_ms,
            description: self.metadata.description.clone(),
        }
    }
}
