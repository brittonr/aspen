//! Constants for DNS module.
//!
//! Tiger Style: All constants are explicitly typed with fixed limits
//! to prevent unbounded resource allocation.

use std::time::Duration;

// ============================================================================
// DNS Key Convention Constants
// ============================================================================

/// Key prefix for all DNS records in the KV store.
///
/// All DNS records are stored with keys in the format:
/// `{DNS_KEY_PREFIX}{domain}:{record_type}`
///
/// Examples:
/// - `dns:example.com:A` - A records for example.com
/// - `dns:mail.example.com:MX` - MX records for mail.example.com
/// - `dns:*.example.com:A` - Wildcard A records
pub const DNS_KEY_PREFIX: &str = "dns:";

/// Key prefix for DNS zone metadata.
///
/// Zone metadata is stored with keys in the format:
/// `{DNS_ZONE_PREFIX}{zone_name}`
pub const DNS_ZONE_PREFIX: &str = "dns:_zone:";

// ============================================================================
// RFC 1035 Domain Name Limits
// ============================================================================

/// Maximum domain name length (RFC 1035 Section 2.3.4).
///
/// A domain name is represented as a sequence of labels, where each label
/// consists of a length octet followed by that number of octets. The domain
/// name terminates with the zero length octet for the null label of the root.
pub const MAX_DOMAIN_LENGTH: usize = 253;

/// Maximum label length within a domain name (RFC 1035 Section 2.3.4).
///
/// Each label is limited to 63 octets.
pub const MAX_LABEL_LENGTH: usize = 63;

/// Minimum label length (labels must have at least one character).
pub const MIN_LABEL_LENGTH: usize = 1;

// ============================================================================
// DNS Record Limits
// ============================================================================

/// Maximum number of records per domain/type combination.
///
/// Tiger Style: Bounded to prevent unbounded growth of multi-value records
/// (e.g., multiple A records for load balancing).
pub const MAX_RECORDS_PER_DOMAIN: u32 = 100;

/// Maximum number of zones per cluster.
///
/// Tiger Style: Bounded to prevent unbounded zone creation.
pub const MAX_ZONES: u32 = 10_000;

/// Maximum records in a batch operation.
///
/// Tiger Style: Bounded to prevent memory exhaustion during bulk operations.
pub const MAX_BATCH_SIZE: u32 = 100;

/// Maximum TXT record total length (RFC 1035).
///
/// TXT records can contain multiple strings, each up to 255 bytes.
/// Total TXT-RDATA is limited to 65535 bytes.
pub const MAX_TXT_LENGTH: usize = 65535;

/// Maximum individual TXT string length (RFC 1035).
pub const MAX_TXT_STRING_LENGTH: usize = 255;

/// Maximum number of TXT strings per record.
///
/// Tiger Style: Bounded to prevent pathological cases.
pub const MAX_TXT_STRINGS: u32 = 256;

/// Maximum length of an MX exchange domain.
pub const MAX_MX_EXCHANGE_LENGTH: usize = MAX_DOMAIN_LENGTH;

/// Maximum length of an SRV target domain.
pub const MAX_SRV_TARGET_LENGTH: usize = MAX_DOMAIN_LENGTH;

/// Maximum length of a CNAME target domain.
pub const MAX_CNAME_TARGET_LENGTH: usize = MAX_DOMAIN_LENGTH;

/// Maximum number of MX records per domain.
///
/// Tiger Style: Practical limit for mail exchangers.
pub const MAX_MX_RECORDS: u32 = 32;

/// Maximum number of SRV records per domain.
///
/// Tiger Style: Practical limit for service records.
pub const MAX_SRV_RECORDS: u32 = 64;

/// Maximum number of NS records per zone.
///
/// Tiger Style: Practical limit for nameservers.
pub const MAX_NS_RECORDS: u32 = 16;

// ============================================================================
// TTL Constants
// ============================================================================

/// Default TTL for DNS records (1 hour = 3600 seconds).
///
/// Used when no TTL is specified during record creation.
pub const DEFAULT_TTL: u32 = 3600;

/// Minimum TTL allowed (1 minute = 60 seconds).
///
/// Tiger Style: Floor prevents excessively aggressive caching behavior
/// that could overwhelm the cluster with refresh requests.
pub const MIN_TTL: u32 = 60;

/// Maximum TTL allowed (1 week = 604800 seconds).
///
/// Tiger Style: Ceiling prevents indefinitely cached records that
/// would be difficult to update in practice.
pub const MAX_TTL: u32 = 604_800;

// ============================================================================
// Client Cache Constants
// ============================================================================

/// Maximum number of cached DNS records in the client.
///
/// Tiger Style: Bounded to prevent unbounded memory growth.
/// 100,000 records with ~1KB average size = ~100MB max cache.
pub const MAX_CACHED_RECORDS: usize = 100_000;

/// Maximum number of concurrent event subscribers.
///
/// Tiger Style: Bounded to prevent resource exhaustion.
pub const MAX_SUBSCRIBERS: usize = 100;

/// Timeout for sync operations.
///
/// Tiger Style: Explicit timeout prevents indefinite hangs.
pub const SYNC_TIMEOUT: Duration = Duration::from_secs(30);

/// Timeout for connection establishment.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Interval for background sync (drift correction).
pub const BACKGROUND_SYNC_INTERVAL: Duration = Duration::from_secs(60);

/// Staleness threshold - if no updates for this duration, status becomes Stale.
pub const STALENESS_THRESHOLD: Duration = Duration::from_secs(300);

/// Maximum reconnection attempts before giving up.
pub const MAX_RECONNECT_ATTEMPTS: u32 = 10;

/// Initial reconnection delay.
pub const RECONNECT_INITIAL_DELAY: Duration = Duration::from_secs(1);

/// Maximum reconnection delay (exponential backoff cap).
pub const RECONNECT_MAX_DELAY: Duration = Duration::from_secs(60);

// ============================================================================
// Ticket Constants
// ============================================================================

/// Prefix for DNS client tickets.
///
/// Tickets serialize to `aspendns{base64-encoded-data}` format.
pub const DNS_TICKET_PREFIX: &str = "aspendns";

// ============================================================================
// Validation Constants
// ============================================================================

/// Valid characters for domain name labels.
///
/// Per RFC 1035, labels can contain letters, digits, and hyphens.
/// The first and last characters must be alphanumeric.
pub const LABEL_ALLOWED_CHARS: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-";

/// Wildcard character for wildcard DNS records.
pub const WILDCARD_CHAR: char = '*';

/// Label separator in domain names.
pub const LABEL_SEPARATOR: char = '.';
