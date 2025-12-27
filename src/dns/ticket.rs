//! DNS client tickets for sharing read-only access.
//!
//! Provides compact, URL-safe tickets for DNS client connections.
//! Follows the iroh-tickets pattern used by AspenDocsTicket.

use anyhow::Context;
use anyhow::Result;
use iroh::EndpointAddr;
use iroh_tickets::Ticket;
use serde::Deserialize;
use serde::Serialize;

use super::constants::DNS_TICKET_PREFIX;

/// DNS client ticket for read-only synchronization.
///
/// Contains all information needed to connect to an Aspen cluster
/// and synchronize DNS records via iroh-docs:
/// - `cluster_id`: Cluster identifier for tracking
/// - `namespace_id`: The iroh-docs namespace containing DNS records
/// - `priority`: Priority for sync scheduling (0 = highest)
/// - `peers`: Bootstrap peer addresses for initial connection
/// - `zone_filter`: Optional list of zones to sync (None = all zones)
///
/// Tiger Style: Bounded peer list, explicit zone filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsClientTicket {
    /// Cluster identifier (typically the cluster cookie hash).
    pub cluster_id: String,
    /// Priority for overlay subscription (0 = highest).
    pub priority: u8,
    /// Namespace ID (hex-encoded).
    pub namespace_id: String,
    /// Bootstrap peer addresses.
    pub peers: Vec<EndpointAddr>,
    /// Optional zone filter (None = all DNS records).
    ///
    /// If specified, only records for these zones will be synced.
    /// Example: `["example.com", "internal.example.com"]`
    pub zone_filter: Option<Vec<String>>,
}

impl DnsClientTicket {
    /// Maximum number of bootstrap peers in a ticket.
    ///
    /// Tiger Style: Fixed limit to prevent unbounded ticket size.
    pub const MAX_BOOTSTRAP_PEERS: u32 = 16;

    /// Maximum number of zones in filter.
    ///
    /// Tiger Style: Fixed limit to prevent unbounded filtering.
    pub const MAX_ZONE_FILTER: u32 = 100;

    /// Create a new DNS client ticket.
    ///
    /// # Arguments
    /// * `cluster_id` - Cluster identifier
    /// * `namespace_id` - The iroh-docs namespace ID (hex-encoded)
    /// * `peers` - Bootstrap peer addresses
    pub fn new(cluster_id: impl Into<String>, namespace_id: impl Into<String>, peers: Vec<EndpointAddr>) -> Self {
        Self {
            cluster_id: cluster_id.into(),
            priority: 0,
            namespace_id: namespace_id.into(),
            peers,
            zone_filter: None,
        }
    }

    /// Set the priority for this ticket.
    ///
    /// Lower numbers = higher priority (0 is highest).
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set a zone filter for this ticket.
    ///
    /// Only records for the specified zones will be synced.
    ///
    /// # Errors
    /// Returns error if more than MAX_ZONE_FILTER zones are specified.
    pub fn with_zone_filter(mut self, zones: Vec<String>) -> Result<Self> {
        if zones.len() > Self::MAX_ZONE_FILTER as usize {
            anyhow::bail!("zone filter exceeds maximum ({} > {})", zones.len(), Self::MAX_ZONE_FILTER);
        }
        self.zone_filter = Some(zones);
        Ok(self)
    }

    /// Add a bootstrap peer to the ticket.
    ///
    /// # Errors
    /// Returns error if the maximum number of peers is reached.
    pub fn add_peer(&mut self, peer: EndpointAddr) -> Result<()> {
        if self.peers.len() >= Self::MAX_BOOTSTRAP_PEERS as usize {
            anyhow::bail!("cannot add more than {} bootstrap peers", Self::MAX_BOOTSTRAP_PEERS);
        }
        self.peers.push(peer);
        Ok(())
    }

    /// Serialize the ticket to a URL-safe string.
    ///
    /// Format: `aspendns{base32-encoded-postcard-payload}`
    pub fn serialize(&self) -> String {
        <Self as Ticket>::serialize(self)
    }

    /// Deserialize a ticket from a URL-safe string.
    ///
    /// # Errors
    /// Returns error if the string is not a valid DNS client ticket.
    pub fn deserialize(input: &str) -> Result<Self> {
        <Self as Ticket>::deserialize(input).context("failed to deserialize DNS client ticket")
    }

    /// Check if a domain should be synced based on the zone filter.
    ///
    /// Returns true if:
    /// - No zone filter is set (sync all)
    /// - The domain matches one of the filtered zones
    /// - The domain is a subdomain of a filtered zone
    pub fn should_sync_domain(&self, domain: &str) -> bool {
        match &self.zone_filter {
            None => true,
            Some(zones) => {
                let domain_lower = domain.to_lowercase();
                zones.iter().any(|zone| {
                    let zone_lower = zone.to_lowercase();
                    // Exact match
                    domain_lower == zone_lower
                        // Subdomain match (domain ends with .zone)
                        || domain_lower.ends_with(&format!(".{}", zone_lower))
                })
            }
        }
    }
}

