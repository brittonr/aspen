//! DNS record types and data structures.
//!
//! This module defines the core types for DNS record management:
//! - `RecordType`: Enum of supported DNS record types (A, AAAA, MX, etc.)
//! - `DnsRecord`: Complete DNS record with domain, type, TTL, and data
//! - `DnsRecordData`: Type-specific record data (addresses, targets, etc.)
//! - `DnsEvent`: Events emitted when DNS records change

use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::constants::DNS_KEY_PREFIX;

// ============================================================================
// Record Type Enum
// ============================================================================

/// DNS record types supported by Aspen DNS.
///
/// Each variant corresponds to a standard DNS resource record type
/// as defined in RFC 1035 and subsequent RFCs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RecordType {
    /// IPv4 address record (RFC 1035).
    A,
    /// IPv6 address record (RFC 3596).
    AAAA,
    /// Canonical name (alias) record (RFC 1035).
    CNAME,
    /// Mail exchange record (RFC 1035).
    MX,
    /// Text record (RFC 1035).
    TXT,
    /// Service locator record (RFC 2782).
    SRV,
    /// Nameserver record (RFC 1035).
    NS,
    /// Start of authority record (RFC 1035).
    SOA,
    /// Pointer record for reverse DNS (RFC 1035).
    PTR,
    /// Certification Authority Authorization (RFC 8659).
    CAA,
}

impl RecordType {
    /// Convert to the key suffix used in Aspen KV store.
    ///
    /// # Example
    /// ```
    /// use aspen::dns::RecordType;
    /// assert_eq!(RecordType::A.as_key_suffix(), "A");
    /// assert_eq!(RecordType::AAAA.as_key_suffix(), "AAAA");
    /// ```
    pub const fn as_key_suffix(&self) -> &'static str {
        match self {
            RecordType::A => "A",
            RecordType::AAAA => "AAAA",
            RecordType::CNAME => "CNAME",
            RecordType::MX => "MX",
            RecordType::TXT => "TXT",
            RecordType::SRV => "SRV",
            RecordType::NS => "NS",
            RecordType::SOA => "SOA",
            RecordType::PTR => "PTR",
            RecordType::CAA => "CAA",
        }
    }

    /// Parse a record type from its string representation.
    ///
    /// Case-insensitive matching.
    pub fn from_str_ignore_case(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "A" => Some(RecordType::A),
            "AAAA" => Some(RecordType::AAAA),
            "CNAME" => Some(RecordType::CNAME),
            "MX" => Some(RecordType::MX),
            "TXT" => Some(RecordType::TXT),
            "SRV" => Some(RecordType::SRV),
            "NS" => Some(RecordType::NS),
            "SOA" => Some(RecordType::SOA),
            "PTR" => Some(RecordType::PTR),
            "CAA" => Some(RecordType::CAA),
            _ => None,
        }
    }

    /// Returns all supported record types.
    pub const fn all() -> &'static [RecordType] {
        &[
            RecordType::A,
            RecordType::AAAA,
            RecordType::CNAME,
            RecordType::MX,
            RecordType::TXT,
            RecordType::SRV,
            RecordType::NS,
            RecordType::SOA,
            RecordType::PTR,
            RecordType::CAA,
        ]
    }
}

impl fmt::Display for RecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_key_suffix())
    }
}

/// Error returned when parsing a record type fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRecordTypeError(String);

impl fmt::Display for ParseRecordTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid record type: {}", self.0)
    }
}

impl std::error::Error for ParseRecordTypeError {}

impl FromStr for RecordType {
    type Err = ParseRecordTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_ignore_case(s).ok_or_else(|| ParseRecordTypeError(s.to_string()))
    }
}

// ============================================================================
// Record Data Types
// ============================================================================

/// MX (Mail Exchange) record with priority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MxRecord {
    /// Priority (lower = preferred).
    pub priority: u16,
    /// Mail server hostname.
    pub exchange: String,
}

