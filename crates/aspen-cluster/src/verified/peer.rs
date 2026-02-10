//! Pure peer address parsing functions.
//!
//! This module contains pure functions for parsing peer specifications.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - No I/O operations
//! - Explicit error types
//! - Deterministic behavior for testing and verification

// ============================================================================
// Peer Spec Parsing
// ============================================================================

/// Result of parsing a peer specification string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerSpecParseResult<'a> {
    /// Successfully parsed peer spec.
    Ok {
        /// The node ID (Raft logical ID)
        node_id: u64,
        /// The endpoint spec (could be endpoint ID or JSON)
        endpoint_spec: &'a str,
    },
    /// Missing '@' separator in peer spec.
    MissingSeparator,
    /// Invalid node ID (not a valid u64).
    InvalidNodeId,
    /// Empty node ID part.
    EmptyNodeId,
    /// Empty endpoint spec part.
    EmptyEndpointSpec,
}

impl<'a> PeerSpecParseResult<'a> {
    /// Check if parsing was successful.
    #[inline]
    pub fn is_ok(&self) -> bool {
        matches!(self, PeerSpecParseResult::Ok { .. })
    }
}

/// Parse a peer specification string into node ID and endpoint spec.
///
/// Peer specs have the format: `node_id@endpoint_spec`
///
/// Where:
/// - `node_id` is a u64 Raft logical ID
/// - `endpoint_spec` is either a bare endpoint ID or JSON endpoint address
///
/// # Arguments
///
/// * `spec` - The peer specification string
///
/// # Returns
///
/// `PeerSpecParseResult` indicating success or failure type.
///
/// # Examples
///
/// ```ignore
/// // Simple endpoint ID
/// match parse_peer_spec("1@abc123") {
///     PeerSpecParseResult::Ok { node_id, endpoint_spec } => {
///         assert_eq!(node_id, 1);
///         assert_eq!(endpoint_spec, "abc123");
///     }
///     _ => panic!("expected success"),
/// }
///
/// // JSON endpoint
/// match parse_peer_spec("2@{\"id\":\"xyz\"}") {
///     PeerSpecParseResult::Ok { node_id, endpoint_spec } => {
///         assert_eq!(node_id, 2);
///         assert!(endpoint_spec.starts_with('{'));
///     }
///     _ => panic!("expected success"),
/// }
/// ```
///
/// # Tiger Style
///
/// - Pure function with no I/O
/// - Explicit error variants for each failure mode
/// - Returns borrowed slice to avoid allocation
#[inline]
pub fn parse_peer_spec(spec: &str) -> PeerSpecParseResult<'_> {
    // Find the '@' separator
    let Some(at_pos) = spec.find('@') else {
        return PeerSpecParseResult::MissingSeparator;
    };

    let node_id_str = &spec[..at_pos];
    let endpoint_spec = &spec[at_pos + 1..];

    // Validate node_id part is not empty
    if node_id_str.is_empty() {
        return PeerSpecParseResult::EmptyNodeId;
    }

    // Validate endpoint_spec part is not empty
    if endpoint_spec.is_empty() {
        return PeerSpecParseResult::EmptyEndpointSpec;
    }

    // Parse node_id as u64
    let Ok(node_id) = node_id_str.parse::<u64>() else {
        return PeerSpecParseResult::InvalidNodeId;
    };

    PeerSpecParseResult::Ok { node_id, endpoint_spec }
}

// ============================================================================
// JSON Detection
// ============================================================================

