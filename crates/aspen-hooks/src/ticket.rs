//! Hook trigger tickets for external programs.
//!
//! Provides a compact, URL-safe way to share hook trigger credentials with external
//! programs. A hook ticket encapsulates everything needed to trigger hooks on a
//! remote Aspen cluster via Iroh P2P.
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
//! use aspen_hooks::ticket::AspenHookTicket;
//! use iroh::EndpointAddr;
//!
//! // Create a hook ticket
//! let ticket = AspenHookTicket::new("my-cluster", vec![addr])
//!     .with_event_type("write_committed")
//!     .with_default_payload(r#"{"source": "external"}"#)
//!     .with_expiry_hours(24);
//!
//! // Serialize for sharing
//! let url = ticket.serialize();
//! println!("Trigger URL: {}", url);
//!
//! // Deserialize on external program
//! let parsed = AspenHookTicket::deserialize(&url)?;
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

use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use iroh::EndpointAddr;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

// Tiger Style: Explicit bounds for all fields
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

/// Default ticket validity duration (24 hours).
pub const DEFAULT_EXPIRY_HOURS: u64 = 24;

/// Aspen hook trigger ticket for external programs.
///
/// Encapsulates everything needed to trigger a hook on a remote Aspen cluster:
/// - Connection information (bootstrap peers)
/// - Hook configuration (event type, default payload)
/// - Optional authentication (capability token)
/// - Expiration time
///
/// # Tiger Style
///
/// - Fixed-size bounds on all variable fields
/// - Fail-fast validation on construction
/// - Explicit expiration support
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspenHookTicket {
    /// Protocol version for forward compatibility.
    pub version: u8,

    /// Cluster identifier (human-readable).
    pub cluster_id: String,

    /// Bootstrap peer addresses for initial connection.
    pub bootstrap_peers: Vec<EndpointAddr>,

    /// Event type to trigger (e.g., "write_committed").
    pub event_type: String,

    /// Default payload template (JSON string).
    /// External programs can override this when triggering.
    pub default_payload: Option<String>,

    /// Optional authentication token for authorized triggers.
    /// 32-byte HMAC token for capability-based access control.
    pub auth_token: Option<[u8; 32]>,

    /// Unix timestamp when this ticket expires (0 = no expiry).
    pub expires_at_secs: u64,

    /// Optional relay URL for NAT traversal.
    pub relay_url: Option<String>,

    /// Priority hint for connection ordering (0 = highest).
    pub priority: u8,
}

/// Current ticket protocol version.
const TICKET_VERSION: u8 = 1;

impl AspenHookTicket {
    /// Create a new hook ticket.
    ///
    /// # Arguments
    ///
    /// * `cluster_id` - Human-readable cluster identifier
    /// * `bootstrap_peers` - Addresses for initial connection (max 16)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ticket = AspenHookTicket::new("prod-cluster", vec![addr]);
    /// ```
    pub fn new(cluster_id: impl Into<String>, bootstrap_peers: Vec<EndpointAddr>) -> Self {
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
    ///
    /// Valid types: write_committed, delete_committed, leader_elected,
    /// membership_changed, node_added, node_removed, snapshot_created,
    /// snapshot_installed, health_changed, ttl_expired
    pub fn with_event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = event_type.into();
        self
    }

    /// Set the default payload template.
    ///
    /// The payload should be valid JSON. External programs can override
    /// this when triggering.
    pub fn with_default_payload(mut self, payload: impl Into<String>) -> Self {
        self.default_payload = Some(payload.into());
        self
    }

    /// Set the authentication token for authorized triggers.
    ///
    /// The token is a 32-byte HMAC derived from the cluster secret.
    pub fn with_auth_token(mut self, token: [u8; 32]) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Set the expiration time.
    ///
    /// # Arguments
    ///
    /// * `expires_at_secs` - Unix timestamp when ticket expires (0 = no expiry)
    pub fn with_expiry(mut self, expires_at_secs: u64) -> Self {
        self.expires_at_secs = expires_at_secs;
        self
    }

