//! DNS validation functions.
//!
//! Provides RFC-compliant validation for domain names, record data, and TTL values.
//! All validation follows RFC 1035 and subsequent RFCs for specific record types.

use std::net::Ipv4Addr;
use std::net::Ipv6Addr;

use super::constants::DEFAULT_TTL;
use super::constants::LABEL_SEPARATOR;
use super::constants::MAX_CNAME_TARGET_LENGTH;
use super::constants::MAX_DOMAIN_LENGTH;
use super::constants::MAX_LABEL_LENGTH;
use super::constants::MAX_MX_EXCHANGE_LENGTH;
use super::constants::MAX_MX_RECORDS;
use super::constants::MAX_NS_RECORDS;
use super::constants::MAX_SRV_RECORDS;
use super::constants::MAX_SRV_TARGET_LENGTH;
use super::constants::MAX_TTL;
use super::constants::MAX_TXT_LENGTH;
use super::constants::MAX_TXT_STRING_LENGTH;
use super::constants::MAX_TXT_STRINGS;
use super::constants::MIN_LABEL_LENGTH;
use super::constants::MIN_TTL;
use super::constants::WILDCARD_CHAR;
use super::error::DnsError;
use super::error::DnsResult;
use super::types::CaaRecord;
use super::types::DnsRecord;
use super::types::DnsRecordData;
use super::types::MxRecord;
use super::types::RecordType;
use super::types::SoaRecord;
use super::types::SrvRecord;

// ============================================================================
// Domain Name Validation
// ============================================================================

/// Validate a domain name according to RFC 1035.
///
/// Valid domain names:
/// - Maximum 253 characters total
/// - Labels separated by dots
/// - Each label 1-63 characters
/// - Labels start and end with alphanumeric
/// - Labels contain only alphanumeric and hyphens
/// - Supports wildcard: `*.example.com`
///
/// # Examples
///
/// ```
/// use aspen::dns::validation::validate_domain;
///
/// assert!(validate_domain("example.com").is_ok());
/// assert!(validate_domain("*.example.com").is_ok());
/// assert!(validate_domain("sub-domain.example.com").is_ok());
/// assert!(validate_domain("-invalid.com").is_err());
/// assert!(validate_domain("too..many.dots").is_err());
/// ```
pub fn validate_domain(domain: &str) -> DnsResult<()> {
    // Check overall length
    if domain.is_empty() {
        return Err(DnsError::InvalidDomain {
            reason: "domain name cannot be empty".to_string(),
        });
    }

    if domain.len() > MAX_DOMAIN_LENGTH {
        return Err(DnsError::InvalidDomain {
            reason: format!("domain name exceeds maximum length of {} characters", MAX_DOMAIN_LENGTH),
        });
    }

    // Split into labels
    let labels: Vec<&str> = domain.split(LABEL_SEPARATOR).collect();

    if labels.is_empty() {
        return Err(DnsError::InvalidDomain {
            reason: "domain name has no labels".to_string(),
        });
    }

    for (i, label) in labels.iter().enumerate() {
        validate_label(label, i == 0)?;
    }

    Ok(())
}

/// Validate a single domain label.
fn validate_label(label: &str, allow_wildcard: bool) -> DnsResult<()> {
    // Check for empty label (consecutive dots)
    if label.is_empty() {
        return Err(DnsError::InvalidDomain {
            reason: "empty label (consecutive dots)".to_string(),
        });
    }

    // Handle wildcard label
    if label == "*" {
        if allow_wildcard {
            return Ok(());
        } else {
            return Err(DnsError::InvalidDomain {
                reason: "wildcard (*) only allowed as first label".to_string(),
            });
        }
    }

    // Check label length
    if label.len() < MIN_LABEL_LENGTH {
        return Err(DnsError::InvalidDomain {
            reason: format!("label '{}' is too short", label),
        });
    }

    if label.len() > MAX_LABEL_LENGTH {
        return Err(DnsError::InvalidDomain {
            reason: format!("label '{}' exceeds maximum length of {} characters", label, MAX_LABEL_LENGTH),
        });
    }

    // Check first character (must be alphanumeric or wildcard)
    let first = label.chars().next().unwrap();
    if !first.is_ascii_alphanumeric() {
        // Allow underscore for SRV records (_tcp, _udp)
        if first != '_' {
            return Err(DnsError::InvalidDomain {
                reason: format!("label '{}' must start with alphanumeric character", label),
            });
        }
    }

    // Check last character (must be alphanumeric)
    let last = label.chars().last().unwrap();
    if !last.is_ascii_alphanumeric() {
        return Err(DnsError::InvalidDomain {
            reason: format!("label '{}' must end with alphanumeric character", label),
        });
    }

    // Check all characters (must be alphanumeric, hyphen, or underscore for SRV)
    for c in label.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
            return Err(DnsError::InvalidDomain {
                reason: format!("label '{}' contains invalid character '{}'", label, c),
            });
        }
    }

    Ok(())
}

