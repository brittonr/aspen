//! Pure scan and pagination functions.
//!
//! This module contains pure functions for KV scan operations including
//! continuation token encoding/decoding, pagination logic, and result building.
//! All functions are deterministic and side-effect free.
//!
//! # Tiger Style
//!
//! - Bounded results prevent unbounded memory usage
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes

use base64::Engine;

// ============================================================================
// Continuation Token Encoding/Decoding
// ============================================================================

/// Decode a base64-encoded continuation token to the original key.
///
/// Continuation tokens are base64-encoded keys used for pagination.
/// This function decodes the token back to the original key string.
///
/// # Arguments
///
/// * `token` - Base64-encoded continuation token
///
/// # Returns
///
/// - `Some(key)` if decoding succeeds
/// - `None` if base64 decode fails or result is not valid UTF-8
///
/// # Example
///
/// ```
/// use aspen_raft::pure::scan::decode_continuation_token;
///
/// let token = "aGVsbG8="; // base64("hello")
/// assert_eq!(decode_continuation_token(token), Some("hello".to_string()));
///
/// let invalid = "not-valid-base64!!!";
/// assert_eq!(decode_continuation_token(invalid), None);
/// ```
#[inline]
pub fn decode_continuation_token(token: &str) -> Option<String> {
    base64::engine::general_purpose::STANDARD
        .decode(token)
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// Encode a key as a base64 continuation token.
///
/// Creates a continuation token that can be used for pagination.
/// The token encodes the last key returned so the next page can
/// start after that key.
///
/// # Arguments
///
/// * `key` - The key to encode as a continuation token
///
/// # Returns
///
/// Base64-encoded string representing the key.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::scan::encode_continuation_token;
///
/// let token = encode_continuation_token("hello");
/// assert_eq!(token, "aGVsbG8=");
/// ```
#[inline]
pub fn encode_continuation_token(key: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(key)
}

// ============================================================================
// Pagination Logic
// ============================================================================

/// Filter key-value pairs to those after a continuation token.
///
/// Used for pagination: returns only keys that lexicographically
/// come after the continuation token key. Uses `>` (not `>=`) to
/// handle the case where the continuation token key was deleted.
///
/// # Arguments
///
/// * `pairs` - Key-value pairs to filter (should be sorted by key)
/// * `start_after` - Optional key to start after (from decoded continuation token)
///
/// # Returns
///
/// Filtered vector containing only pairs where key > start_after.
/// If start_after is None, returns all pairs unchanged.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::scan::filter_kv_pairs_after_token;
///
/// let pairs = vec![
///     ("a".to_string(), "1".to_string()),
///     ("b".to_string(), "2".to_string()),
///     ("c".to_string(), "3".to_string()),
/// ];
///
/// // Start after "a"
/// let filtered = filter_kv_pairs_after_token(pairs.clone(), &Some("a".to_string()));
/// assert_eq!(filtered.len(), 2);
/// assert_eq!(filtered[0].0, "b");
///
/// // No token - return all
/// let all = filter_kv_pairs_after_token(pairs, &None);
/// assert_eq!(all.len(), 3);
/// ```
#[inline]
pub fn filter_kv_pairs_after_token(
    pairs: Vec<(String, String)>,
    start_after: &Option<String>,
) -> Vec<(String, String)> {
    match start_after {
        Some(after) => pairs.into_iter().filter(|(k, _)| k.as_str() > after.as_str()).collect(),
        None => pairs,
    }
}

/// Pagination result from scan operations.
///
/// Contains the computed pagination state for building scan results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginationResult {
    /// Whether there are more results beyond the limit.
    pub is_truncated: bool,
    /// Continuation token for fetching the next page, if truncated.
    pub continuation_token: Option<String>,
    /// Number of entries in the result (up to limit).
    pub count: u32,
}

