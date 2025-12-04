//! Aspen cluster tickets for peer discovery bootstrap.
//!
//! Tickets provide a compact, URL-safe way to share cluster membership information.
//! They contain the gossip topic ID and bootstrap peer endpoint IDs, encoded as
//! a base32 string following the iroh ticket pattern.
//!
//! # Example
//!
//! ```
//! use aspen::cluster::ticket::AspenClusterTicket;
//! use iroh_gossip::proto::TopicId;
//!
//! // Create a new ticket
//! let topic_id = TopicId::from_bytes([23u8; 32]);
//! let ticket = AspenClusterTicket::new(topic_id, "my-cluster".to_string());
//!
//! // Serialize for sharing
//! let ticket_str = ticket.serialize();
//! println!("Join with: --ticket {}", ticket_str);
//!
//! // Deserialize on another node
//! let parsed = AspenClusterTicket::deserialize(&ticket_str)?;
//! assert_eq!(parsed.topic_id, topic_id);
//! # Ok::<(), anyhow::Error>(())
//! ```

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use iroh::EndpointId;
use iroh_gossip::proto::TopicId;
use iroh_tickets::Ticket;
use serde::{Deserialize, Serialize};

/// Aspen cluster ticket for gossip-based peer discovery.
///
/// Contains all information needed to join an Aspen cluster via iroh-gossip:
/// - `topic_id`: The gossip topic for cluster membership announcements
/// - `bootstrap`: Set of initial peer endpoint IDs to connect to
/// - `cluster_id`: Human-readable cluster identifier
///
/// Tiger Style: Fixed-size TopicId (32 bytes), bounded bootstrap set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AspenClusterTicket {
    /// Gossip topic ID for cluster membership.
    pub topic_id: TopicId,
    /// Bootstrap peer endpoint IDs (max 16 peers).
    pub bootstrap: BTreeSet<EndpointId>,
    /// Human-readable cluster identifier.
    pub cluster_id: String,
}

impl AspenClusterTicket {
    /// Maximum number of bootstrap peers in a ticket.
    ///
    /// Tiger Style: Fixed limit to prevent unbounded ticket size.
    pub const MAX_BOOTSTRAP_PEERS: usize = 16;

    /// Create a new ticket with a topic ID and cluster identifier.
    ///
    /// The bootstrap peer set is initially empty. Use `with_bootstrap()` or
    /// `add_bootstrap()` to add peers.
    pub fn new(topic_id: TopicId, cluster_id: String) -> Self {
        Self {
            topic_id,
            bootstrap: BTreeSet::new(),
            cluster_id,
        }
    }

    /// Create a ticket with a single bootstrap peer.
    pub fn with_bootstrap(
        topic_id: TopicId,
        cluster_id: String,
        bootstrap_peer: EndpointId,
    ) -> Self {
        let mut ticket = Self::new(topic_id, cluster_id);
        ticket.bootstrap.insert(bootstrap_peer);
        ticket
    }

    /// Add a bootstrap peer to the ticket.
    ///
    /// Returns `Err` if the maximum number of bootstrap peers (16) is reached.
    ///
    /// Tiger Style: Fail fast on limit violation.
    pub fn add_bootstrap(&mut self, peer: EndpointId) -> Result<()> {
        if self.bootstrap.len() >= Self::MAX_BOOTSTRAP_PEERS {
            anyhow::bail!(
                "cannot add more than {} bootstrap peers to ticket",
                Self::MAX_BOOTSTRAP_PEERS
            );
        }
        self.bootstrap.insert(peer);
        Ok(())
    }

