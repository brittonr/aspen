//! URL parsing for aspen:// URLs.
//!
//! Supported formats:
//! - `aspen://<ticket>/<repo_id>` - Connect via unsigned cluster ticket
//! - `aspen://<signed_ticket>/<repo_id>` - Connect via signed cluster ticket
//! - `aspen://<node_id>/<repo_id>` - Direct node connection (52-char base32 node ID)

use aspen::cluster::ticket::AspenClusterTicket;
use aspen::cluster::ticket::SignedAspenClusterTicket;
use aspen::forge::git::bridge::BridgeError;
use aspen::forge::git::bridge::BridgeResult;
use aspen::forge::identity::RepoId;

/// Parsed aspen:// URL.
#[derive(Debug, Clone)]
pub struct AspenUrl {
    /// Connection target (ticket or node ID).
    pub target: ConnectionTarget,
    /// Repository ID (for direct repos) or deterministic mirror ID (for federated repos).
    pub repo_id: RepoId,
    /// If present, this is a federated clone through the local cluster's federation layer.
    pub federated: Option<FederatedTarget>,
}

/// Connection target type.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ConnectionTarget {
    /// Connect via unsigned cluster ticket (includes direct addresses for relay-less connectivity).
    Ticket(AspenClusterTicket),
    /// Connect via signed cluster ticket (verified).
    SignedTicket(SignedAspenClusterTicket),
    /// Direct connection to a node via node ID (52-char base32).
    NodeId(String),
}

/// A federated repo target: the repo lives on a remote cluster, accessed via the local cluster's
/// federation layer.
#[derive(Debug, Clone)]
pub struct FederatedTarget {
    /// Public key of the origin cluster (base32-encoded, 52 chars).
    pub origin_key: String,
    /// Repo ID on the origin cluster (hex-encoded BLAKE3 hash, 64 chars).
    pub upstream_repo_id: String,
    /// Optional direct socket address hint for the origin cluster (e.g., "192.168.1.1:54866").
    /// Parsed from `ASPEN_ORIGIN_ADDR` env var at URL parse time.
    pub origin_addr_hint: Option<String>,
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
        // Order matters: check longer prefixes first (aspensigned) before shorter (aspen)
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
            // Unsigned cluster ticket (includes direct addresses for relay-less connectivity)
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

        // Check for federated repo: "fed:<origin-base32>:<repo-hex>"
        if let Some(fed_rest) = repo_id_str.strip_prefix("fed:") {
            let fed_parts: Vec<&str> = fed_rest.splitn(2, ':').collect();
            if fed_parts.len() != 2 {
                return Err(BridgeError::InvalidUrl {
                    url: format!("federated ID must be fed:<origin-key>:<repo-hex>, got: {repo_id_str}"),
                });
            }

            let origin_key = fed_parts[0];
            let upstream_repo_hex = fed_parts[1];

            // Validate origin key: accept base32 (52 chars) or hex (64 chars)
            if origin_key.len() != 52 && origin_key.len() != 64 {
                return Err(BridgeError::InvalidUrl {
                    url: format!("origin key must be 52 (base32) or 64 (hex) characters, got {}", origin_key.len()),
                });
            }

            // Validate upstream repo ID (64-char hex)
            if upstream_repo_hex.len() != 64 || !upstream_repo_hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(BridgeError::InvalidUrl {
                    url: format!("upstream repo ID must be 64 hex characters, got: {upstream_repo_hex}"),
                });
            }

            // Derive deterministic mirror repo ID: blake3(origin_key_bytes || repo_id_bytes)
            let origin_key_bytes = origin_key.as_bytes();
            let upstream_bytes =
                hex::decode(upstream_repo_hex).map_err(|_| BridgeError::InvalidUrl { url: url.to_string() })?;
            let mut hasher = blake3::Hasher::new();
            hasher.update(origin_key_bytes);
            hasher.update(&upstream_bytes);
            let mirror_hash = hasher.finalize();
            let repo_id = RepoId::from_hash(mirror_hash);

            // Check for origin address hint from environment
            let origin_addr_hint = std::env::var("ASPEN_ORIGIN_ADDR").ok();

            let federated = FederatedTarget {
                origin_key: origin_key.to_string(),
                upstream_repo_id: upstream_repo_hex.to_string(),
                origin_addr_hint,
            };

            return Ok(Self {
                target,
                repo_id,
                federated: Some(federated),
            });
        }

        // Parse repo ID (hex-encoded BLAKE3 hash)
        let repo_id = RepoId::from_hex(repo_id_str).map_err(|_| BridgeError::InvalidUrl { url: url.to_string() })?;

        Ok(Self {
            target,
            repo_id,
            federated: None,
        })
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

    /// Check if this is a federated URL (repo on a remote cluster).
    #[cfg(test)]
    pub fn is_federated(&self) -> bool {
        self.federated.is_some()
    }
}

#[cfg(test)]
mod tests {
    use iroh_gossip::proto::TopicId;

    use super::*;

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

    // ========================================================================
    // Federated URL tests
    // ========================================================================

    #[test]
    fn test_parse_federated_url() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let origin_key = "a".repeat(52); // 52-char base32 public key
        let repo_hex = "bb".repeat(32); // 64-char hex repo ID
        let url = format!("aspen://{}/fed:{}:{}", ticket_str, origin_key, repo_hex);