impl MxRecord {
    /// Create a new MX record.
    pub fn new(priority: u16, exchange: impl Into<String>) -> Self {
        Self {
            priority,
            exchange: exchange.into(),
        }
    }
}

/// SRV (Service) record for service discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrvRecord {
    /// Priority (lower = preferred).
    pub priority: u16,
    /// Weight for load balancing among same-priority records.
    pub weight: u16,
    /// TCP/UDP port number.
    pub port: u16,
    /// Target hostname.
    pub target: String,
}

impl SrvRecord {
    /// Create a new SRV record.
    pub fn new(priority: u16, weight: u16, port: u16, target: impl Into<String>) -> Self {
        Self {
            priority,
            weight,
            port,
            target: target.into(),
        }
    }
}

/// SOA (Start of Authority) record for zone metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoaRecord {
    /// Primary nameserver for this zone.
    pub mname: String,
    /// Email of zone administrator (in DNS format: hostmaster.example.com).
    pub rname: String,
    /// Zone serial number (typically YYYYMMDDNN).
    pub serial: u32,
    /// Refresh interval for secondary servers (seconds).
    pub refresh: u32,
    /// Retry interval after failed refresh (seconds).
    pub retry: u32,
    /// Expiration time for secondary servers (seconds).
    pub expire: u32,
    /// Minimum TTL (negative caching TTL).
    pub minimum: u32,
}

impl SoaRecord {
    /// Create a new SOA record with sensible defaults.
    pub fn new(mname: impl Into<String>, rname: impl Into<String>) -> Self {
        Self {
            mname: mname.into(),
            rname: rname.into(),
            serial: 1,
            refresh: 3600,   // 1 hour
            retry: 600,      // 10 minutes
            expire: 604_800, // 1 week
            minimum: 3600,   // 1 hour
        }
    }

    /// Create a new SOA record with all fields specified.
    #[allow(clippy::too_many_arguments)]
    pub fn with_all(
        mname: impl Into<String>,
        rname: impl Into<String>,
        serial: u32,
        refresh: u32,
        retry: u32,
        expire: u32,
        minimum: u32,
    ) -> Self {
        Self {
            mname: mname.into(),
            rname: rname.into(),
            serial,
            refresh,
            retry,
            expire,
            minimum,
        }
    }
}

/// CAA (Certification Authority Authorization) record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaaRecord {
    /// Flags (0 = non-critical, 128 = critical).
    pub flags: u8,
    /// Tag type (e.g., "issue", "issuewild", "iodef").
    pub tag: String,
    /// Tag value (e.g., CA domain or URL).
    pub value: String,
}

impl CaaRecord {
    /// Create a new CAA record.
    pub fn new(flags: u8, tag: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            flags,
            tag: tag.into(),
            value: value.into(),
        }
    }

    /// Create a CAA "issue" record (allows a CA to issue certificates).
    pub fn issue(ca_domain: impl Into<String>) -> Self {
        Self::new(0, "issue", ca_domain)
    }

    /// Create a CAA "issuewild" record (allows wildcard certificate issuance).
    pub fn issue_wild(ca_domain: impl Into<String>) -> Self {
        Self::new(0, "issuewild", ca_domain)
    }
}

// ============================================================================
// Record Data Enum
// ============================================================================