    /// Set expiration relative to now.
    ///
    /// # Arguments
    ///
    /// * `hours` - Hours from now until expiration
    pub fn with_expiry_hours(mut self, hours: u64) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        self.expires_at_secs = now.saturating_add(hours * 3600);
        self
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
    ///
    /// # Errors
    ///
    /// Returns error if the maximum number of peers (16) is reached.
    ///
    /// # Tiger Style
    ///
    /// Fail-fast on limit violation.
    pub fn add_bootstrap_peer(&mut self, peer: EndpointAddr) -> Result<()> {
        if self.bootstrap_peers.len() >= MAX_BOOTSTRAP_PEERS {
            anyhow::bail!("cannot add more than {} bootstrap peers to ticket", MAX_BOOTSTRAP_PEERS);
        }
        self.bootstrap_peers.push(peer);
        Ok(())
    }

    /// Check if the ticket has expired.
    pub fn is_expired(&self) -> bool {
        if self.expires_at_secs == 0 {
            return false; // 0 means no expiry
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        now >= self.expires_at_secs
    }

    /// Check if the ticket requires authentication.
    pub fn requires_auth(&self) -> bool {
        self.auth_token.is_some()
    }

    /// Validate the ticket fields.
    ///
    /// # Errors
    ///
    /// Returns error if any field violates Tiger Style bounds.
    pub fn validate(&self) -> Result<()> {
        // Validate cluster_id
        if self.cluster_id.is_empty() {
            anyhow::bail!("cluster_id cannot be empty");
        }
        if self.cluster_id.len() > MAX_CLUSTER_ID_SIZE {
            anyhow::bail!("cluster_id too long: {} bytes (max {})", self.cluster_id.len(), MAX_CLUSTER_ID_SIZE);
        }

        // Validate bootstrap_peers
        if self.bootstrap_peers.is_empty() {
            anyhow::bail!("at least one bootstrap peer is required");
        }
        if self.bootstrap_peers.len() > MAX_BOOTSTRAP_PEERS {
            anyhow::bail!("too many bootstrap peers: {} (max {})", self.bootstrap_peers.len(), MAX_BOOTSTRAP_PEERS);
        }

        // Validate event_type
        if self.event_type.is_empty() {
            anyhow::bail!("event_type cannot be empty");
        }
        if self.event_type.len() > MAX_EVENT_TYPE_SIZE {
            anyhow::bail!("event_type too long: {} bytes (max {})", self.event_type.len(), MAX_EVENT_TYPE_SIZE);
        }

        // Validate default_payload
        if let Some(ref payload) = self.default_payload {
            if payload.len() > MAX_PAYLOAD_SIZE {
                anyhow::bail!("default_payload too long: {} bytes (max {})", payload.len(), MAX_PAYLOAD_SIZE);
            }
            // Validate it's valid JSON
            serde_json::from_str::<serde_json::Value>(payload).context("default_payload is not valid JSON")?;
        }

        // Validate relay_url
        if let Some(ref url) = self.relay_url
            && url.len() > MAX_RELAY_URL_SIZE
        {
            anyhow::bail!("relay_url too long: {} bytes (max {})", url.len(), MAX_RELAY_URL_SIZE);
        }

        // Validate version
        if self.version > TICKET_VERSION {
            anyhow::bail!("unsupported ticket version {} (max supported: {})", self.version, TICKET_VERSION);
        }

        Ok(())
    }

    /// Serialize the ticket to a base32-encoded string.
    ///
    /// The format is: `aspenhook{base32-encoded-postcard-payload}`
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ticket = AspenHookTicket::new("cluster", vec![addr])
    ///     .with_event_type("write_committed");
    /// let serialized = ticket.serialize();
    /// assert!(serialized.starts_with("aspenhook"));
    /// ```
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket from a base32-encoded string.
    ///
    /// Returns an error if the string is not a valid hook ticket.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let ticket = AspenHookTicket::deserialize("aspenhook...")?;
    /// println!("Cluster: {}", ticket.cluster_id);
    /// ```
    pub fn deserialize(input: &str) -> Result<Self> {
        let ticket = <Self as Ticket>::deserialize(input).context("failed to deserialize Aspen hook ticket")?;

        // Validate after deserialization
        ticket.validate()?;

        // Check expiration
        if ticket.is_expired() {
            anyhow::bail!("hook ticket has expired");
        }

        Ok(ticket)
    }

    /// Returns the expiration time as a human-readable string.
    pub fn expiry_string(&self) -> String {
        if self.expires_at_secs == 0 {
            "never".to_string()
        } else {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

            if now >= self.expires_at_secs {
                "expired".to_string()
            } else {
                let remaining = self.expires_at_secs - now;
                if remaining < 60 {
                    format!("{}s", remaining)
                } else if remaining < 3600 {
                    format!("{}m", remaining / 60)
                } else if remaining < 86400 {
                    format!("{}h", remaining / 3600)
                } else {
                    format!("{}d", remaining / 86400)
                }
            }
        }
    }
}

impl Ticket for AspenHookTicket {
    const KIND: &'static str = HOOK_TICKET_PREFIX;

