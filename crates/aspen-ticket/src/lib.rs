//! Aspen cluster tickets for peer discovery bootstrap.
//!
//! Tickets provide a compact, URL-safe way to share cluster membership information.
//! They contain the gossip topic ID and bootstrap peer endpoint IDs, encoded as
//! a base32 string following the iroh ticket pattern.
//!
//! # Example
//!
//! ```
//! use aspen_ticket::AspenClusterTicket;
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
use std::net::SocketAddr;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Context;
use anyhow::Result;
use iroh::EndpointAddr;
use iroh::EndpointId;
use iroh::PublicKey;
use iroh::SecretKey;
use iroh::Signature;
use iroh::TransportAddr;
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

// ============================================================================
// V2 Cluster Tickets (with direct addresses for relay-less environments)
// ============================================================================

/// Bootstrap peer information including direct socket addresses.
///
/// Unlike the V1 ticket which only stores EndpointId (public key), this struct
/// includes the direct socket addresses needed to establish connections without
/// relying on discovery mechanisms (mDNS, DNS, DHT, or relay).
///
/// This is essential for:
/// - VM-to-host networking where multicast (mDNS) doesn't traverse
/// - Air-gapped environments without relay/DNS access
/// - Testing scenarios with disabled discovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapPeer {
    /// The peer's cryptographic identity (public key).
    pub endpoint_id: EndpointId,
    /// Direct socket addresses for this peer (e.g., "10.100.0.11:7777").
    /// Can be empty if only relay-based connection is intended.
    pub direct_addrs: Vec<SocketAddr>,
}

impl BootstrapPeer {
    /// Create a new bootstrap peer with only the endpoint ID (no direct addresses).
    pub fn new(endpoint_id: EndpointId) -> Self {
        Self {
            endpoint_id,
            direct_addrs: Vec::new(),
        }
    }

    /// Create a bootstrap peer with direct addresses from an EndpointAddr.
    pub fn from_endpoint_addr(addr: &EndpointAddr) -> Self {
        // Extract direct IP addresses from TransportAddr enum
        let direct_addrs = addr
            .addrs
            .iter()
            .filter_map(|transport_addr| match transport_addr {
                TransportAddr::Ip(socket_addr) => Some(*socket_addr),
                _ => None,
            })
            .collect();

        Self {
            endpoint_id: addr.id,
            direct_addrs,
        }
    }

    /// Convert to an EndpointAddr for connection.
    pub fn to_endpoint_addr(&self) -> EndpointAddr {
        let addrs: BTreeSet<TransportAddr> = self.direct_addrs.iter().map(|addr| TransportAddr::Ip(*addr)).collect();

        EndpointAddr {
            id: self.endpoint_id,
            addrs,
        }
    }
}

impl From<&EndpointAddr> for BootstrapPeer {
    fn from(addr: &EndpointAddr) -> Self {
        Self::from_endpoint_addr(addr)
    }
}

impl From<EndpointId> for BootstrapPeer {
    fn from(id: EndpointId) -> Self {
        Self::new(id)
    }
}

/// Aspen cluster ticket V2 with direct address support.
///
/// This ticket format includes direct socket addresses for bootstrap peers,
/// enabling connection without discovery mechanisms. Use this for:
/// - VM networking where mDNS doesn't work
/// - Relay-disabled environments
/// - Air-gapped deployments
///
/// The ticket is backward-compatible: clients that only support V1 can fall back
/// to discovery-based connection using just the endpoint IDs.
///
/// Tiger Style: Fixed limits on peers and addresses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AspenClusterTicketV2 {
    /// Gossip topic ID for cluster membership.
    pub topic_id: TopicId,
    /// Bootstrap peers with direct addresses (max 16 peers).
    pub bootstrap: Vec<BootstrapPeer>,
    /// Human-readable cluster identifier.
    pub cluster_id: String,
}

impl AspenClusterTicketV2 {
    /// Maximum number of bootstrap peers in a ticket.
    pub const MAX_BOOTSTRAP_PEERS: usize = 16;

    /// Maximum direct addresses per peer.
    pub const MAX_DIRECT_ADDRS_PER_PEER: usize = 8;

    /// Create a new V2 ticket with a topic ID and cluster identifier.
    pub fn new(topic_id: TopicId, cluster_id: String) -> Self {
        Self {
            topic_id,
            bootstrap: Vec::new(),
            cluster_id,
        }
    }

    /// Create a ticket with a single bootstrap peer from an EndpointAddr.
    pub fn with_bootstrap_addr(topic_id: TopicId, cluster_id: String, addr: &EndpointAddr) -> Self {
        let mut ticket = Self::new(topic_id, cluster_id);
        ticket.bootstrap.push(BootstrapPeer::from_endpoint_addr(addr));
        ticket
    }

    /// Add a bootstrap peer from an EndpointAddr.
    ///
    /// Returns `Err` if the maximum number of bootstrap peers is reached.
    pub fn add_bootstrap_addr(&mut self, addr: &EndpointAddr) -> Result<()> {
        if self.bootstrap.len() >= Self::MAX_BOOTSTRAP_PEERS {
            anyhow::bail!("cannot add more than {} bootstrap peers to ticket", Self::MAX_BOOTSTRAP_PEERS);
        }
        let mut peer = BootstrapPeer::from_endpoint_addr(addr);
        // Limit direct addresses per peer
        peer.direct_addrs.truncate(Self::MAX_DIRECT_ADDRS_PER_PEER);
        self.bootstrap.push(peer);
        Ok(())
    }

