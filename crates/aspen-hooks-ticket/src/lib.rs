//! Hook trigger tickets for external programs.
//!
//! Provides a compact, URL-safe way to share hook trigger credentials with external
//! programs. The shared ticket payload stays alloc-safe by default; runtime crates
//! convert bootstrap peers to concrete iroh addresses at the shell boundary.
//!
//! # Ticket Format
//!
//! Hook tickets are serialized as base32-encoded strings with an `aspenhook` prefix:
//!
//! ```text
//! aspenhook7g2wc...agd6q
//! ```
//!
//! # Example
//!
//! ```ignore
//! use aspen_cluster_types::{NodeAddress, NodeTransportAddr};
//! use aspen_hooks_ticket::AspenHookTicket;
//! use core::net::SocketAddr;
//!
//! let ticket = AspenHookTicket::new(
//!     "my-cluster",
//!     vec![NodeAddress::from_parts(
//!         "3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
//!         [NodeTransportAddr::Ip(SocketAddr::from(([127, 0, 0, 1], 7777)))],
//!     )],
//! )
//! .with_event_type("write_committed")
//! .with_default_payload(r#"{"source": "external"}"#)
//! .with_expiry_from_now(1_000, 24);
//!
//! let encoded = ticket.serialize();
//! let parsed = AspenHookTicket::deserialize_at(&encoded, 1_001)?;
//! assert_eq!(parsed.cluster_id, "my-cluster");
//! # Ok::<(), aspen_hooks_ticket::HookTicketError>(())
//! ```
//!
//! # Security
//!
//! - **Unauthenticated tickets**: Can be rate-limited but not revoked
//! - **Token-authenticated tickets**: Require HMAC capability token
//! - **Expiration**: Tickets can have time-limited validity
//!
//! # Tiger Style
//!
//! All fields are bounded to prevent unbounded resource consumption:
//! - `MAX_BOOTSTRAP_PEERS`: 16
//! - `MAX_CLUSTER_ID_SIZE`: 128 bytes
//! - `MAX_EVENT_TYPE_SIZE`: 64 bytes
//! - `MAX_PAYLOAD_SIZE`: 4096 bytes

#![cfg_attr(not(any(test, feature = "std")), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::fmt;
use core::result::Result as CoreResult;
#[cfg(feature = "std")]
use std::time::SystemTime;
#[cfg(feature = "std")]
use std::time::UNIX_EPOCH;

use aspen_cluster_types::NodeAddress;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;
use thiserror::Error;

/// Result alias for hook ticket operations.
pub type HookTicketResult<T> = CoreResult<T, HookTicketError>;

/// Maximum number of bootstrap peers in a ticket.
pub const MAX_BOOTSTRAP_PEERS: usize = 16;

/// Maximum length of the cluster ID.
pub const MAX_CLUSTER_ID_SIZE: usize = 128;

/// Maximum length of the event type.
pub const MAX_EVENT_TYPE_SIZE: usize = 64;

/// Maximum length of the default payload.
pub const MAX_PAYLOAD_SIZE: usize = 4096;

/// Maximum length of the relay URL.
pub const MAX_RELAY_URL_SIZE: usize = 256;

/// Ticket prefix for serialization.
pub const HOOK_TICKET_PREFIX: &str = "aspenhook";

/// Default ticket validity duration in hours.
pub const DEFAULT_EXPIRY_HOURS: u64 = 24;

/// Number of seconds in one minute.
const SECONDS_PER_MINUTE: u64 = 60;

/// Number of seconds in one hour.
const SECONDS_PER_HOUR: u64 = 3_600;

/// Number of seconds in one day.
const SECONDS_PER_DAY: u64 = 86_400;

/// Current ticket protocol version.
const TICKET_VERSION: u8 = 1;