/// Check if a domain is a wildcard domain.
pub fn is_wildcard_domain(domain: &str) -> bool {
    domain.starts_with(WILDCARD_CHAR)
}

/// Get the parent domain for wildcard matching.
///
/// For `foo.example.com`, returns `*.example.com` for wildcard lookup.
pub fn wildcard_parent(domain: &str) -> Option<String> {
    let parts: Vec<&str> = domain.split(LABEL_SEPARATOR).collect();
    if parts.len() <= 1 {
        return None;
    }
    Some(format!("*.{}", parts[1..].join(".")))
}

// ============================================================================
// TTL Validation
// ============================================================================

/// Validate a TTL value.
///
/// TTL must be between MIN_TTL and MAX_TTL.
pub fn validate_ttl(ttl: u32) -> DnsResult<u32> {
    if ttl < MIN_TTL {
        return Err(DnsError::InvalidTtl {
            ttl,
            min: MIN_TTL,
            max: MAX_TTL,
        });
    }

    if ttl > MAX_TTL {
        return Err(DnsError::InvalidTtl {
            ttl,
            min: MIN_TTL,
            max: MAX_TTL,
        });
    }

    Ok(ttl)
}

/// Normalize a TTL value, clamping to allowed range.
pub fn normalize_ttl(ttl: u32) -> u32 {
    ttl.clamp(MIN_TTL, MAX_TTL)
}

/// Get the default TTL.
pub const fn default_ttl() -> u32 {
    DEFAULT_TTL
}

// ============================================================================
// Record Data Validation
// ============================================================================

/// Validate a complete DNS record.
pub fn validate_record(record: &DnsRecord) -> DnsResult<()> {
    // Validate domain
    validate_domain(&record.domain)?;

    // Validate TTL
    validate_ttl(record.ttl_seconds)?;

    // Validate record-specific data
    validate_record_data(&record.data)?;

    Ok(())
}

/// Validate record-specific data.
pub fn validate_record_data(data: &DnsRecordData) -> DnsResult<()> {
    match data {
        DnsRecordData::A { addresses } => validate_a_record(addresses),
        DnsRecordData::AAAA { addresses } => validate_aaaa_record(addresses),
        DnsRecordData::CNAME { target } => validate_cname_record(target),
        DnsRecordData::MX { records } => validate_mx_records(records),
        DnsRecordData::TXT { strings } => validate_txt_record(strings),
        DnsRecordData::SRV { records } => validate_srv_records(records),
        DnsRecordData::NS { nameservers } => validate_ns_records(nameservers),
        DnsRecordData::SOA(soa) => validate_soa_record(soa),
        DnsRecordData::PTR { target } => validate_ptr_record(target),
        DnsRecordData::CAA { records } => validate_caa_records(records),
    }
}

/// Validate A record addresses.
fn validate_a_record(addresses: &[Ipv4Addr]) -> DnsResult<()> {
    if addresses.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::A,
            reason: "A record must have at least one address".to_string(),
        });
    }

    // Check for duplicate addresses
    let mut seen = std::collections::HashSet::new();
    for addr in addresses {
        if !seen.insert(*addr) {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::A,
                reason: format!("duplicate address: {}", addr),
            });
        }
    }

    Ok(())
}

/// Validate AAAA record addresses.
fn validate_aaaa_record(addresses: &[Ipv6Addr]) -> DnsResult<()> {
    if addresses.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::AAAA,
            reason: "AAAA record must have at least one address".to_string(),
        });
    }

    // Check for duplicate addresses
    let mut seen = std::collections::HashSet::new();
    for addr in addresses {
        if !seen.insert(*addr) {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::AAAA,
                reason: format!("duplicate address: {}", addr),
            });
        }
    }

    Ok(())
}

/// Validate CNAME record target.
fn validate_cname_record(target: &str) -> DnsResult<()> {
    if target.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::CNAME,
            reason: "CNAME target cannot be empty".to_string(),
        });
    }

    if target.len() > MAX_CNAME_TARGET_LENGTH {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::CNAME,
            reason: format!("CNAME target exceeds maximum length of {} characters", MAX_CNAME_TARGET_LENGTH),
        });
    }

    validate_domain(target).map_err(|e| DnsError::InvalidRecordData {
        record_type: RecordType::CNAME,
        reason: format!("invalid target domain: {}", e),
    })?;

    Ok(())
}