    /// Get all endpoint addresses for direct connection.
    pub fn endpoint_addrs(&self) -> Vec<EndpointAddr> {
        self.bootstrap.iter().map(|p| p.to_endpoint_addr()).collect()
    }

    /// Get just the endpoint IDs (for V1 compatibility).
    pub fn endpoint_ids(&self) -> BTreeSet<EndpointId> {
        self.bootstrap.iter().map(|p| p.endpoint_id).collect()
    }

    /// Convert to a V1 ticket (loses direct address information).
    pub fn to_v1(&self) -> AspenClusterTicket {
        let mut ticket = AspenClusterTicket::new(self.topic_id, self.cluster_id.clone());
        for peer in &self.bootstrap {
            // Ignore errors from max peers limit
            let _ = ticket.add_bootstrap(peer.endpoint_id);
        }
        ticket
    }

    /// Create from a V1 ticket (no direct addresses).
    pub fn from_v1(v1: &AspenClusterTicket) -> Self {
        Self {
            topic_id: v1.topic_id,
            bootstrap: v1.bootstrap.iter().map(|id| BootstrapPeer::new(*id)).collect(),
            cluster_id: v1.cluster_id.clone(),
        }
    }

    /// Serialize the ticket to a base32-encoded string.
    ///
    /// The format is: `aspenv2{base32-encoded-postcard-payload}`
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket from a base32-encoded string.
    pub fn deserialize(input: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(input).context("failed to deserialize Aspen V2 ticket")
    }

    /// Inject an additional direct address into all bootstrap peers.
    ///
    /// This is used for VM connectivity where the bridge IP must be added
    /// to the ticket so VMs can reach the host's Iroh endpoint from the
    /// isolated VM network (e.g., 10.200.0.0/24).
    ///
    /// The address is only added if it's not already present in the peer's
    /// direct address list.
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address to inject (e.g., 10.200.0.1:PORT)
    pub fn inject_direct_addr(&mut self, addr: SocketAddr) {
        for peer in &mut self.bootstrap {
            if !peer.direct_addrs.contains(&addr) {
                peer.direct_addrs.push(addr);
            }
        }
    }
}

impl Ticket for AspenClusterTicketV2 {
    const KIND: &'static str = "aspenv2";

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect(
            "AspenClusterTicketV2 postcard serialization failed - \
             indicates library bug or memory corruption",
        )
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

// ============================================================================
// Unified ticket parsing (V1 or V2)
// ============================================================================

/// Parse either a V1 or V2 ticket string and return endpoint addresses.
///
/// This function provides a unified interface for ticket parsing:
/// - V1 tickets (aspen...) return addresses with only endpoint IDs
/// - V2 tickets (aspenv2...) return addresses with direct socket addresses
///
/// Use this when you want to connect regardless of ticket version.
pub fn parse_ticket_to_addrs(ticket_str: &str) -> Result<(TopicId, String, Vec<EndpointAddr>)> {
    if ticket_str.starts_with("aspenv2") {
        let ticket = AspenClusterTicketV2::deserialize(ticket_str)?;
        let addrs = ticket.endpoint_addrs();
        Ok((ticket.topic_id, ticket.cluster_id, addrs))
    } else if ticket_str.starts_with("aspen") {
        let ticket = AspenClusterTicket::deserialize(ticket_str)?;
        let addrs = ticket
            .bootstrap
            .iter()
            .map(|id| EndpointAddr {
                id: *id,
                addrs: BTreeSet::new(),
            })
            .collect();
        Ok((ticket.topic_id, ticket.cluster_id, addrs))
    } else {
        anyhow::bail!("invalid ticket format: must start with 'aspen' or 'aspenv2'")
    }
}

// ============================================================================
// Signed Cluster Tickets (HIGH-5 Security Enhancement)
// ============================================================================

/// Current signed ticket protocol version.
///
/// Version history:
/// - v1: Initial signed ticket format with Ed25519 signatures
const SIGNED_TICKET_VERSION: u8 = 1;

/// Default ticket validity duration (24 hours).
const DEFAULT_TICKET_VALIDITY_SECS: u64 = 24 * 3600;

/// Clock skew tolerance for timestamp validation (5 minutes).
const CLOCK_SKEW_TOLERANCE_SECS: u64 = 300;

/// Signed cluster ticket with Ed25519 signature verification.
///
/// Provides cryptographic authentication for cluster join tickets:
/// - Proves ticket was created by a known cluster member
/// - Prevents ticket forgery by malicious actors
/// - Includes timestamp-based expiration for replay prevention
/// - Uses 128-bit nonce for additional replay protection
///
/// Tiger Style: Fixed-size signature (64 bytes), bounded fields, fail-fast verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAspenClusterTicket {
    /// Protocol version for forward compatibility.
    pub version: u8,
    /// The inner ticket payload (topic, bootstrap, cluster_id).
    pub ticket: AspenClusterTicket,
    /// Public key of the ticket creator (cluster member who signed).
    pub issuer: PublicKey,
    /// Unix timestamp when ticket was created (seconds since epoch).
    pub issued_at_secs: u64,
    /// Unix timestamp when ticket expires (seconds since epoch).
    pub expires_at_secs: u64,
    /// Random nonce for replay prevention (128 bits).
    pub nonce: [u8; 16],
    /// Ed25519 signature over the serialized payload (64 bytes).
    pub signature: Signature,
}