    fn to_bytes(&self) -> Vec<u8> {
        // Tiger Style: Document panic conditions explicitly
        //
        // Postcard serialization of AspenHookTicket can only fail if:
        // 1. Bug in postcard library
        // 2. Memory corruption
        // 3. OOM during Vec allocation
        //
        // This struct contains only bounded fields with Tiger Style limits,
        // making serialization deterministic.
        //
        // If this panics, it indicates a serious system issue.
        postcard::to_stdvec(self).expect(
            "AspenHookTicket postcard serialization failed - \
             indicates library bug or memory corruption",
        )
    }

    fn from_bytes(bytes: &[u8]) -> std::result::Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

#[cfg(test)]
mod tests {
    use iroh::EndpointId;
    use iroh::SecretKey;

    use super::*;

    fn create_test_endpoint_addr(seed: u8) -> EndpointAddr {
        let secret_key = SecretKey::from([seed; 32]);
        let endpoint_id: EndpointId = secret_key.public();
        EndpointAddr::from(endpoint_id)
    }

    #[test]
    fn test_ticket_new() {
        let addr = create_test_endpoint_addr(1);
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
        let addr = create_test_endpoint_addr(1);
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
    fn test_ticket_roundtrip() {
        let addr = create_test_endpoint_addr(1);
        let ticket = AspenHookTicket::new("test-cluster", vec![addr])
            .with_event_type("write_committed")
            .with_default_payload(r#"{"key": "value"}"#)
            .with_priority(2);

        let serialized = ticket.serialize();
        assert!(serialized.starts_with(HOOK_TICKET_PREFIX));

        let parsed = AspenHookTicket::deserialize(&serialized).expect("should parse");
        assert_eq!(parsed.cluster_id, "test-cluster");
        assert_eq!(parsed.event_type, "write_committed");
        assert_eq!(parsed.default_payload, Some(r#"{"key": "value"}"#.to_string()));
        assert_eq!(parsed.priority, 2);
    }

    #[test]
    fn test_ticket_with_auth_roundtrip() {
        let addr = create_test_endpoint_addr(1);
        let token = [0xAB; 32];

        let ticket = AspenHookTicket::new("secure-cluster", vec![addr])
            .with_event_type("leader_elected")
            .with_auth_token(token);

        let serialized = ticket.serialize();
        let parsed = AspenHookTicket::deserialize(&serialized).expect("should parse");

        assert!(parsed.requires_auth());
        assert_eq!(parsed.auth_token, Some(token));
    }

    #[test]
    fn test_expiry() {
        let addr = create_test_endpoint_addr(1);

        // No expiry
        let ticket = AspenHookTicket::new("test", vec![addr.clone()]).with_event_type("write_committed");
        assert!(!ticket.is_expired());
        assert_eq!(ticket.expiry_string(), "never");

        // Already expired
        let expired =
            AspenHookTicket::new("test", vec![addr.clone()]).with_event_type("write_committed").with_expiry(1); // Unix timestamp 1 = 1970
        assert!(expired.is_expired());
        assert_eq!(expired.expiry_string(), "expired");

        // Future expiry
        let future = AspenHookTicket::new("test", vec![addr]).with_event_type("write_committed").with_expiry_hours(24);
        assert!(!future.is_expired());
        // Should show hours remaining
        let expiry_str = future.expiry_string();
        assert!(expiry_str.ends_with('h') || expiry_str.ends_with('d'), "expected hours/days, got: {}", expiry_str);
    }

    #[test]
    fn test_validation_empty_cluster_id() {
        let addr = create_test_endpoint_addr(1);
        let ticket = AspenHookTicket::new("", vec![addr]).with_event_type("write_committed");
        assert!(ticket.validate().is_err());
    }

    #[test]
    fn test_validation_empty_event_type() {
        let addr = create_test_endpoint_addr(1);
        let ticket = AspenHookTicket::new("test", vec![addr]);
        // Event type not set
        assert!(ticket.validate().is_err());
    }

    #[test]
    fn test_validation_no_peers() {
        let ticket = AspenHookTicket::new("test", vec![]).with_event_type("write_committed");
        assert!(ticket.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_payload_json() {
        let addr = create_test_endpoint_addr(1);
        let ticket = AspenHookTicket::new("test", vec![addr])
            .with_event_type("write_committed")
            .with_default_payload("not valid json {{{");
        assert!(ticket.validate().is_err());
    }

    #[test]
    fn test_add_bootstrap_peer_limit() {
        let mut ticket = AspenHookTicket::new("test", vec![]).with_event_type("write_committed");

        // Add up to the limit
        for i in 0..MAX_BOOTSTRAP_PEERS {
            let addr = create_test_endpoint_addr(i as u8);
            ticket.add_bootstrap_peer(addr).expect("should succeed");
        }

        // One more should fail
        let extra = create_test_endpoint_addr(255);
        assert!(ticket.add_bootstrap_peer(extra).is_err());
    }

    #[test]
    fn test_invalid_ticket_string() {
        // Invalid prefix
        assert!(AspenHookTicket::deserialize("invalid").is_err());
        assert!(AspenHookTicket::deserialize("aspenclient...").is_err());

        // Invalid base32
        assert!(AspenHookTicket::deserialize("aspenhook!!!").is_err());

        // Empty
        assert!(AspenHookTicket::deserialize("").is_err());
    }

    #[test]
    fn test_deserialize_expired_ticket() {
        let addr = create_test_endpoint_addr(1);
        let ticket = AspenHookTicket::new("test", vec![addr]).with_event_type("write_committed").with_expiry(1); // Already expired

        let serialized = ticket.serialize();

        // Deserialize should fail for expired ticket
        let result = AspenHookTicket::deserialize(&serialized);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expired"));
    }

    #[test]
    fn test_multiple_bootstrap_peers() {
        let addrs: Vec<EndpointAddr> = (1..=5).map(|i| create_test_endpoint_addr(i)).collect();

        let ticket = AspenHookTicket::new("multi-peer", addrs.clone()).with_event_type("snapshot_created");

        assert_eq!(ticket.bootstrap_peers.len(), 5);

        let serialized = ticket.serialize();
        let parsed = AspenHookTicket::deserialize(&serialized).expect("should parse");
        assert_eq!(parsed.bootstrap_peers.len(), 5);
    }
}