/// Validate MX records.
fn validate_mx_records(records: &[MxRecord]) -> DnsResult<()> {
    if records.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::MX,
            reason: "MX record must have at least one exchange".to_string(),
        });
    }

    if records.len() > MAX_MX_RECORDS as usize {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::MX,
            reason: format!("too many MX records (max {})", MAX_MX_RECORDS),
        });
    }

    for mx in records {
        if mx.exchange.is_empty() {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::MX,
                reason: "MX exchange cannot be empty".to_string(),
            });
        }

        if mx.exchange.len() > MAX_MX_EXCHANGE_LENGTH {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::MX,
                reason: format!("MX exchange exceeds maximum length of {} characters", MAX_MX_EXCHANGE_LENGTH),
            });
        }

        validate_domain(&mx.exchange).map_err(|e| DnsError::InvalidRecordData {
            record_type: RecordType::MX,
            reason: format!("invalid exchange domain: {}", e),
        })?;
    }

    Ok(())
}

/// Validate TXT record strings.
fn validate_txt_record(strings: &[String]) -> DnsResult<()> {
    if strings.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::TXT,
            reason: "TXT record must have at least one string".to_string(),
        });
    }

    if strings.len() > MAX_TXT_STRINGS as usize {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::TXT,
            reason: format!("too many TXT strings (max {})", MAX_TXT_STRINGS),
        });
    }

    let mut total_len = 0;
    for s in strings {
        if s.len() > MAX_TXT_STRING_LENGTH {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::TXT,
                reason: format!("TXT string exceeds maximum length of {} characters", MAX_TXT_STRING_LENGTH),
            });
        }
        total_len += s.len();
    }

    if total_len > MAX_TXT_LENGTH {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::TXT,
            reason: format!("total TXT data exceeds maximum length of {} bytes", MAX_TXT_LENGTH),
        });
    }

    Ok(())
}

/// Validate SRV records.
fn validate_srv_records(records: &[SrvRecord]) -> DnsResult<()> {
    if records.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::SRV,
            reason: "SRV record must have at least one entry".to_string(),
        });
    }

    if records.len() > MAX_SRV_RECORDS as usize {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::SRV,
            reason: format!("too many SRV records (max {})", MAX_SRV_RECORDS),
        });
    }

    for srv in records {
        // Port 0 is valid (means "no service available")

        if srv.target.is_empty() {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::SRV,
                reason: "SRV target cannot be empty".to_string(),
            });
        }

        if srv.target.len() > MAX_SRV_TARGET_LENGTH {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::SRV,
                reason: format!("SRV target exceeds maximum length of {} characters", MAX_SRV_TARGET_LENGTH),
            });
        }

        // Target "." means no service
        if srv.target != "." {
            validate_domain(&srv.target).map_err(|e| DnsError::InvalidRecordData {
                record_type: RecordType::SRV,
                reason: format!("invalid target domain: {}", e),
            })?;
        }
    }

    Ok(())
}

/// Validate NS records.
fn validate_ns_records(nameservers: &[String]) -> DnsResult<()> {
    if nameservers.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::NS,
            reason: "NS record must have at least one nameserver".to_string(),
        });
    }

    if nameservers.len() > MAX_NS_RECORDS as usize {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::NS,
            reason: format!("too many NS records (max {})", MAX_NS_RECORDS),
        });
    }

    for ns in nameservers {
        if ns.is_empty() {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::NS,
                reason: "nameserver cannot be empty".to_string(),
            });
        }

        validate_domain(ns).map_err(|e| DnsError::InvalidRecordData {
            record_type: RecordType::NS,
            reason: format!("invalid nameserver domain: {}", e),
        })?;
    }

    Ok(())
}

