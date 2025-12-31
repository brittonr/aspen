//! Aspen client ticket for connecting to clusters.
//!
//! Client tickets contain bootstrap information and access control
//! for connecting to an Aspen cluster.

use anyhow::Context;
use anyhow::Result;
use iroh::EndpointAddr;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

use super::subscription::AccessLevel;

/// Prefix for serialized Aspen client tickets.
pub const CLIENT_TICKET_PREFIX: &str = "aspenclient";

/// Aspen client ticket for connecting to a cluster.
///
/// Contains all information needed to bootstrap a connection:
/// - Bootstrap peer addresses
/// - Cluster identification
/// - Access control (read-only vs read-write)
/// - Optional expiration and authentication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspenClientTicket {
    /// Cluster identifier (typically hash of cluster cookie).
    pub cluster_id: String,
    /// Bootstrap peer addresses for initial connection.
    pub bootstrap_peers: Vec<EndpointAddr>,
    /// Access level granted by this ticket.
    pub access: AccessLevel,
    /// Unix timestamp when this ticket expires (0 = no expiry).
    pub expires_at_secs: u64,
    /// Optional authentication token for write access (HMAC of cluster secret).
    pub auth_token: Option<[u8; 32]>,
    /// Priority hint for overlay ordering.
    pub priority: u8,
}

impl AspenClientTicket {
    /// Create a new client ticket.
    pub fn new(cluster_id: impl Into<String>, bootstrap_peers: Vec<EndpointAddr>) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            bootstrap_peers,
            access: AccessLevel::ReadOnly,
            expires_at_secs: 0,
            auth_token: None,
            priority: 0,
        }
    }

    /// Set the access level for this ticket.
    pub fn with_access(mut self, access: AccessLevel) -> Self {
        self.access = access;
        self
    }

    /// Set the expiration time for this ticket.
    pub fn with_expiry(mut self, expires_at_secs: u64) -> Self {
        self.expires_at_secs = expires_at_secs;
        self
    }

    /// Set the authentication token for write access.
    pub fn with_auth_token(mut self, token: [u8; 32]) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Set the priority hint for overlay ordering.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this ticket has expired.
    pub fn is_expired(&self) -> bool {
        if self.expires_at_secs == 0 {
            return false;
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        now >= self.expires_at_secs
    }

    /// Serialize to a URL-safe string.
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize from a ticket string.
    pub fn deserialize(s: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(s).context("failed to deserialize Aspen client ticket")
    }
}

impl Ticket for AspenClientTicket {
    const KIND: &'static str = CLIENT_TICKET_PREFIX;

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("AspenClientTicket serialization should not fail")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

#[cfg(test)]
mod tests {
    use iroh::EndpointId;
    use iroh::SecretKey;

    use super::*;

    #[test]
    fn test_ticket_roundtrip() {
        let secret_key = SecretKey::from([1u8; 32]);
        let endpoint_id: EndpointId = secret_key.public();
        let addr = EndpointAddr::new(endpoint_id);

        let ticket = AspenClientTicket::new("test-cluster", vec![addr])
            .with_access(AccessLevel::ReadWrite)
            .with_priority(1);

        let serialized = ticket.serialize();
        assert!(serialized.starts_with(CLIENT_TICKET_PREFIX));

        let parsed = AspenClientTicket::deserialize(&serialized).expect("should parse");
        assert_eq!(parsed.cluster_id, "test-cluster");
        assert_eq!(parsed.access, AccessLevel::ReadWrite);
        assert_eq!(parsed.priority, 1);
    }

    #[test]
    fn test_expiry() {
        let ticket = AspenClientTicket::new("test", vec![]);
        assert!(!ticket.is_expired());

        // Set expiry to past
        let expired = ticket.clone().with_expiry(1);
        assert!(expired.is_expired());

        // Set expiry to far future
        let future = AspenClientTicket::new("test", vec![]).with_expiry(u64::MAX);
        assert!(!future.is_expired());
    }

    #[test]
    fn test_invalid_ticket() {
        assert!(AspenClientTicket::deserialize("invalid").is_err());
        assert!(AspenClientTicket::deserialize("aspenclient!!").is_err());
    }
}