/// Type-specific DNS record data.
///
/// Each variant contains the data specific to that record type.
/// Multi-value records (like A with multiple IPs) store all values in a Vec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum DnsRecordData {
    /// IPv4 addresses.
    A {
        /// List of IPv4 addresses (supports round-robin/load balancing).
        addresses: Vec<Ipv4Addr>,
    },
    /// IPv6 addresses.
    AAAA {
        /// List of IPv6 addresses.
        addresses: Vec<Ipv6Addr>,
    },
    /// Canonical name (alias).
    CNAME {
        /// Target domain name.
        target: String,
    },
    /// Mail exchange records.
    MX {
        /// List of MX records (sorted by priority during resolution).
        records: Vec<MxRecord>,
    },
    /// Text records.
    TXT {
        /// List of text strings.
        strings: Vec<String>,
    },
    /// Service records.
    SRV {
        /// List of SRV records.
        records: Vec<SrvRecord>,
    },
    /// Nameserver records.
    NS {
        /// List of nameserver hostnames.
        nameservers: Vec<String>,
    },
    /// Start of authority.
    SOA(SoaRecord),
    /// Pointer record (reverse DNS).
    PTR {
        /// Target domain name.
        target: String,
    },
    /// Certification Authority Authorization.
    CAA {
        /// List of CAA records.
        records: Vec<CaaRecord>,
    },
}

impl DnsRecordData {
    /// Get the record type for this data.
    pub const fn record_type(&self) -> RecordType {
        match self {
            DnsRecordData::A { .. } => RecordType::A,
            DnsRecordData::AAAA { .. } => RecordType::AAAA,
            DnsRecordData::CNAME { .. } => RecordType::CNAME,
            DnsRecordData::MX { .. } => RecordType::MX,
            DnsRecordData::TXT { .. } => RecordType::TXT,
            DnsRecordData::SRV { .. } => RecordType::SRV,
            DnsRecordData::NS { .. } => RecordType::NS,
            DnsRecordData::SOA(_) => RecordType::SOA,
            DnsRecordData::PTR { .. } => RecordType::PTR,
            DnsRecordData::CAA { .. } => RecordType::CAA,
        }
    }

    /// Create an A record with a single IPv4 address.
    pub fn a(addr: Ipv4Addr) -> Self {
        DnsRecordData::A { addresses: vec![addr] }
    }

    /// Create an A record with multiple IPv4 addresses.
    pub fn a_multi(addresses: Vec<Ipv4Addr>) -> Self {
        DnsRecordData::A { addresses }
    }

    /// Create an AAAA record with a single IPv6 address.
    pub fn aaaa(addr: Ipv6Addr) -> Self {
        DnsRecordData::AAAA { addresses: vec![addr] }
    }

    /// Create an AAAA record with multiple IPv6 addresses.
    pub fn aaaa_multi(addresses: Vec<Ipv6Addr>) -> Self {
        DnsRecordData::AAAA { addresses }
    }

    /// Create a CNAME record.
    pub fn cname(target: impl Into<String>) -> Self {
        DnsRecordData::CNAME { target: target.into() }
    }

    /// Create an MX record with a single entry.
    pub fn mx(priority: u16, exchange: impl Into<String>) -> Self {
        DnsRecordData::MX {
            records: vec![MxRecord::new(priority, exchange)],
        }
    }

    /// Create a TXT record with a single string.
    pub fn txt(text: impl Into<String>) -> Self {
        DnsRecordData::TXT {
            strings: vec![text.into()],
        }
    }

    /// Create a TXT record with multiple strings.
    pub fn txt_multi(strings: Vec<String>) -> Self {
        DnsRecordData::TXT { strings }
    }

    /// Create a PTR record.
    pub fn ptr(target: impl Into<String>) -> Self {
        DnsRecordData::PTR { target: target.into() }
    }
}

// ============================================================================
// DNS Record
// ============================================================================

/// A complete DNS record with domain, type, TTL, and data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Fully qualified domain name (e.g., "api.example.com").
    pub domain: String,
    /// Time-to-live in seconds.
    pub ttl_seconds: u32,
    /// Record data (type-specific).
    pub data: DnsRecordData,
    /// Unix timestamp when this record was last updated (milliseconds).
    pub updated_at_ms: u64,
}