/// Errors returned by shared hook ticket parsing and validation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum HookTicketError {
    /// Ticket bytes or ticket string could not be decoded.
    #[error("failed to deserialize Aspen hook ticket: {reason}")]
    Deserialize {
        /// Human-readable decode failure.
        reason: String,
    },

    /// Cluster ID is required.
    #[error("cluster_id cannot be empty")]
    EmptyClusterId,

    /// Cluster ID exceeded the maximum bound.
    #[error("cluster_id too long: {actual_bytes} bytes (max {max_bytes})")]
    ClusterIdTooLong {
        /// Actual byte length.
        actual_bytes: u32,
        /// Maximum allowed bytes.
        max_bytes: u32,
    },

    /// At least one bootstrap peer is required.
    #[error("at least one bootstrap peer is required")]
    NoBootstrapPeers,

    /// Bootstrap peer count exceeded the bound.
    #[error("too many bootstrap peers: {actual_peers} (max {max_peers})")]
    TooManyBootstrapPeers {
        /// Actual peer count.
        actual_peers: u32,
        /// Maximum allowed peers.
        max_peers: u32,
    },

    /// Event type is required.
    #[error("event_type cannot be empty")]
    EmptyEventType,

    /// Event type exceeded the maximum bound.
    #[error("event_type too long: {actual_bytes} bytes (max {max_bytes})")]
    EventTypeTooLong {
        /// Actual byte length.
        actual_bytes: u32,
        /// Maximum allowed bytes.
        max_bytes: u32,
    },

    /// Default payload exceeded the maximum bound.
    #[error("default_payload too long: {actual_bytes} bytes (max {max_bytes})")]
    DefaultPayloadTooLong {
        /// Actual byte length.
        actual_bytes: u32,
        /// Maximum allowed bytes.
        max_bytes: u32,
    },

    /// Default payload was not valid JSON.
    #[error("default_payload is not valid JSON: {reason}")]
    InvalidDefaultPayloadJson {
        /// Underlying JSON parse failure.
        reason: String,
    },

    /// Relay URL exceeded the maximum bound.
    #[error("relay_url too long: {actual_bytes} bytes (max {max_bytes})")]
    RelayUrlTooLong {
        /// Actual byte length.
        actual_bytes: u32,
        /// Maximum allowed bytes.
        max_bytes: u32,
    },

    /// Ticket version is newer than this crate understands.
    #[error("unsupported ticket version {version} (max supported: {max_supported})")]
    UnsupportedVersion {
        /// Encountered version.
        version: u8,
        /// Maximum supported version.
        max_supported: u8,
    },

    /// Ticket is expired at the supplied validation time.
    #[error("hook ticket has expired")]
    ExpiredTicket {
        /// Stored expiry timestamp.
        expires_at_secs: u64,
        /// Validation time.
        now_secs: u64,
    },
}

/// Aspen hook trigger ticket for external programs.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspenHookTicket {
    /// Protocol version for forward compatibility.
    pub version: u8,

    /// Cluster identifier (human-readable).
    pub cluster_id: String,

    /// Bootstrap peer addresses for initial connection.
    pub bootstrap_peers: Vec<NodeAddress>,

    /// Event type to trigger (e.g. `write_committed`).
    pub event_type: String,

    /// Default payload template (JSON string).
    pub default_payload: Option<String>,

    /// Optional authentication token for authorized triggers.
    pub auth_token: Option<[u8; 32]>,

    /// Unix timestamp when this ticket expires (`0` = no expiry).
    pub expires_at_secs: u64,

    /// Optional relay URL for NAT traversal.
    pub relay_url: Option<String>,

    /// Priority hint for connection ordering (`0` = highest).
    pub priority: u8,
}

impl fmt::Debug for AspenHookTicket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AspenHookTicket")
            .field("version", &self.version)
            .field("cluster_id", &self.cluster_id)
            .field("bootstrap_peer_count", &self.bootstrap_peers.len())
            .field("event_type", &self.event_type)
            .field("has_default_payload", &self.default_payload.is_some())
            .field("has_auth_token", &self.auth_token.is_some())
            .field("expires_at_secs", &self.expires_at_secs)
            .field("has_relay_url", &self.relay_url.is_some())
            .field("priority", &self.priority)
            .finish()
    }
}

