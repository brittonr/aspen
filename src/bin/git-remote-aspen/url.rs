//! URL parsing for aspen:// URLs.
//!
//! Supported formats:
//! - `aspen://<ticket>/<repo_id>` - Connect via unsigned cluster ticket
//! - `aspen://<signed_ticket>/<repo_id>` - Connect via signed cluster ticket
//! - `aspen://<node_id>/<repo_id>` - Direct node connection (52-char base32 node ID)

use aspen::cluster::ticket::{AspenClusterTicket, SignedAspenClusterTicket};
use aspen::forge::git::bridge::{BridgeError, BridgeResult};
use aspen::forge::identity::RepoId;

/// Parsed aspen:// URL.
#[derive(Debug, Clone)]
pub struct AspenUrl {
    /// Connection target (ticket or node ID).
    pub target: ConnectionTarget,
    /// Repository ID.
    pub repo_id: RepoId,
}

/// Connection target type.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConnectionTarget {
    /// Connect via unsigned cluster ticket.
    Ticket(AspenClusterTicket),
    /// Connect via signed cluster ticket (verified).
    SignedTicket(SignedAspenClusterTicket),
    /// Direct connection to a node via node ID (52-char base32).
    NodeId(String),
}

impl AspenUrl {
    /// Parse an aspen:// URL.
    ///
    /// Format: `aspen://<target>/<repo_id>`
    ///
    /// Target can be:
    /// - A cluster ticket starting with "aspen" (unsigned)
    /// - A signed cluster ticket starting with "aspensigned"
    /// - A node ID (52 character base32-encoded public key)
    pub fn parse(url: &str) -> BridgeResult<Self> {
        // Strip scheme
        let rest = url.strip_prefix("aspen://").ok_or_else(|| BridgeError::InvalidUrl { url: url.to_string() })?;

        // Split into target and repo_id
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(BridgeError::InvalidUrl { url: url.to_string() });
        }

        let target_str = parts[0];
        let repo_id_str = parts[1];

        // Determine target type by prefix
        let target = if target_str.starts_with("aspensigned") {
            // Signed cluster ticket
            let signed_ticket =
                SignedAspenClusterTicket::deserialize(target_str).map_err(|e| BridgeError::InvalidUrl {
                    url: format!("invalid signed ticket: {e}"),
                })?;

            // Verify the signature
            if signed_ticket.verify().is_none() {
                return Err(BridgeError::InvalidUrl {
                    url: "signed ticket verification failed".to_string(),
                });
            }

            ConnectionTarget::SignedTicket(signed_ticket)
        } else if target_str.starts_with("aspen") {
            // Unsigned cluster ticket
            let ticket = AspenClusterTicket::deserialize(target_str).map_err(|e| BridgeError::InvalidUrl {
                url: format!("invalid ticket: {e}"),
            })?;
            ConnectionTarget::Ticket(ticket)
        } else {
            // Assume it's a node ID (52 chars base32)
            // Node IDs are 52-char base32-encoded Ed25519 public keys
            if target_str.len() != 52 {
                return Err(BridgeError::InvalidUrl {
                    url: format!("node ID must be 52 characters, got {}", target_str.len()),
                });
            }
            ConnectionTarget::NodeId(target_str.to_string())
        };

        // Parse repo ID (hex-encoded BLAKE3 hash)
        let repo_id = RepoId::from_hex(repo_id_str).map_err(|_| BridgeError::InvalidUrl { url: url.to_string() })?;

        Ok(Self { target, repo_id })
    }

    /// Get the repository ID.
    pub fn repo_id(&self) -> &RepoId {
        &self.repo_id
    }

    /// Get the cluster ticket (if using ticket-based connection).
    #[allow(dead_code)]
    pub fn ticket(&self) -> Option<&AspenClusterTicket> {
        match &self.target {
            ConnectionTarget::Ticket(t) => Some(t),
            ConnectionTarget::SignedTicket(s) => Some(&s.ticket),
            ConnectionTarget::NodeId(_) => None,
        }
    }

    /// Check if this URL uses a signed ticket.
    #[allow(dead_code)]
    pub fn is_signed(&self) -> bool {
        matches!(&self.target, ConnectionTarget::SignedTicket(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh_gossip::proto::TopicId;

    fn create_test_ticket() -> AspenClusterTicket {
        let topic_id = TopicId::from_bytes([42u8; 32]);
        AspenClusterTicket::new(topic_id, "test-cluster".into())
    }

    #[test]
    fn test_parse_ticket_url() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let repo_id_hex = "a".repeat(64);
        let url = format!("aspen://{}/{}", ticket_str, repo_id_hex);

        let parsed = AspenUrl::parse(&url).unwrap();

        match &parsed.target {
            ConnectionTarget::Ticket(t) => {
                assert_eq!(t.cluster_id, "test-cluster");
            }
            _ => panic!("expected Ticket"),
        }
    }

    #[test]
    fn test_parse_node_url() {
        // 52-char base32 node ID
        let node_id = "a".repeat(52);
        let repo_id_hex = "b".repeat(64);
        let url = format!("aspen://{}/{}", node_id, repo_id_hex);

        let parsed = AspenUrl::parse(&url).unwrap();

        match &parsed.target {
            ConnectionTarget::NodeId(id) => assert_eq!(id, &node_id),
            _ => panic!("expected NodeId"),
        }
    }

    #[test]
    fn test_parse_invalid_scheme() {
        let result = AspenUrl::parse("https://example.com/repo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_repo_id() {
        let result = AspenUrl::parse("aspen://nodeabcd");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_node_id_length() {
        // Wrong length for node ID (not 52 chars, not a ticket)
        let repo_id_hex = "a".repeat(64);
        let url = format!("aspen://short123/{}", repo_id_hex);

        let result = AspenUrl::parse(&url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_ticket() {
        let repo_id_hex = "a".repeat(64);
        // Invalid ticket that starts with "aspen" but isn't valid
        let url = format!("aspen://aspen_invalid_data/{}", repo_id_hex);

        let result = AspenUrl::parse(&url);
        assert!(result.is_err());
    }
}