impl DnsRecord {
    /// Create a new DNS record with current timestamp.
    pub fn new(domain: impl Into<String>, ttl_seconds: u32, data: DnsRecordData) -> Self {
        Self {
            domain: domain.into(),
            ttl_seconds,
            data,
            updated_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }

    /// Create a new DNS record with explicit timestamp.
    pub fn with_timestamp(
        domain: impl Into<String>,
        ttl_seconds: u32,
        data: DnsRecordData,
        updated_at_ms: u64,
    ) -> Self {
        Self {
            domain: domain.into(),
            ttl_seconds,
            data,
            updated_at_ms,
        }
    }

    /// Get the record type.
    pub const fn record_type(&self) -> RecordType {
        self.data.record_type()
    }

    /// Generate the KV store key for this record.
    ///
    /// Format: `dns:{domain}:{type}`
    pub fn kv_key(&self) -> String {
        format!("{}{}:{}", DNS_KEY_PREFIX, self.domain, self.record_type())
    }

    /// Parse domain and record type from a KV store key.
    ///
    /// Returns `None` if the key doesn't match the expected format.
    pub fn parse_kv_key(key: &str) -> Option<(String, RecordType)> {
        let key = key.strip_prefix(DNS_KEY_PREFIX)?;
        let (domain, type_str) = key.rsplit_once(':')?;
        let record_type = RecordType::from_str_ignore_case(type_str)?;
        Some((domain.to_string(), record_type))
    }

    /// Serialize this record to JSON bytes for storage.
    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize a record from JSON bytes.
    pub fn from_json_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

// ============================================================================
// DNS Events
// ============================================================================

/// Event emitted when DNS records change.
///
/// Used by the client library to notify subscribers of updates.
#[derive(Debug, Clone)]
pub enum DnsEvent {
    /// A new record was added.
    RecordAdded {
        /// The domain that was added.
        domain: String,
        /// The record type.
        record_type: RecordType,
        /// The full record data.
        record: DnsRecord,
    },
    /// An existing record was updated.
    RecordUpdated {
        /// The domain that was updated.
        domain: String,
        /// The record type.
        record_type: RecordType,
        /// The previous record data.
        old: DnsRecord,
        /// The new record data.
        new: DnsRecord,
    },
    /// A record was deleted.
    RecordDeleted {
        /// The domain that was deleted.
        domain: String,
        /// The record type.
        record_type: RecordType,
    },
    /// Sync status changed.
    SyncStatusChanged {
        /// The new sync status.
        status: SyncStatus,
    },
}

/// Current sync status of the DNS client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Not connected to any cluster.
    Disconnected,
    /// Establishing connection.
    Connecting,
    /// Performing initial sync.
    Syncing,
    /// Fully synced and receiving updates.
    Synced,
    /// Synced but data may be stale (no recent updates).
    Stale,
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncStatus::Disconnected => write!(f, "disconnected"),
            SyncStatus::Connecting => write!(f, "connecting"),
            SyncStatus::Syncing => write!(f, "syncing"),
            SyncStatus::Synced => write!(f, "synced"),
            SyncStatus::Stale => write!(f, "stale"),
        }
    }
}

// ============================================================================
// Zone Types
// ============================================================================

/// A DNS zone grouping related records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    /// Zone apex (e.g., "example.com").
    pub name: String,
    /// Whether this zone is enabled.
    pub enabled: bool,
    /// Default TTL for records in this zone.
    pub default_ttl: u32,
    /// Zone metadata.
    pub metadata: ZoneMetadata,
}

/// Metadata for a DNS zone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZoneMetadata {
    /// SOA serial number (auto-incremented on changes).
    pub serial: u32,
    /// Unix timestamp of last modification (milliseconds).
    pub last_modified_ms: u64,
    /// Description of this zone.
    pub description: Option<String>,
}