impl AspenHookTicket {
    /// Create a new hook ticket.
    pub fn new(cluster_id: impl Into<String>, bootstrap_peers: Vec<NodeAddress>) -> Self {
        Self {
            version: TICKET_VERSION,
            cluster_id: cluster_id.into(),
            bootstrap_peers,
            event_type: String::new(),
            default_payload: None,
            auth_token: None,
            expires_at_secs: 0,
            relay_url: None,
            priority: 0,
        }
    }

    /// Set the event type to trigger.
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = event_type.into();
        self
    }

    /// Set the default payload template.
    pub fn with_default_payload(mut self, payload: impl Into<String>) -> Self {
        self.default_payload = Some(payload.into());
        self
    }

    /// Set the authentication token for authorized triggers.
    pub fn with_auth_token(mut self, token: [u8; 32]) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Set the expiration time directly.
    pub fn with_expiry(mut self, expires_at_secs: u64) -> Self {
        self.expires_at_secs = expires_at_secs;
        self
    }

    /// Set expiration relative to an explicit current time.
    pub fn with_expiry_from_now(mut self, now_secs: u64, hours: u64) -> Self {
        self.expires_at_secs = now_secs.saturating_add(hours.saturating_mul(SECONDS_PER_HOUR));
        self
    }

    /// Set expiration relative to the current wall-clock time.
    #[cfg(feature = "std")]
    pub fn with_expiry_hours(self, hours: u64) -> Self {
        self.with_expiry_from_now(unix_epoch_secs(), hours)
    }

    /// Set the relay URL for NAT traversal.
    pub fn with_relay_url(mut self, url: impl Into<String>) -> Self {
        self.relay_url = Some(url.into());
        self
    }

    /// Set the priority hint for connection ordering.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Add a bootstrap peer to the ticket.
    pub fn add_bootstrap_peer(&mut self, peer: NodeAddress) -> HookTicketResult<()> {
        if self.bootstrap_peers.len() >= MAX_BOOTSTRAP_PEERS {
            return Err(HookTicketError::TooManyBootstrapPeers {
                actual_peers: saturating_usize_to_u32(self.bootstrap_peers.len().saturating_add(1)),
                max_peers: saturating_usize_to_u32(MAX_BOOTSTRAP_PEERS),
            });
        }
        self.bootstrap_peers.push(peer);
        Ok(())
    }

    /// Returns true when the ticket requires authentication.
    pub fn requires_auth(&self) -> bool {
        self.auth_token.is_some()
    }

    /// Returns true when the ticket is expired at `now_secs`.
    pub fn is_expired_at(&self, now_secs: u64) -> bool {
        self.expires_at_secs != 0 && now_secs >= self.expires_at_secs
    }

    /// Returns true when the ticket is expired at the current wall-clock time.
    #[cfg(feature = "std")]
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(unix_epoch_secs())
    }

    /// Validate the ticket fields without consulting wall-clock time.
    pub fn validate(&self) -> HookTicketResult<()> {
        validate_ticket_fields(self)
    }

    /// Validate the ticket fields and expiry at `now_secs`.
    pub fn validate_at(&self, now_secs: u64) -> HookTicketResult<()> {
        self.validate()?;
        if self.is_expired_at(now_secs) {
            return Err(HookTicketError::ExpiredTicket {
                expires_at_secs: self.expires_at_secs,
                now_secs,
            });
        }
        Ok(())
    }

    /// Serialize the ticket to a base32-encoded string.
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket and validate it at `now_secs`.
    pub fn deserialize_at(input: &str, now_secs: u64) -> HookTicketResult<Self> {
        let ticket = <Self as Ticket>::deserialize(input).map_err(|error| HookTicketError::Deserialize {
            reason: error.to_string(),
        })?;
        ticket.validate_at(now_secs)?;
        Ok(ticket)
    }

    /// Deserialize a ticket and validate it at the current wall-clock time.
    #[cfg(feature = "std")]
    pub fn deserialize(input: &str) -> HookTicketResult<Self> {
        Self::deserialize_at(input, unix_epoch_secs())
    }

    /// Render the expiry status relative to `now_secs`.
    pub fn expiry_string_at(&self, now_secs: u64) -> String {
        if self.expires_at_secs == 0 {
            return "never".to_string();
        }
        if self.is_expired_at(now_secs) {
            return "expired".to_string();
        }

        let remaining_secs = self.expires_at_secs.saturating_sub(now_secs);
        if remaining_secs < SECONDS_PER_MINUTE {
            return format!("{}s", remaining_secs);
        }
        if remaining_secs < SECONDS_PER_HOUR {
            return format!("{}m", remaining_secs / SECONDS_PER_MINUTE);
        }
        if remaining_secs < SECONDS_PER_DAY {
            return format!("{}h", remaining_secs / SECONDS_PER_HOUR);
        }
        format!("{}d", remaining_secs / SECONDS_PER_DAY)
    }

    /// Render the expiry status relative to the current wall-clock time.
    #[cfg(feature = "std")]
    pub fn expiry_string(&self) -> String {
        self.expiry_string_at(unix_epoch_secs())
    }
}