impl Ticket for DnsClientTicket {
    const KIND: &'static str = DNS_TICKET_PREFIX;

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect(
            "DnsClientTicket postcard serialization failed - \
             indicates library bug or memory corruption",
        )
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

    fn create_test_endpoint_addr(seed: u8) -> EndpointAddr {
        let secret_key = SecretKey::from([seed; 32]);
        let endpoint_id: EndpointId = secret_key.public();
        EndpointAddr::new(endpoint_id)
    }

    #[test]
    fn test_ticket_new() {
        let ticket = DnsClientTicket::new("test-cluster", "abc123", vec![]);

        assert_eq!(ticket.cluster_id, "test-cluster");
        assert_eq!(ticket.namespace_id, "abc123");
        assert_eq!(ticket.priority, 0);
        assert!(ticket.peers.is_empty());
        assert!(ticket.zone_filter.is_none());
    }

    #[test]
    fn test_ticket_with_priority() {
        let ticket = DnsClientTicket::new("test-cluster", "abc123", vec![]).with_priority(5);

        assert_eq!(ticket.priority, 5);
    }

    #[test]
    fn test_ticket_with_zone_filter() {
        let ticket = DnsClientTicket::new("test-cluster", "abc123", vec![])
            .with_zone_filter(vec!["example.com".to_string(), "internal.com".to_string()])
            .unwrap();

        assert_eq!(ticket.zone_filter, Some(vec!["example.com".to_string(), "internal.com".to_string()]));
    }

    #[test]
    fn test_ticket_zone_filter_limit() {
        let zones: Vec<String> = (0..=DnsClientTicket::MAX_ZONE_FILTER).map(|i| format!("zone{}.com", i)).collect();

        let result = DnsClientTicket::new("test-cluster", "abc123", vec![]).with_zone_filter(zones);

        assert!(result.is_err());
    }

    #[test]
    fn test_ticket_add_peer() {
        let mut ticket = DnsClientTicket::new("test-cluster", "abc123", vec![]);
        let peer = create_test_endpoint_addr(1);

        ticket.add_peer(peer.clone()).unwrap();

        assert_eq!(ticket.peers.len(), 1);
    }

    #[test]
    fn test_ticket_add_peer_limit() {
        let mut ticket = DnsClientTicket::new("test-cluster", "abc123", vec![]);

        for i in 0..DnsClientTicket::MAX_BOOTSTRAP_PEERS {
            let peer = create_test_endpoint_addr(i as u8);
            ticket.add_peer(peer).unwrap();
        }

        // One more should fail
        let extra_peer = create_test_endpoint_addr(255);
        let result = ticket.add_peer(extra_peer);
        assert!(result.is_err());
    }

    #[test]
    fn test_ticket_roundtrip() {
        let peer = create_test_endpoint_addr(1);
        let ticket = DnsClientTicket::new("test-cluster", "namespace123", vec![peer])
            .with_priority(3)
            .with_zone_filter(vec!["example.com".to_string()])
            .unwrap();

        let serialized = ticket.serialize();
        assert!(serialized.starts_with(DNS_TICKET_PREFIX));

        let parsed = DnsClientTicket::deserialize(&serialized).unwrap();
        assert_eq!(parsed, ticket);
    }

    #[test]
    fn test_ticket_invalid_deserialize() {
        assert!(DnsClientTicket::deserialize("invalid").is_err());
        assert!(DnsClientTicket::deserialize("aspendns!!!").is_err());
        assert!(DnsClientTicket::deserialize("").is_err());
    }

    #[test]
    fn test_should_sync_domain_no_filter() {
        let ticket = DnsClientTicket::new("test-cluster", "abc123", vec![]);

        assert!(ticket.should_sync_domain("anything.com"));
        assert!(ticket.should_sync_domain("sub.domain.example.org"));
    }

    #[test]
    fn test_should_sync_domain_with_filter() {
        let ticket = DnsClientTicket::new("test-cluster", "abc123", vec![])
            .with_zone_filter(vec!["example.com".to_string(), "internal.net".to_string()])
            .unwrap();

        // Exact match
        assert!(ticket.should_sync_domain("example.com"));
        assert!(ticket.should_sync_domain("internal.net"));

        // Subdomain match
        assert!(ticket.should_sync_domain("api.example.com"));
        assert!(ticket.should_sync_domain("deep.sub.example.com"));
        assert!(ticket.should_sync_domain("srv.internal.net"));

        // Non-matching domains
        assert!(!ticket.should_sync_domain("other.com"));
        assert!(!ticket.should_sync_domain("notexample.com"));
        assert!(!ticket.should_sync_domain("example.org"));
    }

    #[test]
    fn test_should_sync_domain_case_insensitive() {
        let ticket = DnsClientTicket::new("test-cluster", "abc123", vec![])
            .with_zone_filter(vec!["Example.COM".to_string()])
            .unwrap();

        assert!(ticket.should_sync_domain("example.com"));
        assert!(ticket.should_sync_domain("EXAMPLE.COM"));
        assert!(ticket.should_sync_domain("Sub.Example.Com"));
    }
}