        let parsed = AspenUrl::parse(&url).unwrap();

        assert!(parsed.is_federated());
        let fed = parsed.federated.as_ref().unwrap();
        assert_eq!(fed.origin_key, origin_key);
        assert_eq!(fed.upstream_repo_id, repo_hex);

        // Mirror repo ID should be deterministic
        let mut hasher = blake3::Hasher::new();
        hasher.update(origin_key.as_bytes());
        hasher.update(&hex::decode(&repo_hex).unwrap());
        let expected_mirror = hasher.finalize();
        assert_eq!(parsed.repo_id().0, *expected_mirror.as_bytes());
    }

    #[test]
    fn test_parse_federated_deterministic_mirror_id() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let origin_key = "b".repeat(52);
        let repo_hex = "cc".repeat(32);
        let url = format!("aspen://{}/fed:{}:{}", ticket_str, origin_key, repo_hex);

        let parsed1 = AspenUrl::parse(&url).unwrap();
        let parsed2 = AspenUrl::parse(&url).unwrap();

        // Same inputs → same mirror repo ID
        assert_eq!(parsed1.repo_id().to_hex(), parsed2.repo_id().to_hex());
    }

    #[test]
    fn test_parse_federated_different_origin_different_mirror() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let repo_hex = "dd".repeat(32);

        let url1 = format!("aspen://{}/fed:{}:{}", ticket_str, "a".repeat(52), repo_hex);
        let url2 = format!("aspen://{}/fed:{}:{}", ticket_str, "b".repeat(52), repo_hex);

        let parsed1 = AspenUrl::parse(&url1).unwrap();
        let parsed2 = AspenUrl::parse(&url2).unwrap();

        // Different origin keys → different mirror repo IDs
        assert_ne!(parsed1.repo_id().to_hex(), parsed2.repo_id().to_hex());
    }

    #[test]
    fn test_parse_federated_malformed_missing_repo() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let origin_key = "a".repeat(52);
        let url = format!("aspen://{}/fed:{}", ticket_str, origin_key);

        let result = AspenUrl::parse(&url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_federated_malformed_short_origin_key() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let repo_hex = "aa".repeat(32);
        let url = format!("aspen://{}/fed:short:{}", ticket_str, repo_hex);

        let result = AspenUrl::parse(&url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_federated_malformed_bad_repo_hex() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let origin_key = "a".repeat(52);
        let url = format!("aspen://{}/fed:{}:not_hex_at_all", ticket_str, origin_key);

        let result = AspenUrl::parse(&url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_federated_hex_origin_key() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let origin_key = "ab".repeat(32); // 64-char hex public key
        let repo_hex = "cc".repeat(32);
        let url = format!("aspen://{}/fed:{}:{}", ticket_str, origin_key, repo_hex);

        let parsed = AspenUrl::parse(&url).unwrap();
        assert!(parsed.is_federated());
        let fed = parsed.federated.as_ref().unwrap();
        assert_eq!(fed.origin_key, origin_key);
    }

    #[test]
    fn test_parse_non_federated_unchanged() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let repo_id_hex = "ee".repeat(32);
        let url = format!("aspen://{}/{}", ticket_str, repo_id_hex);

        let parsed = AspenUrl::parse(&url).unwrap();

        assert!(!parsed.is_federated());
        assert!(parsed.federated.is_none());
        assert_eq!(parsed.repo_id().to_hex(), repo_id_hex);
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

    #[test]
    fn test_parse_invalid_repo_id_hex() {
        // Valid 52-char node ID but invalid repo ID (not valid hex)
        let node_id = "a".repeat(52);
        let url = format!("aspen://{}/not_valid_hex", node_id);

        let result = AspenUrl::parse(&url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_repo_id_wrong_length() {
        // Valid node ID, repo ID that's hex but wrong length (not 64 chars)
        let node_id = "a".repeat(52);
        let url = format!("aspen://{}/abcdef", node_id);

        let result = AspenUrl::parse(&url);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_url() {
        let result = AspenUrl::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_just_scheme() {
        let result = AspenUrl::parse("aspen://");
        assert!(result.is_err());
    }

    #[test]
    fn test_repo_id_accessor() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let repo_id_hex = "c".repeat(64);
        let url = format!("aspen://{}/{}", ticket_str, repo_id_hex);

        let parsed = AspenUrl::parse(&url).unwrap();
        assert_eq!(parsed.repo_id().to_hex(), repo_id_hex);
    }

    #[test]
    fn test_ticket_accessor() {
        let ticket = create_test_ticket();
        let ticket_str = ticket.serialize();
        let repo_id_hex = "d".repeat(64);
        let url = format!("aspen://{}/{}", ticket_str, repo_id_hex);

        let parsed = AspenUrl::parse(&url).unwrap();
        assert!(parsed.ticket().is_some());
        assert!(!parsed.is_signed());
    }

    #[test]
    fn test_node_id_has_no_ticket() {
        let node_id = "a".repeat(52);
        let repo_id_hex = "e".repeat(64);
        let url = format!("aspen://{}/{}", node_id, repo_id_hex);

        let parsed = AspenUrl::parse(&url).unwrap();
        assert!(parsed.ticket().is_none());
        assert!(!parsed.is_signed());
    }
}