/// Check if an endpoint string appears to be JSON format.
///
/// This is a quick heuristic check that looks for a leading '{'.
/// Full JSON validation happens during actual parsing.
///
/// # Arguments
///
/// * `endpoint_spec` - The endpoint specification string
///
/// # Returns
///
/// `true` if the string starts with '{', indicating JSON format.
///
/// # Tiger Style
///
/// - Simple heuristic, no parsing
/// - O(1) time complexity
#[inline]
pub fn is_json_endpoint(endpoint_spec: &str) -> bool {
    endpoint_spec.starts_with('{')
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Peer Spec Parsing Tests
    // ========================================================================

    #[test]
    fn test_parse_simple_spec() {
        match parse_peer_spec("1@abc123def") {
            PeerSpecParseResult::Ok { node_id, endpoint_spec } => {
                assert_eq!(node_id, 1);
                assert_eq!(endpoint_spec, "abc123def");
            }
            other => panic!("Expected Ok, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_json_endpoint() {
        match parse_peer_spec("42@{\"id\":\"xyz789\",\"addrs\":[]}") {
            PeerSpecParseResult::Ok { node_id, endpoint_spec } => {
                assert_eq!(node_id, 42);
                assert!(endpoint_spec.starts_with('{'));
                assert!(endpoint_spec.contains("xyz789"));
            }
            other => panic!("Expected Ok, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_large_node_id() {
        match parse_peer_spec("18446744073709551615@endpoint") {
            PeerSpecParseResult::Ok { node_id, endpoint_spec } => {
                assert_eq!(node_id, u64::MAX);
                assert_eq!(endpoint_spec, "endpoint");
            }
            other => panic!("Expected Ok, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_missing_separator() {
        assert_eq!(parse_peer_spec("123endpoint"), PeerSpecParseResult::MissingSeparator);
    }

    #[test]
    fn test_parse_empty_node_id() {
        assert_eq!(parse_peer_spec("@endpoint"), PeerSpecParseResult::EmptyNodeId);
    }

    #[test]
    fn test_parse_empty_endpoint() {
        assert_eq!(parse_peer_spec("123@"), PeerSpecParseResult::EmptyEndpointSpec);
    }

    #[test]
    fn test_parse_invalid_node_id() {
        assert_eq!(parse_peer_spec("not_a_number@endpoint"), PeerSpecParseResult::InvalidNodeId);
    }

    #[test]
    fn test_parse_negative_node_id() {
        assert_eq!(parse_peer_spec("-1@endpoint"), PeerSpecParseResult::InvalidNodeId);
    }

    #[test]
    fn test_parse_node_id_overflow() {
        // u64::MAX + 1
        assert_eq!(parse_peer_spec("18446744073709551616@endpoint"), PeerSpecParseResult::InvalidNodeId);
    }

    #[test]
    fn test_parse_multiple_at_signs() {
        // Should split on first '@'
        match parse_peer_spec("1@endpoint@with@multiple@at") {
            PeerSpecParseResult::Ok { node_id, endpoint_spec } => {
                assert_eq!(node_id, 1);
                assert_eq!(endpoint_spec, "endpoint@with@multiple@at");
            }
            other => panic!("Expected Ok, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_empty_string() {
        assert_eq!(parse_peer_spec(""), PeerSpecParseResult::MissingSeparator);
    }

    #[test]
    fn test_parse_just_at_sign() {
        assert_eq!(parse_peer_spec("@"), PeerSpecParseResult::EmptyNodeId);
    }

    #[test]
    fn test_is_ok_helper() {
        assert!(parse_peer_spec("1@endpoint").is_ok());
        assert!(!parse_peer_spec("invalid").is_ok());
    }

    // ========================================================================
    // JSON Detection Tests
    // ========================================================================

    #[test]
    fn test_is_json_endpoint_true() {
        assert!(is_json_endpoint("{\"id\":\"abc\"}"));
        assert!(is_json_endpoint("{}"));
        assert!(is_json_endpoint("{"));
    }

    #[test]
    fn test_is_json_endpoint_false() {
        assert!(!is_json_endpoint("abc123"));
        assert!(!is_json_endpoint(""));
        assert!(!is_json_endpoint(" {\"id\":\"abc\"}")); // Leading space
        assert!(!is_json_endpoint("[1,2,3]")); // Array, not object
    }

    #[test]
    fn test_is_json_endpoint_whitespace() {
        // Whitespace before brace is not considered JSON
        assert!(!is_json_endpoint(" {}"));
        assert!(!is_json_endpoint("\t{}"));
        assert!(!is_json_endpoint("\n{}"));
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_parse_valid_spec_roundtrip() {
        check!().with_type::<(u64, String)>().for_each(|(node_id, endpoint)| {
            if !endpoint.is_empty() && !endpoint.contains('@') {
                let spec = format!("{}@{}", node_id, endpoint);
                match parse_peer_spec(&spec) {
                    PeerSpecParseResult::Ok {
                        node_id: parsed_id,
                        endpoint_spec,
                    } => {
                        assert_eq!(parsed_id, *node_id);
                        assert_eq!(endpoint_spec, endpoint);
                    }
                    other => panic!("Expected Ok for valid spec, got {:?}", other),
                }
            }
        });
    }

    #[test]
    fn prop_parse_without_at_fails() {
        check!().with_type::<String>().for_each(|s| {
            if !s.contains('@') {
                assert_eq!(parse_peer_spec(s), PeerSpecParseResult::MissingSeparator);
            }
        });
    }

    #[test]
    fn prop_is_json_consistent() {
        check!().with_type::<String>().for_each(|s| {
            let result = is_json_endpoint(s);
            if s.starts_with('{') {
                assert!(result);
            } else {
                assert!(!result);
            }
        });
    }
}