impl Ticket for AspenHookTicket {
    const KIND: &'static str = HOOK_TICKET_PREFIX;

    fn to_bytes(&self) -> Vec<u8> {
        match postcard::to_allocvec(self) {
            Ok(bytes) => bytes,
            Err(error) => {
                debug_assert!(
                    false,
                    "AspenHookTicket serialization should remain infallible for bounded fields: {error}"
                );
                Vec::new()
            }
        }
    }

    fn from_bytes(bytes: &[u8]) -> core::result::Result<Self, iroh_tickets::ParseError> {
        postcard::from_bytes(bytes).map_err(Into::into)
    }
}

fn validate_ticket_fields(ticket: &AspenHookTicket) -> HookTicketResult<()> {
    if ticket.cluster_id.is_empty() {
        return Err(HookTicketError::EmptyClusterId);
    }
    if ticket.cluster_id.len() > MAX_CLUSTER_ID_SIZE {
        return Err(HookTicketError::ClusterIdTooLong {
            actual_bytes: saturating_usize_to_u32(ticket.cluster_id.len()),
            max_bytes: saturating_usize_to_u32(MAX_CLUSTER_ID_SIZE),
        });
    }
    debug_assert!(!ticket.cluster_id.is_empty(), "validated ticket must have a cluster id");
    debug_assert!(ticket.cluster_id.len() <= MAX_CLUSTER_ID_SIZE, "validated cluster id must fit bounds");

    if ticket.bootstrap_peers.is_empty() {
        return Err(HookTicketError::NoBootstrapPeers);
    }
    if ticket.bootstrap_peers.len() > MAX_BOOTSTRAP_PEERS {
        return Err(HookTicketError::TooManyBootstrapPeers {
            actual_peers: saturating_usize_to_u32(ticket.bootstrap_peers.len()),
            max_peers: saturating_usize_to_u32(MAX_BOOTSTRAP_PEERS),
        });
    }
    debug_assert!(!ticket.bootstrap_peers.is_empty(), "validated ticket must have bootstrap peers");
    debug_assert!(
        ticket.bootstrap_peers.len() <= MAX_BOOTSTRAP_PEERS,
        "validated bootstrap peer count must fit bounds"
    );

    if ticket.event_type.is_empty() {
        return Err(HookTicketError::EmptyEventType);
    }
    if ticket.event_type.len() > MAX_EVENT_TYPE_SIZE {
        return Err(HookTicketError::EventTypeTooLong {
            actual_bytes: saturating_usize_to_u32(ticket.event_type.len()),
            max_bytes: saturating_usize_to_u32(MAX_EVENT_TYPE_SIZE),
        });
    }
    debug_assert!(!ticket.event_type.is_empty(), "validated ticket must have an event type");
    debug_assert!(ticket.event_type.len() <= MAX_EVENT_TYPE_SIZE, "validated event type must fit bounds");

    if let Some(payload) = &ticket.default_payload {
        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(HookTicketError::DefaultPayloadTooLong {
                actual_bytes: saturating_usize_to_u32(payload.len()),
                max_bytes: saturating_usize_to_u32(MAX_PAYLOAD_SIZE),
            });
        }
        serde_json::from_str::<serde_json::Value>(payload).map_err(|error| {
            HookTicketError::InvalidDefaultPayloadJson {
                reason: error.to_string(),
            }
        })?;
        debug_assert!(payload.len() <= MAX_PAYLOAD_SIZE, "validated payload must fit bounds");
    }

    if let Some(relay_url) = &ticket.relay_url {
        if relay_url.len() > MAX_RELAY_URL_SIZE {
            return Err(HookTicketError::RelayUrlTooLong {
                actual_bytes: saturating_usize_to_u32(relay_url.len()),
                max_bytes: saturating_usize_to_u32(MAX_RELAY_URL_SIZE),
            });
        }
        debug_assert!(relay_url.len() <= MAX_RELAY_URL_SIZE, "validated relay url must fit bounds");
    }

    if ticket.version > TICKET_VERSION {
        return Err(HookTicketError::UnsupportedVersion {
            version: ticket.version,
            max_supported: TICKET_VERSION,
        });
    }
    debug_assert!(ticket.version <= TICKET_VERSION, "validated ticket version must be supported");

    Ok(())
}