    /// Serialize the ticket to a base32-encoded string.
    ///
    /// The format is: `aspen{base32-encoded-postcard-payload}`
    ///
    /// # Example
    ///
    /// ```
    /// # use aspen::cluster::ticket::AspenClusterTicket;
    /// # use iroh_gossip::proto::TopicId;
    /// let ticket = AspenClusterTicket::new(
    ///     TopicId::from_bytes([1u8; 32]),
    ///     "test-cluster".into(),
    /// );
    /// let serialized = ticket.serialize();
    /// assert!(serialized.starts_with("aspen"));
    /// ```
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket from a base32-encoded string.
    ///
    /// Returns an error if the string is not a valid Aspen ticket.
    ///
    /// # Example
    ///
    /// ```
    /// # use aspen::cluster::ticket::AspenClusterTicket;
    /// # use iroh_gossip::proto::TopicId;
    /// let ticket = AspenClusterTicket::new(
    ///     TopicId::from_bytes([1u8; 32]),
    ///     "test-cluster".into(),
    /// );
    /// let serialized = ticket.serialize();
    /// let deserialized = AspenClusterTicket::deserialize(&serialized)?;
    /// assert_eq!(ticket, deserialized);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn deserialize(input: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(input).context("failed to deserialize Aspen ticket")
    }
}

impl Ticket for AspenClusterTicket {
    const KIND: &'static str = "aspen";

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect("postcard serialization should not fail")
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_endpoint_id(seed: u8) -> EndpointId {
        let secret_key = iroh::SecretKey::from([seed; 32]);
        secret_key.public()
    }

    #[test]
    fn test_ticket_new() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        assert_eq!(ticket.topic_id, topic_id);
        assert_eq!(ticket.cluster_id, "test-cluster");
        assert!(ticket.bootstrap.is_empty());
    }

    #[test]
    fn test_ticket_with_bootstrap() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);

        assert_eq!(ticket.bootstrap.len(), 1);
        assert!(ticket.bootstrap.contains(&peer_id));
    }

    #[test]
    fn test_add_bootstrap() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        let peer1 = create_test_endpoint_id(1);
        let peer2 = create_test_endpoint_id(2);

        ticket.add_bootstrap(peer1).unwrap();
        ticket.add_bootstrap(peer2).unwrap();

        assert_eq!(ticket.bootstrap.len(), 2);
        assert!(ticket.bootstrap.contains(&peer1));
        assert!(ticket.bootstrap.contains(&peer2));
    }

    #[test]
    fn test_add_bootstrap_max_limit() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Add exactly MAX_BOOTSTRAP_PEERS
        for i in 0..AspenClusterTicket::MAX_BOOTSTRAP_PEERS {
            let peer = create_test_endpoint_id(i as u8);
            ticket.add_bootstrap(peer).unwrap();
        }

        assert_eq!(
            ticket.bootstrap.len(),
            AspenClusterTicket::MAX_BOOTSTRAP_PEERS
        );

        // Adding one more should fail
        let extra_peer = create_test_endpoint_id(255);
        let result = ticket.add_bootstrap(extra_peer);
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let topic_id = TopicId::from_bytes([42u8; 32]);
        let peer1 = create_test_endpoint_id(1);
        let peer2 = create_test_endpoint_id(2);

        let mut ticket = AspenClusterTicket::new(topic_id, "production-cluster".into());
        ticket.add_bootstrap(peer1).unwrap();
        ticket.add_bootstrap(peer2).unwrap();

        // Serialize
        let serialized = ticket.serialize();
        assert!(serialized.starts_with("aspen"));

        // Deserialize
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();
        assert_eq!(ticket, deserialized);
    }

    #[test]
    fn test_deserialize_invalid() {
        // Invalid prefix
        let result = AspenClusterTicket::deserialize("invalid_ticket");
        assert!(result.is_err());

        // Invalid base32
        let result = AspenClusterTicket::deserialize("aspen!!!invalid!!!");
        assert!(result.is_err());

        // Empty string
        let result = AspenClusterTicket::deserialize("");
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_empty_bootstrap() {
        let topic_id = TopicId::from_bytes([1u8; 32]);
        let ticket = AspenClusterTicket::new(topic_id, "empty-bootstrap".into());

        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.bootstrap.len(), 0);
        assert_eq!(deserialized.cluster_id, "empty-bootstrap");
    }
}
