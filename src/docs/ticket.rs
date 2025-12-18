//! Aspen docs ticket for sharing read/write access.
//!
//! Contains cluster metadata alongside the iroh-docs namespace info.

use anyhow::{Context, Result};
use iroh::EndpointAddr;
use iroh_tickets::Ticket;
use serde::{Deserialize, Serialize};

/// Prefix for serialized Aspen docs tickets.
pub const TICKET_PREFIX: &str = "aspendocs";

/// Aspen-specific docs ticket with cluster metadata.
///
/// Wraps iroh-docs namespace info with additional Aspen context:
/// - Cluster identification
/// - Priority for overlay subscription
/// - Access control (read-only vs read-write)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspenDocsTicket {
    /// Cluster identifier (typically the cluster cookie hash).
    pub cluster_id: String,
    /// Priority for overlay subscription (0 = highest).
    pub priority: u8,
    /// Namespace ID (hex-encoded).
    pub namespace_id: String,
    /// Bootstrap peer addresses.
    pub peers: Vec<EndpointAddr>,
    /// Whether this ticket grants write access.
    pub read_write: bool,
}

impl AspenDocsTicket {
    /// Create a new docs ticket.
    pub fn new(
        cluster_id: String,
        priority: u8,
        namespace_id: String,
        peers: Vec<EndpointAddr>,
        read_write: bool,
    ) -> Self {
        Self {
            cluster_id,
            priority,
            namespace_id,
            peers,
            read_write,
        }
    }

    /// Serialize to a URL-safe string.
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize from a ticket string.
    pub fn deserialize(s: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(s).context("failed to deserialize Aspen docs ticket")
    }
}

impl Ticket for AspenDocsTicket {
    const KIND: &'static str = TICKET_PREFIX;

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("AspenDocsTicket serialization should not fail")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::{EndpointId, SecretKey};

    #[test]
    fn test_ticket_roundtrip() {
        let secret_key = SecretKey::from([1u8; 32]);
        let endpoint_id: EndpointId = secret_key.public();
        let addr = EndpointAddr::new(endpoint_id);

        let ticket = AspenDocsTicket::new(
            "test-cluster".to_string(),
            0,
            "abc123".to_string(),
            vec![addr],
            true,
        );

        let serialized = ticket.serialize();
        assert!(serialized.starts_with(TICKET_PREFIX));

        let parsed = AspenDocsTicket::deserialize(&serialized).expect("should parse");
        assert_eq!(parsed.cluster_id, "test-cluster");
        assert_eq!(parsed.priority, 0);
        assert_eq!(parsed.namespace_id, "abc123");
        assert!(parsed.read_write);
    }

    #[test]
    fn test_invalid_ticket() {
        assert!(AspenDocsTicket::deserialize("invalid").is_err());
        assert!(AspenDocsTicket::deserialize("aspendocs!!!").is_err());
    }
}