/// Validate SOA record.
fn validate_soa_record(soa: &SoaRecord) -> DnsResult<()> {
    // Validate mname (primary nameserver)
    if soa.mname.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::SOA,
            reason: "SOA mname (primary nameserver) cannot be empty".to_string(),
        });
    }
    validate_domain(&soa.mname).map_err(|e| DnsError::InvalidRecordData {
        record_type: RecordType::SOA,
        reason: format!("invalid mname domain: {}", e),
    })?;

    // Validate rname (responsible person email as domain)
    if soa.rname.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::SOA,
            reason: "SOA rname (contact email) cannot be empty".to_string(),
        });
    }
    validate_domain(&soa.rname).map_err(|e| DnsError::InvalidRecordData {
        record_type: RecordType::SOA,
        reason: format!("invalid rname domain: {}", e),
    })?;

    // Validate timing values (must be reasonable)
    if soa.refresh == 0 {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::SOA,
            reason: "SOA refresh interval cannot be zero".to_string(),
        });
    }

    if soa.retry == 0 {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::SOA,
            reason: "SOA retry interval cannot be zero".to_string(),
        });
    }

    if soa.expire <= soa.refresh {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::SOA,
            reason: "SOA expire must be greater than refresh".to_string(),
        });
    }

    Ok(())
}

/// Validate PTR record target.
fn validate_ptr_record(target: &str) -> DnsResult<()> {
    if target.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::PTR,
            reason: "PTR target cannot be empty".to_string(),
        });
    }

    validate_domain(target).map_err(|e| DnsError::InvalidRecordData {
        record_type: RecordType::PTR,
        reason: format!("invalid target domain: {}", e),
    })?;

    Ok(())
}

/// Validate CAA records.
fn validate_caa_records(records: &[CaaRecord]) -> DnsResult<()> {
    if records.is_empty() {
        return Err(DnsError::InvalidRecordData {
            record_type: RecordType::CAA,
            reason: "CAA record must have at least one entry".to_string(),
        });
    }

    for caa in records {
        // Validate flags (only 0 and 128 are defined)
        if caa.flags != 0 && caa.flags != 128 {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::CAA,
                reason: format!("invalid CAA flags value: {}", caa.flags),
            });
        }

        // Validate tag
        if caa.tag.is_empty() {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::CAA,
                reason: "CAA tag cannot be empty".to_string(),
            });
        }

        // Known tags: issue, issuewild, iodef
        let valid_tags = ["issue", "issuewild", "iodef"];
        if !valid_tags.contains(&caa.tag.as_str()) {
            // Allow unknown tags but log warning
            // Per RFC 8659, unknown tags should be accepted
        }

        // Validate value is not empty for issue/issuewild
        if (caa.tag == "issue" || caa.tag == "issuewild") && caa.value.is_empty() {
            return Err(DnsError::InvalidRecordData {
                record_type: RecordType::CAA,
                reason: format!("CAA {} value cannot be empty", caa.tag),
            });
        }
    }

    Ok(())
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Parse an IPv4 address from a string.
pub fn parse_ipv4(s: &str) -> DnsResult<Ipv4Addr> {
    s.parse().map_err(|_| DnsError::InvalidRecordData {
        record_type: RecordType::A,
        reason: format!("invalid IPv4 address: {}", s),
    })
}