impl SignedAspenClusterTicket {
    /// Create and sign a new cluster ticket.
    ///
    /// Signs the ticket payload with the provided secret key. The issuer's
    /// public key is derived from the secret key and embedded in the ticket.
    ///
    /// # Arguments
    /// * `ticket` - The unsigned cluster ticket payload
    /// * `secret_key` - The signer's Iroh secret key
    ///
    /// # Returns
    /// A signed ticket with 24-hour validity (default TTL).
    pub fn sign(ticket: AspenClusterTicket, secret_key: &SecretKey) -> Result<Self> {
        Self::sign_with_validity(ticket, secret_key, DEFAULT_TICKET_VALIDITY_SECS)
    }

    /// Create and sign a ticket with custom validity duration.
    ///
    /// # Arguments
    /// * `ticket` - The unsigned cluster ticket payload
    /// * `secret_key` - The signer's Iroh secret key
    /// * `validity_secs` - How long the ticket is valid (in seconds)
    pub fn sign_with_validity(ticket: AspenClusterTicket, secret_key: &SecretKey, validity_secs: u64) -> Result<Self> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).context("system time before Unix epoch")?.as_secs();

        // Generate random nonce for replay prevention
        let mut nonce = [0u8; 16];
        {
            use rand::RngCore;
            rand::rng().fill_bytes(&mut nonce);
        }

        // Build the unsigned payload for signing
        let payload = SignedTicketPayload {
            version: SIGNED_TICKET_VERSION,
            ticket: &ticket,
            issuer: secret_key.public(),
            issued_at_secs: now,
            expires_at_secs: now.saturating_add(validity_secs),
            nonce,
        };

        // Serialize payload to canonical bytes
        let payload_bytes = payload.to_bytes()?;

        // Sign with Ed25519
        let signature = secret_key.sign(&payload_bytes);

        Ok(Self {
            version: SIGNED_TICKET_VERSION,
            ticket,
            issuer: secret_key.public(),
            issued_at_secs: now,
            expires_at_secs: now.saturating_add(validity_secs),
            nonce,
            signature,
        })
    }

    /// Verify the signature and return the inner ticket if valid.
    ///
    /// Performs the following checks:
    /// 1. Signature verification using issuer's public key
    /// 2. Timestamp validation (not expired, not in future)
    /// 3. Version compatibility check
    ///
    /// Returns `None` if any validation fails (fail-fast, no error details
    /// to prevent information leakage to attackers).
    pub fn verify(&self) -> Option<&AspenClusterTicket> {
        // Reject unknown future versions
        if self.version > SIGNED_TICKET_VERSION {
            return None;
        }

        // Build the payload for signature verification
        let payload = SignedTicketPayload {
            version: self.version,
            ticket: &self.ticket,
            issuer: self.issuer,
            issued_at_secs: self.issued_at_secs,
            expires_at_secs: self.expires_at_secs,
            nonce: self.nonce,
        };

        // Serialize payload to canonical bytes
        let payload_bytes = payload.to_bytes().ok()?;

        // Verify Ed25519 signature
        if self.issuer.verify(&payload_bytes, &self.signature).is_err() {
            return None;
        }

        // Verify timestamps (with clock skew tolerance)
        if !self.is_timestamp_valid() {
            return None;
        }

        Some(&self.ticket)
    }

    /// Verify signature and timestamps, returning detailed error on failure.
    ///
    /// Unlike `verify()`, this method provides error context for debugging.
    /// Use this for logging/diagnostics, NOT for security decisions.
    pub fn verify_with_error(&self) -> Result<&AspenClusterTicket> {
        // Reject unknown future versions
        if self.version > SIGNED_TICKET_VERSION {
            anyhow::bail!("unsupported ticket version {} (max supported: {})", self.version, SIGNED_TICKET_VERSION);
        }

        // Build the payload for signature verification
        let payload = SignedTicketPayload {
            version: self.version,
            ticket: &self.ticket,
            issuer: self.issuer,
            issued_at_secs: self.issued_at_secs,
            expires_at_secs: self.expires_at_secs,
            nonce: self.nonce,
        };

        // Serialize payload to canonical bytes
        let payload_bytes = payload.to_bytes().context("failed to serialize ticket payload for verification")?;

        // Verify Ed25519 signature
        self.issuer.verify(&payload_bytes, &self.signature).context("signature verification failed")?;

        // Verify timestamps
        self.verify_timestamps()?;

        Ok(&self.ticket)
    }

    /// Check if the timestamp is valid (not expired, not in future).
    fn is_timestamp_valid(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

        // Check not issued in the future (with clock skew tolerance)
        if self.issued_at_secs > now.saturating_add(CLOCK_SKEW_TOLERANCE_SECS) {
            return false;
        }

        // Check not expired
        if self.expires_at_secs < now {
            return false;
        }

        true
    }

    /// Verify timestamps with detailed error messages.
    fn verify_timestamps(&self) -> Result<()> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).context("system time before Unix epoch")?.as_secs();

        // Check not issued in the future (with clock skew tolerance)
        if self.issued_at_secs > now.saturating_add(CLOCK_SKEW_TOLERANCE_SECS) {
            anyhow::bail!(
                "ticket issued_at {} is in the future (now: {}, tolerance: {}s)",
                self.issued_at_secs,
                now,
                CLOCK_SKEW_TOLERANCE_SECS
            );
        }

        // Check not expired
        if self.expires_at_secs < now {
            anyhow::bail!("ticket expired at {} (now: {})", self.expires_at_secs, now);
        }

        Ok(())
    }

    /// Serialize the signed ticket to a base32-encoded string.
    ///
    /// Format: `aspensigned{base32-encoded-postcard-payload}`
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a signed ticket from a base32-encoded string.
    ///
    /// Note: This only parses the ticket, it does NOT verify the signature.
    /// Call `verify()` after deserialization to validate the ticket.
    pub fn deserialize(input: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(input).context("failed to deserialize signed Aspen ticket")
    }

    /// Returns the issuer's public key.
    pub fn issuer(&self) -> &PublicKey {
        &self.issuer
    }

    /// Returns the ticket expiration timestamp.
    pub fn expires_at(&self) -> u64 {
        self.expires_at_secs
    }

    /// Returns whether the ticket has expired.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        self.expires_at_secs < now
    }
}

