//! DNS record types matching `aspen-dns` wire format.
//!
//! These types produce the exact same JSON as the native `aspen-dns::DnsRecord`,
//! enabling the WASM plugin to read and write records interchangeably with the
//! native handler.

use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

use serde::Deserialize;
use serde::Serialize;

// ============================================================================
// Record Type
// ============================================================================

/// DNS record type.
///
/// Serde: `#[serde(rename_all = "UPPERCASE")]` matches `aspen_dns::RecordType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RecordType {
    A,
    Aaaa,
    Cname,
    Mx,
    Txt,
    Srv,
    Ns,
    Soa,
    Ptr,
    Caa,
}

impl RecordType {
    /// Parse from case-insensitive string.
    pub fn from_str_ignore_case(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "A" => Some(Self::A),
            "AAAA" => Some(Self::Aaaa),
            "CNAME" => Some(Self::Cname),
            "MX" => Some(Self::Mx),
            "TXT" => Some(Self::Txt),
            "SRV" => Some(Self::Srv),
            "NS" => Some(Self::Ns),
            "SOA" => Some(Self::Soa),
            "PTR" => Some(Self::Ptr),
            "CAA" => Some(Self::Caa),
            _ => None,
        }
    }

    /// Get the uppercase string form (e.g., `"A"`, `"AAAA"`, `"MX"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Aaaa => "AAAA",
            Self::Cname => "CNAME",
            Self::Mx => "MX",
            Self::Txt => "TXT",
            Self::Srv => "SRV",
            Self::Ns => "NS",
            Self::Soa => "SOA",
            Self::Ptr => "PTR",
            Self::Caa => "CAA",
        }
    }
}

impl core::fmt::Display for RecordType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ============================================================================
// Record Data (tagged enum — must match aspen-dns wire format)
// ============================================================================

/// DNS record data.
///
/// Matches `aspen_dns::DnsRecordData` JSON format:
/// `{"type": "A", "addresses": ["1.2.3.4"]}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum DnsRecordData {
    A {
        addresses: Vec<Ipv4Addr>,
    },
    #[serde(rename = "AAAA")]
    Aaaa {
        addresses: Vec<Ipv6Addr>,
    },
    #[serde(rename = "CNAME")]
    Cname {
        target: String,
    },
    #[serde(rename = "MX")]
    Mx {
        records: Vec<MxRecord>,
    },
    #[serde(rename = "TXT")]
    Txt {
        strings: Vec<String>,
    },
    #[serde(rename = "SRV")]
    Srv {
        records: Vec<SrvRecord>,
    },
    #[serde(rename = "NS")]
    Ns {
        nameservers: Vec<String>,
    },
    #[serde(rename = "SOA")]
    Soa(SoaRecord),
    #[serde(rename = "PTR")]
    Ptr {
        target: String,
    },
    #[serde(rename = "CAA")]
    Caa {
        records: Vec<CaaRecord>,
    },
}

impl DnsRecordData {
    /// Get the record type for this data variant.
    pub fn record_type(&self) -> RecordType {
        match self {
            Self::A { .. } => RecordType::A,
            Self::Aaaa { .. } => RecordType::Aaaa,
            Self::Cname { .. } => RecordType::Cname,
            Self::Mx { .. } => RecordType::Mx,
            Self::Txt { .. } => RecordType::Txt,
            Self::Srv { .. } => RecordType::Srv,
            Self::Ns { .. } => RecordType::Ns,
            Self::Soa(_) => RecordType::Soa,
            Self::Ptr { .. } => RecordType::Ptr,
            Self::Caa { .. } => RecordType::Caa,
        }
    }

    /// Sort MX/SRV records by priority (in-place).
    pub fn sort_by_priority(&mut self) {
        match self {
            Self::Mx { records } => {
                records.sort_by_key(|r| r.priority);
            }
            Self::Srv { records } => {
                records.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| b.weight.cmp(&a.weight)));
            }
            _ => {}
        }
    }
}

// ============================================================================
// Child Record Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MxRecord {
    pub priority: u16,
    pub exchange: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoaRecord {
    pub mname: String,
    pub rname: String,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaaRecord {
    pub flags: u8,
    pub tag: String,
    pub value: String,
}

// ============================================================================
// DNS Record (full wire record)
// ============================================================================

/// A complete DNS record stored in KV.
///
/// Serialization matches `aspen_dns::DnsRecord` exactly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DnsRecord {
    pub domain: String,
    pub ttl_seconds: u32,
    pub data: DnsRecordData,
    pub updated_at_ms: u64,
}

impl DnsRecord {
    /// Create a new record with current timestamp.
    pub fn new(domain: String, ttl_seconds: u32, data: DnsRecordData) -> Self {
        let updated_at_ms = aspen_wasm_guest_sdk::host::current_time_ms();
        Self {
            domain,
            ttl_seconds,
            data,
            updated_at_ms,
        }
    }

    /// Deserialize from JSON bytes (same as `aspen_dns::DnsRecord::from_json_bytes`).
    pub fn from_json_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }

    /// Get the record type from the data variant.
    pub fn record_type(&self) -> RecordType {
        self.data.record_type()
    }
}

// ============================================================================
// Response Conversion
// ============================================================================

use aspen_client_api::DnsRecordResponse;

impl DnsRecord {
    /// Convert to the client API response type.
    pub fn to_response(&self) -> DnsRecordResponse {
        let data_json =
            serde_json::to_string(&self.data).unwrap_or_else(|e| format!(r#"{{"error":"serialization failed: {e}"}}"#));

        DnsRecordResponse {
            domain: self.domain.clone(),
            record_type: self.record_type().to_string(),
            ttl_seconds: self.ttl_seconds,
            data_json,
            updated_at_ms: self.updated_at_ms,
        }
    }
}
