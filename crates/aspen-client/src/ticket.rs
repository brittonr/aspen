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
        // SAFETY: postcard serialization of #[derive(Serialize)] types with only
        // primitive fields and standard library types is infallible.
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

    // Helper to create a test endpoint address
    fn test_endpoint_addr() -> EndpointAddr {
        let secret_key = SecretKey::from([1u8; 32]);
        let endpoint_id: EndpointId = secret_key.public();
        EndpointAddr::new(endpoint_id)
    }

    #[test]
    fn test_new_defaults() {
        let addr = test_endpoint_addr();
        let ticket = AspenClientTicket::new("test-cluster", vec![addr]);

        assert_eq!(ticket.cluster_id, "test-cluster");
        assert_eq!(ticket.bootstrap_peers.len(), 1);
        assert_eq!(ticket.access, AccessLevel::ReadOnly);
        assert_eq!(ticket.expires_at_secs, 0);
        assert_eq!(ticket.auth_token, None);
        assert_eq!(ticket.priority, 0);
    }

    #[test]
    fn test_builder_pattern() {
        let addr = test_endpoint_addr();
        let auth_token = [42u8; 32];

        let ticket = AspenClientTicket::new("cluster-1", vec![addr])
            .with_access(AccessLevel::ReadWrite)
            .with_expiry(1234567890)
            .with_auth_token(auth_token)
            .with_priority(5);

        assert_eq!(ticket.access, AccessLevel::ReadWrite);
        assert_eq!(ticket.expires_at_secs, 1234567890);
        assert_eq!(ticket.auth_token, Some(auth_token));
        assert_eq!(ticket.priority, 5);
    }

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
    fn test_roundtrip_with_all_fields() {
        let addr = test_endpoint_addr();
        let auth_token = [99u8; 32];

        let original = AspenClientTicket::new("full-cluster", vec![addr.clone(), addr])
            .with_access(AccessLevel::ReadWrite)
            .with_expiry(9999999999)
            .with_auth_token(auth_token)
            .with_priority(10);

        let serialized = original.serialize();
        let parsed = AspenClientTicket::deserialize(&serialized).expect("should parse");

        assert_eq!(parsed, original);
        assert_eq!(parsed.cluster_id, "full-cluster");
        assert_eq!(parsed.bootstrap_peers.len(), 2);
        assert_eq!(parsed.access, AccessLevel::ReadWrite);
        assert_eq!(parsed.expires_at_secs, 9999999999);
        assert_eq!(parsed.auth_token, Some(auth_token));
        assert_eq!(parsed.priority, 10);
    }

    #[test]
    fn test_expiry_never_expires() {
        let ticket = AspenClientTicket::new("test", vec![]);
        assert!(!ticket.is_expired());
        assert_eq!(ticket.expires_at_secs, 0);
    }

    #[test]
    fn test_expiry_past() {
        // Set expiry to past
        let expired = AspenClientTicket::new("test", vec![]).with_expiry(1);
        assert!(expired.is_expired());
    }

    #[test]
    fn test_expiry_far_future() {
        // Set expiry to far future
        let future = AspenClientTicket::new("test", vec![]).with_expiry(u64::MAX);
        assert!(!future.is_expired());
    }

    #[test]
    fn test_expiry_edge_case_zero() {
        // Zero means no expiry
        let ticket = AspenClientTicket::new("test", vec![]).with_expiry(0);
        assert!(!ticket.is_expired());
    }

    #[test]
    fn test_invalid_ticket_empty() {
        assert!(AspenClientTicket::deserialize("").is_err());
    }

    #[test]
    fn test_invalid_ticket_garbage() {
        assert!(AspenClientTicket::deserialize("invalid").is_err());
        assert!(AspenClientTicket::deserialize("aspenclient!!").is_err());
        assert!(AspenClientTicket::deserialize("aspenclient").is_err());
    }

    #[test]
    fn test_invalid_ticket_wrong_prefix() {
        assert!(AspenClientTicket::deserialize("wrongprefix123").is_err());
    }

    #[test]
    fn test_empty_cluster_id() {
        let addr = test_endpoint_addr();
        let ticket = AspenClientTicket::new("", vec![addr]);
        assert_eq!(ticket.cluster_id, "");

        // Should still roundtrip correctly
        let serialized = ticket.serialize();
        let parsed = AspenClientTicket::deserialize(&serialized).expect("should parse");
        assert_eq!(parsed.cluster_id, "");
    }

    #[test]
    fn test_empty_bootstrap_peers() {
        let ticket = AspenClientTicket::new("cluster-1", vec![]);
        assert_eq!(ticket.bootstrap_peers.len(), 0);

        // Should still roundtrip correctly
        let serialized = ticket.serialize();
        let parsed = AspenClientTicket::deserialize(&serialized).expect("should parse");
        assert_eq!(parsed.bootstrap_peers.len(), 0);
    }

    #[test]
    fn test_multiple_bootstrap_peers() {
        let addr1 = test_endpoint_addr();
        let secret_key2 = SecretKey::from([2u8; 32]);
        let endpoint_id2: EndpointId = secret_key2.public();
        let addr2 = EndpointAddr::new(endpoint_id2);

        let ticket = AspenClientTicket::new("cluster-1", vec![addr1, addr2]);
        assert_eq!(ticket.bootstrap_peers.len(), 2);

        let serialized = ticket.serialize();
        let parsed = AspenClientTicket::deserialize(&serialized).expect("should parse");
        assert_eq!(parsed.bootstrap_peers.len(), 2);
    }

    #[test]
    fn test_access_levels() {
        let addr = test_endpoint_addr();

        let readonly = AspenClientTicket::new("test", vec![addr.clone()]).with_access(AccessLevel::ReadOnly);
        assert_eq!(readonly.access, AccessLevel::ReadOnly);

        let readwrite = AspenClientTicket::new("test", vec![addr]).with_access(AccessLevel::ReadWrite);
        assert_eq!(readwrite.access, AccessLevel::ReadWrite);
    }

    #[test]
    fn test_priority_range() {
        let addr = test_endpoint_addr();

        // Min priority
        let min = AspenClientTicket::new("test", vec![addr.clone()]).with_priority(0);
        assert_eq!(min.priority, 0);

        // Max priority
        let max = AspenClientTicket::new("test", vec![addr.clone()]).with_priority(255);
        assert_eq!(max.priority, 255);

        // Mid priority
        let mid = AspenClientTicket::new("test", vec![addr]).with_priority(128);
        assert_eq!(mid.priority, 128);
    }

    #[test]
    fn test_auth_token_roundtrip() {
        let addr = test_endpoint_addr();
        let token1 = [0u8; 32];
        let token2 = [255u8; 32];
        let mut token3 = [0u8; 32];
        for (i, byte) in token3.iter_mut().enumerate() {
            *byte = i as u8;
        }

        for token in [token1, token2, token3] {
            let ticket = AspenClientTicket::new("test", vec![addr.clone()]).with_auth_token(token);
            assert_eq!(ticket.auth_token, Some(token));

            let serialized = ticket.serialize();
            let parsed = AspenClientTicket::deserialize(&serialized).expect("should parse");
            assert_eq!(parsed.auth_token, Some(token));
        }
    }

    #[test]
    fn test_clone_and_eq() {
        let addr = test_endpoint_addr();
        let ticket1 = AspenClientTicket::new("cluster-1", vec![addr.clone()])
            .with_access(AccessLevel::ReadWrite)
            .with_priority(5);

        let ticket2 = ticket1.clone();
        assert_eq!(ticket1, ticket2);

        let ticket3 = AspenClientTicket::new("cluster-2", vec![addr]).with_priority(5);
        assert_ne!(ticket1, ticket3);
    }

    #[test]
    fn test_serialization_deterministic() {
        let addr = test_endpoint_addr();
        let ticket = AspenClientTicket::new("test", vec![addr]).with_priority(7);

        let ser1 = ticket.serialize();
        let ser2 = ticket.serialize();
        assert_eq!(ser1, ser2);
    }
}