/// Parse an IPv6 address from a string.
pub fn parse_ipv6(s: &str) -> DnsResult<Ipv6Addr> {
    s.parse().map_err(|_| DnsError::InvalidRecordData {
        record_type: RecordType::AAAA,
        reason: format!("invalid IPv6 address: {}", s),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain_valid() {
        assert!(validate_domain("example.com").is_ok());
        assert!(validate_domain("sub.example.com").is_ok());
        assert!(validate_domain("sub-domain.example.com").is_ok());
        assert!(validate_domain("a.b.c.d.example.com").is_ok());
        assert!(validate_domain("*.example.com").is_ok());
        assert!(validate_domain("_dmarc.example.com").is_ok());
        assert!(validate_domain("_tcp.example.com").is_ok());
    }

    #[test]
    fn test_validate_domain_invalid() {
        assert!(validate_domain("").is_err());
        assert!(validate_domain("-invalid.com").is_err());
        assert!(validate_domain("invalid-.com").is_err());
        assert!(validate_domain("too..many.dots").is_err());
        assert!(validate_domain("*.*.example.com").is_err()); // Wildcard not in first position
        assert!(validate_domain(&"a".repeat(64)).is_err()); // Label too long
        assert!(validate_domain(&format!("{}.com", "a".repeat(250))).is_err()); // Domain too long
    }

    #[test]
    fn test_is_wildcard_domain() {
        assert!(is_wildcard_domain("*.example.com"));
        assert!(!is_wildcard_domain("www.example.com"));
        assert!(!is_wildcard_domain("example.com"));
    }

    #[test]
    fn test_wildcard_parent() {
        assert_eq!(wildcard_parent("foo.example.com"), Some("*.example.com".to_string()));
        assert_eq!(wildcard_parent("bar.foo.example.com"), Some("*.foo.example.com".to_string()));
        assert_eq!(wildcard_parent("example.com"), Some("*.com".to_string()));
        assert_eq!(wildcard_parent("com"), None);
    }

    #[test]
    fn test_validate_ttl() {
        assert!(validate_ttl(MIN_TTL).is_ok());
        assert!(validate_ttl(DEFAULT_TTL).is_ok());
        assert!(validate_ttl(MAX_TTL).is_ok());
        assert!(validate_ttl(MIN_TTL - 1).is_err());
        assert!(validate_ttl(MAX_TTL + 1).is_err());
    }

    #[test]
    fn test_normalize_ttl() {
        assert_eq!(normalize_ttl(0), MIN_TTL);
        assert_eq!(normalize_ttl(MIN_TTL), MIN_TTL);
        assert_eq!(normalize_ttl(DEFAULT_TTL), DEFAULT_TTL);
        assert_eq!(normalize_ttl(MAX_TTL), MAX_TTL);
        assert_eq!(normalize_ttl(MAX_TTL + 1000), MAX_TTL);
    }

    #[test]
    fn test_validate_a_record() {
        let valid = vec!["192.168.1.1".parse().unwrap()];
        assert!(validate_a_record(&valid).is_ok());

        let empty: Vec<Ipv4Addr> = vec![];
        assert!(validate_a_record(&empty).is_err());

        let duplicate = vec!["192.168.1.1".parse().unwrap(), "192.168.1.1".parse().unwrap()];
        assert!(validate_a_record(&duplicate).is_err());
    }

    #[test]
    fn test_validate_mx_records() {
        let valid = vec![MxRecord::new(10, "mail.example.com")];
        assert!(validate_mx_records(&valid).is_ok());

        let empty: Vec<MxRecord> = vec![];
        assert!(validate_mx_records(&empty).is_err());

        let invalid_exchange = vec![MxRecord::new(10, "-invalid")];
        assert!(validate_mx_records(&invalid_exchange).is_err());
    }

    #[test]
    fn test_validate_txt_record() {
        let valid = vec!["v=spf1 include:example.com ~all".to_string()];
        assert!(validate_txt_record(&valid).is_ok());

        let empty: Vec<String> = vec![];
        assert!(validate_txt_record(&empty).is_err());

        let too_long = vec!["a".repeat(MAX_TXT_STRING_LENGTH + 1)];
        assert!(validate_txt_record(&too_long).is_err());
    }

    #[test]
    fn test_validate_srv_records() {
        let valid = vec![SrvRecord::new(10, 20, 443, "server.example.com")];
        assert!(validate_srv_records(&valid).is_ok());

        // Port 0 with "." target means "no service"
        let no_service = vec![SrvRecord::new(10, 0, 0, ".")];
        assert!(validate_srv_records(&no_service).is_ok());

        let empty: Vec<SrvRecord> = vec![];
        assert!(validate_srv_records(&empty).is_err());
    }

    #[test]
    fn test_validate_soa_record() {
        let valid = SoaRecord::new("ns1.example.com", "hostmaster.example.com");
        assert!(validate_soa_record(&valid).is_ok());

        let invalid = SoaRecord::with_all(
            "ns1.example.com",
            "hostmaster.example.com",
            1,
            3600,
            600,
            1000, // expire <= refresh (invalid)
            3600,
        );
        assert!(validate_soa_record(&invalid).is_err());
    }

    #[test]
    fn test_validate_caa_records() {
        let valid = vec![CaaRecord::issue("letsencrypt.org")];
        assert!(validate_caa_records(&valid).is_ok());

        let empty: Vec<CaaRecord> = vec![];
        assert!(validate_caa_records(&empty).is_err());

        let invalid_flags = vec![CaaRecord::new(64, "issue", "ca.example.com")];
        assert!(validate_caa_records(&invalid_flags).is_err());
    }

    #[test]
    fn test_parse_ipv4() {
        assert!(parse_ipv4("192.168.1.1").is_ok());
        assert!(parse_ipv4("0.0.0.0").is_ok());
        assert!(parse_ipv4("255.255.255.255").is_ok());
        assert!(parse_ipv4("invalid").is_err());
        assert!(parse_ipv4("256.0.0.0").is_err());
    }

    #[test]
    fn test_parse_ipv6() {
        assert!(parse_ipv6("::1").is_ok());
        assert!(parse_ipv6("2001:db8::1").is_ok());
        assert!(parse_ipv6("fe80::1%eth0").is_err()); // Scope ID not supported
        assert!(parse_ipv6("invalid").is_err());
    }
}