fn saturating_usize_to_u32(value: usize) -> u32 {
    match u32::try_from(value) {
        Ok(converted_value) => converted_value,
        Err(_) => u32::MAX,
    }
}

#[cfg(feature = "std")]
#[allow(unknown_lints)]
#[allow(
    ambient_clock,
    reason = "ticket std convenience wrappers need current wall-clock seconds"
)]
fn unix_epoch_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use core::net::SocketAddr;

    use aspen_cluster_types::NodeTransportAddr;

    use super::*;

    fn create_test_node_address(seed: u8) -> NodeAddress {
        let endpoint_id = format!("{seed:02x}").repeat(32);
        NodeAddress::from_parts(endpoint_id, [NodeTransportAddr::Ip(SocketAddr::from((
            [127, 0, 0, 1],
            7000u16.saturating_add(u16::from(seed)),
        )))])
    }

    #[test]
    fn hook_ticket_golden_stays_stable() {
        const GOLDEN: &str = "aspenhookaeegsnznnbxw623tafadanzqg4ydombxga3tanzqg4ydombxga3tanzqg4ydombxga3tanzqg4ydombxga3tanzqg4ydombxga3tanzqg4ydombxga3tanzqg4aqcad7aaaadxzwb53xe2lumvpwg33nnvuxi5dfmqaq66zconxxk4tdmurduitjg4rh2ajtgmztgmztgmztgmztgmztgmztgmztgmztgmztgmztgmztgmztgoip5t5kayaag";
        let addr = create_test_node_address(7);
        let ticket = AspenHookTicket::new("i7-hooks", vec![addr])
            .with_event_type("write_committed")
            .with_default_payload(r#"{"source":"i7"}"#)
            .with_auth_token([0x33; 32])
            .with_expiry(1_700_003_600)
            .with_priority(3);
        assert_eq!(ticket.serialize(), GOLDEN);
        assert_eq!(AspenHookTicket::deserialize_at(GOLDEN, 1_700_000_000).expect("golden parses"), ticket);
    }

    #[test]
    fn test_ticket_new() {
        let addr = create_test_node_address(1);
        let ticket = AspenHookTicket::new("test-cluster", vec![addr]).with_event_type("write_committed");

        assert_eq!(ticket.cluster_id, "test-cluster");
        assert_eq!(ticket.event_type, "write_committed");
        assert_eq!(ticket.bootstrap_peers.len(), 1);
        assert!(ticket.default_payload.is_none());
        assert!(ticket.auth_token.is_none());
        assert_eq!(ticket.expires_at_secs, 0);
        assert_eq!(ticket.version, TICKET_VERSION);
    }

    #[test]
    fn test_ticket_builder() {
        let addr = create_test_node_address(1);
        let token = [42u8; 32];

        let ticket = AspenHookTicket::new("prod-cluster", vec![addr])
            .with_event_type("delete_committed")
            .with_default_payload(r#"{"source": "external"}"#)
            .with_auth_token(token)
            .with_priority(1)
            .with_relay_url("https://relay.example.com");

        assert_eq!(ticket.cluster_id, "prod-cluster");
        assert_eq!(ticket.event_type, "delete_committed");
        assert_eq!(ticket.default_payload, Some(r#"{"source": "external"}"#.to_string()));
        assert_eq!(ticket.auth_token, Some(token));
        assert_eq!(ticket.priority, 1);
        assert_eq!(ticket.relay_url, Some("https://relay.example.com".to_string()));
    }

    #[test]
    fn hook_ticket_debug_redacts_payload_and_auth_token() {
        let addr = create_test_node_address(9);
        let ticket = AspenHookTicket::new("prod-cluster", vec![addr])
            .with_event_type("write_committed")
            .with_default_payload(r#"{"credential":"synthetic-secret-marker"}"#)
            .with_auth_token([42u8; 32])
            .with_relay_url("https://relay.example.com");

        let debug = format!("{ticket:?}");

        assert!(debug.contains("AspenHookTicket"));
        assert!(debug.contains("has_default_payload: true"));
        assert!(debug.contains("has_auth_token: true"));
        assert!(!debug.contains("synthetic-secret-marker"));
        assert!(!debug.contains("default_payload: Some"));
        assert!(!debug.contains("auth_token: Some"));
        assert!(!debug.contains("[42"));
        assert!(!debug.contains("relay.example.com"));
    }

    #[test]
    fn test_ticket_roundtrip() {
        let addr = create_test_node_address(1);
        let ticket = AspenHookTicket::new("test-cluster", vec![addr])
            .with_event_type("write_committed")
            .with_default_payload(r#"{"key": "value"}"#)
            .with_priority(2);

        let serialized = ticket.serialize();
        assert!(serialized.starts_with(HOOK_TICKET_PREFIX));

        let parsed = AspenHookTicket::deserialize_at(&serialized, 10).expect("should parse");
        assert_eq!(parsed.cluster_id, "test-cluster");
        assert_eq!(parsed.event_type, "write_committed");
        assert_eq!(parsed.default_payload, Some(r#"{"key": "value"}"#.to_string()));
        assert_eq!(parsed.priority, 2);
    }

    #[test]
    fn test_ticket_with_auth_roundtrip() {
        let addr = create_test_node_address(1);
        let token = [0xAB; 32];

        let ticket = AspenHookTicket::new("secure-cluster", vec![addr])
            .with_event_type("leader_elected")
            .with_auth_token(token);

        let serialized = ticket.serialize();
        let parsed = AspenHookTicket::deserialize_at(&serialized, 10).expect("should parse");

        assert!(parsed.requires_auth());
        assert_eq!(parsed.auth_token, Some(token));
    }

    #[test]
    fn test_expiry_helpers() {
        let addr = create_test_node_address(1);
        let now_secs = 100;

        let ticket = AspenHookTicket::new("test", vec![addr.clone()]).with_event_type("write_committed");
        assert!(!ticket.is_expired_at(now_secs));
        assert_eq!(ticket.expiry_string_at(now_secs), "never");

        let expired =
            AspenHookTicket::new("test", vec![addr.clone()]).with_event_type("write_committed").with_expiry(1);
        assert!(expired.is_expired_at(now_secs));
        assert_eq!(expired.expiry_string_at(now_secs), "expired");

        let future = AspenHookTicket::new("test", vec![addr])
            .with_event_type("write_committed")
            .with_expiry_from_now(now_secs, DEFAULT_EXPIRY_HOURS);
        assert!(!future.is_expired_at(now_secs));
        assert_eq!(future.expiry_string_at(now_secs), "1d");
    }

    #[test]
    fn test_validation_empty_cluster_id() {
        let addr = create_test_node_address(1);
        let ticket = AspenHookTicket::new("", vec![addr]).with_event_type("write_committed");
        assert!(matches!(ticket.validate(), Err(HookTicketError::EmptyClusterId)));
    }

    #[test]
    fn test_validation_empty_event_type() {
        let addr = create_test_node_address(1);
        let ticket = AspenHookTicket::new("test", vec![addr]);
        assert!(matches!(ticket.validate(), Err(HookTicketError::EmptyEventType)));
    }

    #[test]
    fn test_validation_no_peers() {
        let ticket = AspenHookTicket::new("test", vec![]).with_event_type("write_committed");
        assert!(matches!(ticket.validate(), Err(HookTicketError::NoBootstrapPeers)));
    }

    #[test]
    fn test_validation_invalid_payload_json() {
        let addr = create_test_node_address(1);
        let ticket = AspenHookTicket::new("test", vec![addr])
            .with_event_type("write_committed")
            .with_default_payload("not valid json {{{");
        assert!(matches!(ticket.validate(), Err(HookTicketError::InvalidDefaultPayloadJson { .. })));
    }

    #[test]
    fn test_add_bootstrap_peer_limit() {
        let mut ticket = AspenHookTicket::new("test", vec![]).with_event_type("write_committed");

        for seed in 0..MAX_BOOTSTRAP_PEERS {
            let addr = create_test_node_address(u8::try_from(seed).unwrap_or(u8::MAX));
            ticket.add_bootstrap_peer(addr).expect("should succeed");
        }

        let extra = create_test_node_address(u8::MAX);
        assert!(matches!(ticket.add_bootstrap_peer(extra), Err(HookTicketError::TooManyBootstrapPeers { .. })));
    }

    #[test]
    fn test_invalid_ticket_string() {
        assert!(matches!(AspenHookTicket::deserialize_at("invalid", 10), Err(HookTicketError::Deserialize { .. })));
        assert!(matches!(
            AspenHookTicket::deserialize_at("aspenclient...", 10),
            Err(HookTicketError::Deserialize { .. })
        ));
        assert!(matches!(
            AspenHookTicket::deserialize_at("aspenhook!!!", 10),
            Err(HookTicketError::Deserialize { .. })
        ));
        assert!(matches!(AspenHookTicket::deserialize_at("", 10), Err(HookTicketError::Deserialize { .. })));
    }

    #[test]
    fn test_deserialize_expired_ticket() {
        let addr = create_test_node_address(1);
        let ticket = AspenHookTicket::new("test", vec![addr]).with_event_type("write_committed").with_expiry(1);

        let serialized = ticket.serialize();
        let result = AspenHookTicket::deserialize_at(&serialized, 2);
        assert!(matches!(result, Err(HookTicketError::ExpiredTicket { .. })));
    }

    #[test]
    fn test_multiple_bootstrap_peers() {
        let addrs: Vec<NodeAddress> = (1..=5).map(create_test_node_address).collect();

        let ticket = AspenHookTicket::new("multi-peer", addrs).with_event_type("snapshot_created");
        assert_eq!(ticket.bootstrap_peers.len(), 5);

        let serialized = ticket.serialize();
        let parsed = AspenHookTicket::deserialize_at(&serialized, 10).expect("should parse");
        assert_eq!(parsed.bootstrap_peers.len(), 5);
    }
}