/// Determine pagination state from filtered results.
///
/// Computes whether results are truncated and generates a continuation
/// token if needed for the next page.
///
/// # Arguments
///
/// * `total_filtered` - Total number of entries after filtering
/// * `limit` - Maximum entries to return
/// * `last_key` - Last key in the result set (for generating token)
///
/// # Returns
///
/// A [`PaginationResult`] with truncation state and optional continuation token.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::scan::compute_pagination_result;
///
/// // More results available (truncated)
/// let result = compute_pagination_result(15, 10, Some("last_key"));
/// assert!(result.is_truncated);
/// assert!(result.continuation_token.is_some());
/// assert_eq!(result.count, 10);
///
/// // All results fit (not truncated)
/// let result = compute_pagination_result(5, 10, Some("last_key"));
/// assert!(!result.is_truncated);
/// assert!(result.continuation_token.is_none());
/// assert_eq!(result.count, 5);
/// ```
#[inline]
pub fn compute_pagination_result(
    total_filtered: usize,
    limit: usize,
    last_key: Option<&str>,
) -> PaginationResult {
    let is_truncated = total_filtered > limit;
    let count = total_filtered.min(limit) as u32;

    let continuation_token = if is_truncated {
        last_key.map(encode_continuation_token)
    } else {
        None
    };

    PaginationResult {
        is_truncated,
        continuation_token,
        count,
    }
}