impl Zone {
    /// Create a new zone with default settings.
    pub fn new(name: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            name: name.into(),
            enabled: true,
            default_ttl: super::constants::DEFAULT_TTL,
            metadata: ZoneMetadata {
                serial: 1,
                last_modified_ms: now,
                description: None,
            },
        }
    }

    /// Set the default TTL for this zone.
    pub fn with_default_ttl(mut self, ttl: u32) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Set the description for this zone.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.metadata.description = Some(description.into());
        self
    }

    /// Disable this zone.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Generate the KV store key for this zone.
    pub fn kv_key(&self) -> String {
        format!("{}{}", super::constants::DNS_ZONE_PREFIX, self.name)
    }

    /// Serialize this zone to JSON bytes for storage.
    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    /// Deserialize a zone from JSON bytes.
    pub fn from_json_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_type_suffix() {
        assert_eq!(RecordType::A.as_key_suffix(), "A");
        assert_eq!(RecordType::AAAA.as_key_suffix(), "AAAA");
        assert_eq!(RecordType::MX.as_key_suffix(), "MX");
        assert_eq!(RecordType::SRV.as_key_suffix(), "SRV");
    }

    #[test]
    fn test_record_type_from_str() {
        assert_eq!(RecordType::from_str_ignore_case("a"), Some(RecordType::A));
        assert_eq!(RecordType::from_str_ignore_case("A"), Some(RecordType::A));
        assert_eq!(RecordType::from_str_ignore_case("aaaa"), Some(RecordType::AAAA));
        assert_eq!(RecordType::from_str_ignore_case("INVALID"), None);
    }

    #[test]
    fn test_dns_record_kv_key() {
        let record = DnsRecord::new("api.example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap()));
        assert_eq!(record.kv_key(), "dns:api.example.com:A");
    }

    #[test]
    fn test_parse_kv_key() {
        let (domain, record_type) = DnsRecord::parse_kv_key("dns:api.example.com:A").unwrap();
        assert_eq!(domain, "api.example.com");
        assert_eq!(record_type, RecordType::A);

        let (domain, record_type) = DnsRecord::parse_kv_key("dns:mail.example.com:MX").unwrap();
        assert_eq!(domain, "mail.example.com");
        assert_eq!(record_type, RecordType::MX);

        assert!(DnsRecord::parse_kv_key("invalid:key").is_none());
        assert!(DnsRecord::parse_kv_key("dns:example.com:INVALID").is_none());
    }

    #[test]
    fn test_dns_record_json_roundtrip() {
        let record = DnsRecord::new(
            "example.com",
            3600,
            DnsRecordData::MX {
                records: vec![
                    MxRecord::new(10, "mail1.example.com"),
                    MxRecord::new(20, "mail2.example.com"),
                ],
            },
        );

        let bytes = record.to_json_bytes();
        let parsed = DnsRecord::from_json_bytes(&bytes).unwrap();

        assert_eq!(record.domain, parsed.domain);
        assert_eq!(record.ttl_seconds, parsed.ttl_seconds);
        assert_eq!(record.data, parsed.data);
    }

    #[test]
    fn test_zone_kv_key() {
        let zone = Zone::new("example.com");
        assert_eq!(zone.kv_key(), "dns:_zone:example.com");
    }

    #[test]
    fn test_zone_json_roundtrip() {
        let zone = Zone::new("example.com").with_default_ttl(7200).with_description("Production zone");

        let bytes = zone.to_json_bytes();
        let parsed = Zone::from_json_bytes(&bytes).unwrap();

        assert_eq!(zone.name, parsed.name);
        assert_eq!(zone.default_ttl, parsed.default_ttl);
        assert_eq!(zone.metadata.description, parsed.metadata.description);
    }

    #[test]
    fn test_record_data_type() {
        assert_eq!(DnsRecordData::a("1.2.3.4".parse().unwrap()).record_type(), RecordType::A);
        assert_eq!(DnsRecordData::cname("example.com").record_type(), RecordType::CNAME);
        assert_eq!(DnsRecordData::txt("test").record_type(), RecordType::TXT);
    }
}