impl Ticket for SignedAspenClusterTicket {
    const KIND: &'static str = "aspensigned";

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect(
            "SignedAspenClusterTicket postcard serialization failed - \
             indicates library bug or memory corruption",
        )
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        let ticket = postcard::from_bytes(bytes)?;
        Ok(ticket)
    }
}

/// Internal payload structure for signing (excludes the signature field).
///
/// This struct is used to serialize the data that gets signed, ensuring
/// the signature is computed over a canonical byte representation.
#[derive(Serialize)]
struct SignedTicketPayload<'a> {
    version: u8,
    ticket: &'a AspenClusterTicket,
    issuer: PublicKey,
    issued_at_secs: u64,
    expires_at_secs: u64,
    nonce: [u8; 16],
}

impl SignedTicketPayload<'_> {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("failed to serialize ticket payload")
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

        assert_eq!(ticket.bootstrap.len(), AspenClusterTicket::MAX_BOOTSTRAP_PEERS as usize);

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

    // ========================================================================
    // Signed Ticket Tests
    // ========================================================================

    fn create_test_secret_key(seed: u8) -> SecretKey {
        SecretKey::from([seed; 32])
    }

    #[test]
    fn test_signed_ticket_sign_and_verify() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);

        // Sign the ticket
        let signed = SignedAspenClusterTicket::sign(ticket.clone(), &secret_key).unwrap();

        // Verify should succeed
        let verified = signed.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap(), &ticket);

        // Issuer should match signer's public key
        assert_eq!(*signed.issuer(), secret_key.public());
    }

    #[test]
    fn test_signed_ticket_roundtrip_serialization() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let ticket = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);
        let signed = SignedAspenClusterTicket::sign(ticket.clone(), &secret_key).unwrap();

        // Serialize
        let serialized = signed.serialize();
        assert!(serialized.starts_with("aspensigned"));

        // Deserialize
        let deserialized = SignedAspenClusterTicket::deserialize(&serialized).unwrap();

        // Verify should still succeed after roundtrip
        let verified = deserialized.verify();
        assert!(verified.is_some());
        assert_eq!(verified.unwrap(), &ticket);
    }

    #[test]
    fn test_signed_ticket_wrong_key_rejected() {
        let real_key = create_test_secret_key(42);
        let fake_key = create_test_secret_key(99);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Sign with real key
        let signed = SignedAspenClusterTicket::sign(ticket, &real_key).unwrap();

        // Tamper: replace issuer with fake key's public key
        let tampered = SignedAspenClusterTicket {
            issuer: fake_key.public(),
            ..signed
        };

        // Verification should fail (signature doesn't match fake issuer)
        assert!(tampered.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_tampered_data_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "original-cluster".into());
        let signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Tamper with the inner ticket data
        let mut tampered = signed.clone();
        tampered.ticket.cluster_id = "tampered-cluster".into();

        // Verification should fail (data doesn't match signature)
        assert!(tampered.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_expired_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Create a ticket that's already expired by tampering with timestamps
        let mut signed = SignedAspenClusterTicket::sign_with_validity(ticket, &secret_key, 3600).unwrap();

        // Set expires_at to the past (1 hour ago)
        signed.expires_at_secs = signed.issued_at_secs.saturating_sub(3600);

        // Verification should fail due to expiration
        assert!(signed.verify().is_none());
        assert!(signed.is_expired());
    }

    #[test]
    fn test_signed_ticket_future_issued_at_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let mut signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Tamper: set issued_at far in the future (beyond clock skew tolerance)
        signed.issued_at_secs += 3600; // 1 hour in future

        // Verification should fail
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_nonce_is_random() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket1 = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let ticket2 = AspenClusterTicket::new(topic_id, "test-cluster".into());

        let signed1 = SignedAspenClusterTicket::sign(ticket1, &secret_key).unwrap();
        let signed2 = SignedAspenClusterTicket::sign(ticket2, &secret_key).unwrap();

        // Nonces should be different (randomly generated)
        assert_ne!(signed1.nonce, signed2.nonce);
    }

    #[test]
    fn test_signed_ticket_verify_with_error() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let signed = SignedAspenClusterTicket::sign(ticket.clone(), &secret_key).unwrap();

        // Valid ticket should verify with detailed result
        let result = signed.verify_with_error();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), &ticket);

        // Tampered ticket should give error message
        let mut tampered = signed;
        tampered.ticket.cluster_id = "tampered".into();
        let result = tampered.verify_with_error();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("signature"));
    }

    #[test]
    fn test_signed_ticket_version_field() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Version should be current
        assert_eq!(signed.version, SIGNED_TICKET_VERSION);
    }

    #[test]
    fn test_signed_ticket_future_version_rejected() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        let mut signed = SignedAspenClusterTicket::sign(ticket, &secret_key).unwrap();

        // Set version to a future version
        signed.version = SIGNED_TICKET_VERSION + 1;

        // Verification should fail (unknown version)
        assert!(signed.verify().is_none());
    }

    #[test]
    fn test_signed_ticket_with_custom_validity() {
        let secret_key = create_test_secret_key(42);
        let topic_id = TopicId::from_bytes([23u8; 32]);

        let ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());

        // Create ticket with 1 hour validity
        let one_hour_secs = 3600;
        let signed = SignedAspenClusterTicket::sign_with_validity(ticket.clone(), &secret_key, one_hour_secs).unwrap();

        // Should not be expired
        assert!(!signed.is_expired());

        // Expires at should be roughly 1 hour from issued_at
        let validity_duration = signed.expires_at_secs - signed.issued_at_secs;
        assert_eq!(validity_duration, one_hour_secs);

        // Verification should succeed
        assert!(signed.verify().is_some());
    }

    // ========================================================================
    // BootstrapPeer Tests
    // ========================================================================

    fn create_test_socket_addr(port: u16) -> SocketAddr {
        use std::net::IpAddr;
        use std::net::Ipv4Addr;
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, port as u8)), port)
    }

    fn create_test_endpoint_addr(seed: u8, addrs: &[SocketAddr]) -> EndpointAddr {
        let id = create_test_endpoint_id(seed);
        let transport_addrs: BTreeSet<TransportAddr> = addrs.iter().map(|a| TransportAddr::Ip(*a)).collect();
        EndpointAddr {
            id,
            addrs: transport_addrs,
        }
    }

    #[test]
    fn test_bootstrap_peer_new() {
        let endpoint_id = create_test_endpoint_id(1);
        let peer = BootstrapPeer::new(endpoint_id);

        assert_eq!(peer.endpoint_id, endpoint_id);
        assert!(peer.direct_addrs.is_empty());
    }

    #[test]
    fn test_bootstrap_peer_from_endpoint_addr() {
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr1, addr2]);

        let peer = BootstrapPeer::from_endpoint_addr(&endpoint_addr);

        assert_eq!(peer.endpoint_id, endpoint_addr.id);
        assert_eq!(peer.direct_addrs.len(), 2);
        assert!(peer.direct_addrs.contains(&addr1));
        assert!(peer.direct_addrs.contains(&addr2));
    }

    #[test]
    fn test_bootstrap_peer_from_endpoint_addr_empty() {
        let endpoint_addr = create_test_endpoint_addr(1, &[]);

        let peer = BootstrapPeer::from_endpoint_addr(&endpoint_addr);

        assert_eq!(peer.endpoint_id, endpoint_addr.id);
        assert!(peer.direct_addrs.is_empty());
    }

    #[test]
    fn test_bootstrap_peer_to_endpoint_addr() {
        let endpoint_id = create_test_endpoint_id(1);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);

        let peer = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr1, addr2],
        };

        let endpoint_addr = peer.to_endpoint_addr();

        assert_eq!(endpoint_addr.id, endpoint_id);
        assert_eq!(endpoint_addr.addrs.len(), 2);
        assert!(endpoint_addr.addrs.contains(&TransportAddr::Ip(addr1)));
        assert!(endpoint_addr.addrs.contains(&TransportAddr::Ip(addr2)));
    }

    #[test]
    fn test_bootstrap_peer_roundtrip_conversion() {
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let original = create_test_endpoint_addr(1, &[addr1, addr2]);

        let peer = BootstrapPeer::from_endpoint_addr(&original);
        let roundtripped = peer.to_endpoint_addr();

        assert_eq!(roundtripped.id, original.id);
        assert_eq!(roundtripped.addrs, original.addrs);
    }

    #[test]
    fn test_bootstrap_peer_from_trait_endpoint_addr() {
        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        let peer: BootstrapPeer = (&endpoint_addr).into();

        assert_eq!(peer.endpoint_id, endpoint_addr.id);
        assert!(peer.direct_addrs.contains(&addr));
    }

    #[test]
    fn test_bootstrap_peer_from_trait_endpoint_id() {
        let endpoint_id = create_test_endpoint_id(1);

        let peer: BootstrapPeer = endpoint_id.into();

        assert_eq!(peer.endpoint_id, endpoint_id);
        assert!(peer.direct_addrs.is_empty());
    }

    // ========================================================================
    // AspenClusterTicketV2 Tests
    // ========================================================================

    #[test]
    fn test_ticket_v2_new() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        assert_eq!(ticket.topic_id, topic_id);
        assert_eq!(ticket.cluster_id, "test-cluster");
        assert!(ticket.bootstrap.is_empty());
    }

    #[test]
    fn test_ticket_v2_with_bootstrap_addr() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        let ticket = AspenClusterTicketV2::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);

        assert_eq!(ticket.bootstrap.len(), 1);
        assert_eq!(ticket.bootstrap[0].endpoint_id, endpoint_addr.id);
        assert!(ticket.bootstrap[0].direct_addrs.contains(&addr));
    }

    #[test]
    fn test_ticket_v2_add_bootstrap_addr() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        assert_eq!(ticket.bootstrap.len(), 2);
    }

    #[test]
    fn test_ticket_v2_add_bootstrap_max_limit() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        // Add exactly MAX_BOOTSTRAP_PEERS
        for i in 0..AspenClusterTicketV2::MAX_BOOTSTRAP_PEERS {
            let addr = create_test_socket_addr((7000 + i) as u16);
            let endpoint_addr = create_test_endpoint_addr(i as u8, &[addr]);
            ticket.add_bootstrap_addr(&endpoint_addr).unwrap();
        }

        assert_eq!(ticket.bootstrap.len(), AspenClusterTicketV2::MAX_BOOTSTRAP_PEERS);

        // Adding one more should fail
        let extra_addr = create_test_socket_addr(9999);
        let extra_endpoint = create_test_endpoint_addr(255, &[extra_addr]);
        let result = ticket.add_bootstrap_addr(&extra_endpoint);
        assert!(result.is_err());
    }

    #[test]
    fn test_ticket_v2_add_bootstrap_truncates_addrs() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        // Create endpoint with more than MAX_DIRECT_ADDRS_PER_PEER addresses
        let addrs: Vec<SocketAddr> = (0..12).map(|i| create_test_socket_addr(7000 + i)).collect();
        let endpoint_addr = create_test_endpoint_addr(1, &addrs);

        ticket.add_bootstrap_addr(&endpoint_addr).unwrap();

        // Should be truncated to MAX_DIRECT_ADDRS_PER_PEER
        assert_eq!(ticket.bootstrap[0].direct_addrs.len(), AspenClusterTicketV2::MAX_DIRECT_ADDRS_PER_PEER);
    }

    #[test]
    fn test_ticket_v2_endpoint_addrs() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());
        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        let addrs = ticket.endpoint_addrs();

        assert_eq!(addrs.len(), 2);
        assert!(addrs.iter().any(|a| a.id == endpoint_addr1.id));
        assert!(addrs.iter().any(|a| a.id == endpoint_addr2.id));
    }

    #[test]
    fn test_ticket_v2_endpoint_ids() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let endpoint_id1 = create_test_endpoint_id(1);
        let endpoint_id2 = create_test_endpoint_id(2);

        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());
        ticket.bootstrap.push(BootstrapPeer::new(endpoint_id1));
        ticket.bootstrap.push(BootstrapPeer::new(endpoint_id2));

        let ids = ticket.endpoint_ids();

        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&endpoint_id1));
        assert!(ids.contains(&endpoint_id2));
    }

    #[test]
    fn test_ticket_v2_serialize_deserialize() {
        let topic_id = TopicId::from_bytes([42u8; 32]);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        let mut ticket = AspenClusterTicketV2::new(topic_id, "production-cluster".into());
        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        // Serialize
        let serialized = ticket.serialize();
        assert!(serialized.starts_with("aspenv2"));

        // Deserialize
        let deserialized = AspenClusterTicketV2::deserialize(&serialized).unwrap();
        assert_eq!(ticket, deserialized);
    }

    #[test]
    fn test_ticket_v2_deserialize_invalid() {
        // Invalid prefix
        let result = AspenClusterTicketV2::deserialize("invalid_ticket");
        assert!(result.is_err());

        // V1 prefix (wrong version)
        let v1_ticket = AspenClusterTicket::new(TopicId::from_bytes([1u8; 32]), "test".into());
        let result = AspenClusterTicketV2::deserialize(&v1_ticket.serialize());
        assert!(result.is_err());

        // Empty string
        let result = AspenClusterTicketV2::deserialize("");
        assert!(result.is_err());
    }

    #[test]
    fn test_ticket_v2_serialize_empty_bootstrap() {
        let topic_id = TopicId::from_bytes([1u8; 32]);
        let ticket = AspenClusterTicketV2::new(topic_id, "empty-bootstrap".into());

        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicketV2::deserialize(&serialized).unwrap();

        assert!(deserialized.bootstrap.is_empty());
        assert_eq!(deserialized.cluster_id, "empty-bootstrap");
    }

    // ========================================================================
    // V1/V2 Conversion Tests
    // ========================================================================

    #[test]
    fn test_ticket_v2_to_v1() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        let v2 = AspenClusterTicketV2::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);
        let v1 = v2.to_v1();

        assert_eq!(v1.topic_id, topic_id);
        assert_eq!(v1.cluster_id, "test-cluster");
        assert_eq!(v1.bootstrap.len(), 1);
        assert!(v1.bootstrap.contains(&endpoint_addr.id));
    }

    #[test]
    fn test_ticket_v2_to_v1_loses_addresses() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        let v2 = AspenClusterTicketV2::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);
        let v1 = v2.to_v1();

        // V1 only has endpoint IDs, no addresses
        // Verify by re-converting: V1->V2 will have no addresses
        let back_to_v2 = AspenClusterTicketV2::from_v1(&v1);
        assert!(back_to_v2.bootstrap[0].direct_addrs.is_empty());
    }

    #[test]
    fn test_ticket_v2_from_v1() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let v1 = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);
        let v2 = AspenClusterTicketV2::from_v1(&v1);

        assert_eq!(v2.topic_id, topic_id);
        assert_eq!(v2.cluster_id, "test-cluster");
        assert_eq!(v2.bootstrap.len(), 1);
        assert_eq!(v2.bootstrap[0].endpoint_id, peer_id);
        assert!(v2.bootstrap[0].direct_addrs.is_empty());
    }

    #[test]
    fn test_ticket_v2_from_v1_multiple_peers() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut v1 = AspenClusterTicket::new(topic_id, "test-cluster".into());

        for i in 0..5 {
            v1.add_bootstrap(create_test_endpoint_id(i)).unwrap();
        }

        let v2 = AspenClusterTicketV2::from_v1(&v1);

        assert_eq!(v2.bootstrap.len(), 5);
        for peer in &v2.bootstrap {
            assert!(peer.direct_addrs.is_empty());
        }
    }

    #[test]
    fn test_ticket_v1_v2_roundtrip() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id = create_test_endpoint_id(1);

        let original_v1 = AspenClusterTicket::with_bootstrap(topic_id, "test-cluster".into(), peer_id);

        // V1 -> V2 -> V1
        let v2 = AspenClusterTicketV2::from_v1(&original_v1);
        let back_to_v1 = v2.to_v1();

        assert_eq!(original_v1, back_to_v1);
    }

    // ========================================================================
    // inject_direct_addr Tests
    // ========================================================================

    #[test]
    fn test_inject_direct_addr() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        // Add two bootstrap peers with no addresses
        ticket.bootstrap.push(BootstrapPeer::new(create_test_endpoint_id(1)));
        ticket.bootstrap.push(BootstrapPeer::new(create_test_endpoint_id(2)));

        // Inject a direct address
        let injected_addr = create_test_socket_addr(9999);
        ticket.inject_direct_addr(injected_addr);

        // Both peers should have the injected address
        for peer in &ticket.bootstrap {
            assert!(peer.direct_addrs.contains(&injected_addr));
        }
    }

    #[test]
    fn test_inject_direct_addr_idempotent() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        ticket.bootstrap.push(BootstrapPeer::new(create_test_endpoint_id(1)));

        let addr = create_test_socket_addr(9999);

        // Inject the same address multiple times
        ticket.inject_direct_addr(addr);
        ticket.inject_direct_addr(addr);
        ticket.inject_direct_addr(addr);

        // Should only appear once
        assert_eq!(ticket.bootstrap[0].direct_addrs.len(), 1);
        assert!(ticket.bootstrap[0].direct_addrs.contains(&addr));
    }

    #[test]
    fn test_inject_direct_addr_preserves_existing() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let existing_addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[existing_addr]);

        let mut ticket = AspenClusterTicketV2::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);

        // Inject a new address
        let new_addr = create_test_socket_addr(9999);
        ticket.inject_direct_addr(new_addr);

        // Both addresses should be present
        assert_eq!(ticket.bootstrap[0].direct_addrs.len(), 2);
        assert!(ticket.bootstrap[0].direct_addrs.contains(&existing_addr));
        assert!(ticket.bootstrap[0].direct_addrs.contains(&new_addr));
    }

    #[test]
    fn test_inject_direct_addr_empty_bootstrap() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        // Inject address with no bootstrap peers (should be a no-op)
        let addr = create_test_socket_addr(9999);
        ticket.inject_direct_addr(addr);

        // No crash, no peers
        assert!(ticket.bootstrap.is_empty());
    }

    // ========================================================================
    // parse_ticket_to_addrs Tests
    // ========================================================================

    #[test]
    fn test_parse_ticket_to_addrs_v1() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let peer_id1 = create_test_endpoint_id(1);
        let peer_id2 = create_test_endpoint_id(2);

        let mut ticket = AspenClusterTicket::new(topic_id, "test-cluster".into());
        ticket.add_bootstrap(peer_id1).unwrap();
        ticket.add_bootstrap(peer_id2).unwrap();

        let ticket_str = ticket.serialize();
        let (parsed_topic, parsed_cluster, addrs) = parse_ticket_to_addrs(&ticket_str).unwrap();

        assert_eq!(parsed_topic, topic_id);
        assert_eq!(parsed_cluster, "test-cluster");
        assert_eq!(addrs.len(), 2);

        // V1 tickets have empty address sets
        for addr in &addrs {
            assert!(addr.addrs.is_empty());
        }

        // Endpoint IDs should match
        let ids: BTreeSet<_> = addrs.iter().map(|a| a.id).collect();
        assert!(ids.contains(&peer_id1));
        assert!(ids.contains(&peer_id2));
    }

    #[test]
    fn test_parse_ticket_to_addrs_v2() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let socket_addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[socket_addr]);

        let ticket = AspenClusterTicketV2::with_bootstrap_addr(topic_id, "test-cluster".into(), &endpoint_addr);

        let ticket_str = ticket.serialize();
        let (parsed_topic, parsed_cluster, addrs) = parse_ticket_to_addrs(&ticket_str).unwrap();

        assert_eq!(parsed_topic, topic_id);
        assert_eq!(parsed_cluster, "test-cluster");
        assert_eq!(addrs.len(), 1);

        // V2 tickets include socket addresses
        assert!(addrs[0].addrs.contains(&TransportAddr::Ip(socket_addr)));
    }

    #[test]
    fn test_parse_ticket_to_addrs_v2_multiple_peers() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);
        let endpoint_addr1 = create_test_endpoint_addr(1, &[addr1]);
        let endpoint_addr2 = create_test_endpoint_addr(2, &[addr2]);

        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());
        ticket.add_bootstrap_addr(&endpoint_addr1).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr2).unwrap();

        let ticket_str = ticket.serialize();
        let (_, _, addrs) = parse_ticket_to_addrs(&ticket_str).unwrap();

        assert_eq!(addrs.len(), 2);
    }

    #[test]
    fn test_parse_ticket_to_addrs_invalid_prefix() {
        let result = parse_ticket_to_addrs("invalid_prefix_ticket");
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("invalid ticket format"));
    }

    #[test]
    fn test_parse_ticket_to_addrs_empty_string() {
        let result = parse_ticket_to_addrs("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ticket_to_addrs_empty_bootstrap() {
        // V1 empty
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let v1 = AspenClusterTicket::new(topic_id, "empty".into());
        let (_, _, addrs) = parse_ticket_to_addrs(&v1.serialize()).unwrap();
        assert!(addrs.is_empty());

        // V2 empty
        let v2 = AspenClusterTicketV2::new(topic_id, "empty".into());
        let (_, _, addrs) = parse_ticket_to_addrs(&v2.serialize()).unwrap();
        assert!(addrs.is_empty());
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_ticket_v2_with_ipv6_address() {
        use std::net::IpAddr;
        use std::net::Ipv6Addr;

        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), 7777);
        let endpoint_id = create_test_endpoint_id(1);

        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());
        ticket.bootstrap.push(BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![ipv6_addr],
        });

        // Serialize and deserialize
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicketV2::deserialize(&serialized).unwrap();

        assert!(deserialized.bootstrap[0].direct_addrs.contains(&ipv6_addr));
    }

    #[test]
    fn test_ticket_v2_mixed_ipv4_ipv6() {
        use std::net::IpAddr;
        use std::net::Ipv4Addr;
        use std::net::Ipv6Addr;

        let topic_id = TopicId::from_bytes([23u8; 32]);
        let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 7777);
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), 7777);
        let endpoint_id = create_test_endpoint_id(1);

        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());
        ticket.bootstrap.push(BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![ipv4_addr, ipv6_addr],
        });

        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicketV2::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.bootstrap[0].direct_addrs.len(), 2);
        assert!(deserialized.bootstrap[0].direct_addrs.contains(&ipv4_addr));
        assert!(deserialized.bootstrap[0].direct_addrs.contains(&ipv6_addr));
    }

    #[test]
    fn test_ticket_long_cluster_id() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let long_cluster_id = "a".repeat(1000);

        let ticket = AspenClusterTicket::new(topic_id, long_cluster_id.clone());
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.cluster_id, long_cluster_id);
    }

    #[test]
    fn test_ticket_v2_long_cluster_id() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let long_cluster_id = "a".repeat(1000);

        let ticket = AspenClusterTicketV2::new(topic_id, long_cluster_id.clone());
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicketV2::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.cluster_id, long_cluster_id);
    }

    #[test]
    fn test_ticket_unicode_cluster_id() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let unicode_cluster_id = "test-cluster-";

        let ticket = AspenClusterTicket::new(topic_id, unicode_cluster_id.into());
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicket::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.cluster_id, unicode_cluster_id);
    }

    #[test]
    fn test_ticket_v2_unicode_cluster_id() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let unicode_cluster_id = "test-cluster-";

        let ticket = AspenClusterTicketV2::new(topic_id, unicode_cluster_id.into());
        let serialized = ticket.serialize();
        let deserialized = AspenClusterTicketV2::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.cluster_id, unicode_cluster_id);
    }

    #[test]
    fn test_bootstrap_peer_equality() {
        let endpoint_id = create_test_endpoint_id(1);
        let addr1 = create_test_socket_addr(7777);
        let addr2 = create_test_socket_addr(8888);

        let peer1 = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr1, addr2],
        };

        let peer2 = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr1, addr2],
        };

        // Same content = equal
        assert_eq!(peer1, peer2);

        // Different order = not equal (Vec comparison)
        let peer3 = BootstrapPeer {
            endpoint_id,
            direct_addrs: vec![addr2, addr1],
        };
        assert_ne!(peer1, peer3);
    }

    #[test]
    fn test_ticket_v2_duplicate_bootstrap_peers() {
        let topic_id = TopicId::from_bytes([23u8; 32]);
        let mut ticket = AspenClusterTicketV2::new(topic_id, "test-cluster".into());

        let addr = create_test_socket_addr(7777);
        let endpoint_addr = create_test_endpoint_addr(1, &[addr]);

        // Add the same peer twice
        ticket.add_bootstrap_addr(&endpoint_addr).unwrap();
        ticket.add_bootstrap_addr(&endpoint_addr).unwrap();

        // Both should be added (Vec allows duplicates, unlike V1's BTreeSet)
        assert_eq!(ticket.bootstrap.len(), 2);

        // endpoint_ids() uses BTreeSet, so duplicates are merged
        let ids = ticket.endpoint_ids();
        assert_eq!(ids.len(), 1);
    }
}
