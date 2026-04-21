//! Cluster cookie validation and key derivation.
//!
//! The cluster cookie is the root shared secret for node authentication.
//! It is used for:
//! - HMAC challenge-response authentication between Raft nodes
//! - Gossip topic derivation (all nodes with the same cookie share a topic)
//!
//! # Tiger Style
//!
//! - All functions are pure (no I/O, no async)
//! - Fail-fast on invalid/unsafe cookies
//! - Explicit error messages guide operators to fix issues

/// The unsafe default cookie marker that indicates no custom cookie was set.
///
/// When this is detected, validation rejects startup because all clusters
/// sharing the default cookie would join the same gossip topic — a security
/// vulnerability.
pub const UNSAFE_DEFAULT_COOKIE: &str = "aspen-cookie-UNSAFE-CHANGE-ME";

/// Maximum cookie length in bytes.
///
/// Tiger Style: Bound all inputs to prevent resource exhaustion.
pub const MAX_COOKIE_LENGTH: usize = 256;

/// Errors from cookie validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieError {
    /// Cookie is empty.
    Empty,
    /// Cookie is the unsafe default value.
    UnsafeDefault,
    /// Cookie exceeds maximum length.
    TooLong { length: usize, max: usize },
}

impl std::fmt::Display for CookieError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CookieError::Empty => write!(f, "cluster cookie cannot be empty"),
            CookieError::UnsafeDefault => write!(
                f,
                "using default cluster cookie is not allowed - all clusters with default cookie \
                 share the same gossip topic. Set a unique cookie via --cookie, ASPEN_COOKIE env var, \
                 or config file"
            ),
            CookieError::TooLong { length, max } => {
                write!(f, "cluster cookie too long: {length} bytes (max: {max})")
            }
        }
    }
}

impl std::error::Error for CookieError {}

/// Validate that a cluster cookie is non-empty.
///
/// # Errors
///
/// Returns `CookieError::Empty` if the cookie is an empty string.
#[inline]
pub fn validate_cookie(cookie: &str) -> Result<(), CookieError> {
    if cookie.is_empty() {
        return Err(CookieError::Empty);
    }
    Ok(())
}

/// Check if the cluster cookie is the unsafe default.
///
/// When using the default cookie, all clusters share the same gossip topic,
/// which is a security vulnerability.
///
/// # Errors
///
/// Returns `CookieError::UnsafeDefault` if the cookie matches `UNSAFE_DEFAULT_COOKIE`.
pub fn validate_cookie_safety(cookie: &str) -> Result<(), CookieError> {
    if cookie == UNSAFE_DEFAULT_COOKIE {
        Err(CookieError::UnsafeDefault)
    } else {
        Ok(())
    }
}

/// Validate cookie length is within bounds.
///
/// # Errors
///
/// Returns `CookieError::TooLong` if the cookie exceeds `MAX_COOKIE_LENGTH`.
pub fn validate_cookie_length(cookie: &str) -> Result<(), CookieError> {
    if cookie.len() > MAX_COOKIE_LENGTH {
        return Err(CookieError::TooLong {
            length: cookie.len(),
            max: MAX_COOKIE_LENGTH,
        });
    }
    Ok(())
}

/// Run all cookie validations: non-empty, not default, within length bounds.
pub fn validate_cookie_full(cookie: &str) -> Result<(), CookieError> {
    validate_cookie(cookie)?;
    validate_cookie_safety(cookie)?;
    validate_cookie_length(cookie)?;
    Ok(())
}

/// Derive an HMAC key from the cluster cookie using BLAKE3.
///
/// This produces a 32-byte key suitable for HMAC-SHA256 authentication.
/// The same cookie always produces the same key (deterministic).
///
/// This is the canonical key derivation — used by `aspen-auth::verified_auth::derive_hmac_key`.
#[inline]
pub fn derive_cookie_hmac_key(cookie: &str) -> [u8; 32] {
    let hash = blake3::hash(cookie.as_bytes());
    *hash.as_bytes()
}

/// Derive a gossip topic identifier from the cluster cookie.
///
/// Nodes with the same cookie will join the same gossip topic.
/// Uses BLAKE3 with a domain separator to avoid key reuse with HMAC derivation.
#[inline]
pub fn derive_gossip_topic(cookie: &str) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"aspen-gossip-topic-v1:");
    hasher.update(cookie.as_bytes());
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_cookie_rejected() {
        assert_eq!(validate_cookie(""), Err(CookieError::Empty));
    }

    #[test]
    fn test_valid_cookie_accepted() {
        assert!(validate_cookie("my-cluster").is_ok());
        assert!(validate_cookie("x").is_ok());
    }

    #[test]
    fn test_unsafe_default_rejected() {
        assert_eq!(validate_cookie_safety(UNSAFE_DEFAULT_COOKIE), Err(CookieError::UnsafeDefault));
    }

    #[test]
    fn test_custom_cookie_accepted() {
        assert!(validate_cookie_safety("my-unique-cluster-cookie").is_ok());
        assert!(validate_cookie_safety("production-cluster-2025").is_ok());
    }

    #[test]
    fn test_cookie_too_long() {
        let long_cookie = "a".repeat(MAX_COOKIE_LENGTH.saturating_add(1));
        assert!(matches!(validate_cookie_length(&long_cookie), Err(CookieError::TooLong { .. })));
    }

    #[test]
    fn test_cookie_at_max_length() {
        let max_cookie = "a".repeat(MAX_COOKIE_LENGTH);
        assert!(validate_cookie_length(&max_cookie).is_ok());
    }

    #[test]
    fn test_derive_hmac_key_deterministic() {
        let key1 = derive_cookie_hmac_key("test-cookie");
        let key2 = derive_cookie_hmac_key("test-cookie");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_hmac_key_different_cookies() {
        let key1 = derive_cookie_hmac_key("cookie-1");
        let key2 = derive_cookie_hmac_key("cookie-2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_gossip_topic_deterministic() {
        let topic1 = derive_gossip_topic("test-cookie");
        let topic2 = derive_gossip_topic("test-cookie");
        assert_eq!(topic1, topic2);
    }

    #[test]
    fn test_gossip_topic_differs_from_hmac_key() {
        let hmac_key = derive_cookie_hmac_key("test-cookie");
        let gossip_topic = derive_gossip_topic("test-cookie");
        // Domain separation ensures these are different
        assert_ne!(hmac_key, gossip_topic);
    }

    #[test]
    fn test_full_validation() {
        assert!(validate_cookie_full("my-cluster").is_ok());
        assert_eq!(validate_cookie_full(""), Err(CookieError::Empty));
        assert_eq!(validate_cookie_full(UNSAFE_DEFAULT_COOKIE), Err(CookieError::UnsafeDefault));
    }
}