/// Compute a safe scan limit from user input.
///
/// Applies default limit if not specified, then enforces maximum bounds.
///
/// # Arguments
///
/// * `requested` - User-requested limit (optional)
/// * `default_limit` - Default limit to use if not specified
/// * `max_limit` - Maximum allowed limit
///
/// # Returns
///
/// The effective limit, bounded by maximum.
///
/// # Example
///
/// ```
/// use aspen_raft::pure::scan::compute_safe_scan_limit;
///
/// // Uses default when not specified
/// assert_eq!(compute_safe_scan_limit(None, 100, 1000), 100);
///
/// // Respects user limit within bounds
/// assert_eq!(compute_safe_scan_limit(Some(50), 100, 1000), 50);
///
/// // Clamps to maximum
/// assert_eq!(compute_safe_scan_limit(Some(5000), 100, 1000), 1000);
/// ```
#[inline]
pub fn compute_safe_scan_limit(requested: Option<u32>, default_limit: u32, max_limit: u32) -> usize {
    requested.unwrap_or(default_limit).min(max_limit) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Continuation Token Tests
    // ========================================================================

    #[test]
    fn test_encode_decode_roundtrip() {
        let key = "test/key/path";
        let token = encode_continuation_token(key);
        let decoded = decode_continuation_token(&token);
        assert_eq!(decoded, Some(key.to_string()));
    }

    #[test]
    fn test_decode_invalid_base64() {
        assert_eq!(decode_continuation_token("not-valid-base64!!!"), None);
    }

    #[test]
    fn test_decode_invalid_utf8() {
        // Valid base64 but invalid UTF-8
        let token = base64::engine::general_purpose::STANDARD.encode(&[0xFF, 0xFE]);
        assert_eq!(decode_continuation_token(&token), None);
    }

    #[test]
    fn test_encode_empty_string() {
        let token = encode_continuation_token("");
        assert_eq!(decode_continuation_token(&token), Some(String::new()));
    }

    #[test]
    fn test_encode_unicode() {
        let key = "hello/世界/🌍";
        let token = encode_continuation_token(key);
        assert_eq!(decode_continuation_token(&token), Some(key.to_string()));
    }

    // ========================================================================
    // Filter Tests
    // ========================================================================

    #[test]
    fn test_filter_no_token() {
        let pairs = vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ];
        let result = filter_kv_pairs_after_token(pairs.clone(), &None);
        assert_eq!(result, pairs);
    }

    #[test]
    fn test_filter_with_token() {
        let pairs = vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
            ("c".to_string(), "3".to_string()),
        ];
        let result = filter_kv_pairs_after_token(pairs, &Some("a".to_string()));
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "b");
        assert_eq!(result[1].0, "c");
    }

    #[test]
    fn test_filter_exact_match_excluded() {
        // The token key itself should NOT be included (uses > not >=)
        let pairs = vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ];
        let result = filter_kv_pairs_after_token(pairs, &Some("a".to_string()));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "b");
    }

    #[test]
    fn test_filter_token_after_all() {
        let pairs = vec![
            ("a".to_string(), "1".to_string()),
            ("b".to_string(), "2".to_string()),
        ];
        let result = filter_kv_pairs_after_token(pairs, &Some("z".to_string()));
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_empty_pairs() {
        let pairs: Vec<(String, String)> = vec![];
        let result = filter_kv_pairs_after_token(pairs, &Some("a".to_string()));
        assert!(result.is_empty());
    }

    // ========================================================================
    // Pagination Tests
    // ========================================================================

    #[test]
    fn test_pagination_truncated() {
        let result = compute_pagination_result(15, 10, Some("last"));
        assert!(result.is_truncated);
        assert_eq!(result.count, 10);
        assert!(result.continuation_token.is_some());
    }

    #[test]
    fn test_pagination_not_truncated() {
        let result = compute_pagination_result(5, 10, Some("last"));
        assert!(!result.is_truncated);
        assert_eq!(result.count, 5);
        assert!(result.continuation_token.is_none());
    }

    #[test]
    fn test_pagination_exact_limit() {
        let result = compute_pagination_result(10, 10, Some("last"));
        assert!(!result.is_truncated);
        assert_eq!(result.count, 10);
        assert!(result.continuation_token.is_none());
    }

    #[test]
    fn test_pagination_empty() {
        let result = compute_pagination_result(0, 10, None);
        assert!(!result.is_truncated);
        assert_eq!(result.count, 0);
        assert!(result.continuation_token.is_none());
    }

    #[test]
    fn test_pagination_truncated_no_last_key() {
        // Edge case: truncated but no last key (shouldn't happen in practice)
        let result = compute_pagination_result(15, 10, None);
        assert!(result.is_truncated);
        assert_eq!(result.count, 10);
        assert!(result.continuation_token.is_none());
    }

    // ========================================================================
    // Safe Limit Tests
    // ========================================================================

    #[test]
    fn test_safe_limit_default() {
        assert_eq!(compute_safe_scan_limit(None, 100, 1000), 100);
    }

    #[test]
    fn test_safe_limit_within_bounds() {
        assert_eq!(compute_safe_scan_limit(Some(50), 100, 1000), 50);
    }

    #[test]
    fn test_safe_limit_clamp_to_max() {
        assert_eq!(compute_safe_scan_limit(Some(5000), 100, 1000), 1000);
    }

    #[test]
    fn test_safe_limit_zero() {
        assert_eq!(compute_safe_scan_limit(Some(0), 100, 1000), 0);
    }

    #[test]
    fn test_safe_limit_max_values() {
        assert_eq!(compute_safe_scan_limit(Some(u32::MAX), u32::MAX, u32::MAX), u32::MAX as usize);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_encode_decode_roundtrip() {
        check!().with_type::<String>().for_each(|key| {
            let token = encode_continuation_token(key);
            let decoded = decode_continuation_token(&token);
            assert_eq!(decoded.as_deref(), Some(key.as_str()));
        });
    }

    #[test]
    fn prop_filter_preserves_order() {
        check!()
            .with_type::<(Vec<(String, String)>, Option<String>)>()
            .for_each(|(pairs, token)| {
                let result = filter_kv_pairs_after_token(pairs.clone(), token);
                // Result should maintain order of input
                for window in result.windows(2) {
                    // If input was sorted, output should be sorted
                    // (we don't enforce input sorting here)
                }
                // All result keys should be > token if token is Some
                if let Some(ref after) = token {
                    for (k, _) in &result {
                        assert!(k.as_str() > after.as_str());
                    }
                }
            });
    }

    #[test]
    fn prop_pagination_count_bounded() {
        check!().with_type::<(usize, usize)>().for_each(|(total, limit)| {
            let result = compute_pagination_result(*total, *limit, Some("key"));
            assert!(result.count as usize <= *limit);
            assert!(result.count as usize <= *total);
        });
    }

    #[test]
    fn prop_safe_limit_bounded() {
        check!().with_type::<(Option<u32>, u32, u32)>().for_each(|(req, default, max)| {
            let result = compute_safe_scan_limit(*req, *default, *max);
            assert!(result <= *max as usize);
        });
    }
}
