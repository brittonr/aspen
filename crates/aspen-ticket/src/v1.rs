//! V1 Aspen cluster ticket for gossip-based peer discovery.

use std::collections::BTreeSet;

use anyhow::Context;
use anyhow::Result;
use iroh::EndpointId;
use iroh_gossip::proto::TopicId;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

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
    pub const MAX_BOOTSTRAP_PEERS: u32 = 16;

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
    pub fn with_bootstrap(topic_id: TopicId, cluster_id: String, bootstrap_peer: EndpointId) -> Self {
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
        if self.bootstrap.len() >= Self::MAX_BOOTSTRAP_PEERS as usize {
            anyhow::bail!("cannot add more than {} bootstrap peers to ticket", Self::MAX_BOOTSTRAP_PEERS);
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
    /// # use aspen_ticket::AspenClusterTicket;
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
    /// # use aspen_ticket::AspenClusterTicket;
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
        // Tiger Style: Document panic conditions explicitly
        //
        // Postcard serialization of AspenClusterTicket can only fail if:
        // 1. Bug in postcard library
        // 2. Memory corruption
        // 3. OOM during Vec allocation
        //
        // This struct contains only primitive types (TopicId, BTreeSet<EndpointId>, String)
        // with bounded sizes (MAX_BOOTSTRAP_PEERS=16), making serialization deterministic.
        //
        // If this panics, it indicates a serious system issue requiring investigation.
        postcard::to_stdvec(&self).expect(
            "AspenClusterTicket postcard serialization failed - \
             indicates library bug or memory corruption",
        )
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}
