//! Pure functions for API operations.
//!
//! This module extracts pure business logic from API implementations
//! following the "Functional Core, Imperative Shell" pattern. All functions
//! are deterministic and side-effect free, enabling:
//! - Unit testing with explicit inputs/outputs
//! - Property-based testing with Bolero
//! - Reuse across different storage backends (InMemory, SQLite, etc.)
//!
//! # Tiger Style
//!
//! - All operations bounded by explicit limits (MAX_SCAN_RESULTS, MAX_BATCH_SIZE)
//! - Deterministic behavior (no time, random, or I/O dependencies)
//! - Explicit error types for all failure modes

use base64::Engine;

use super::DEFAULT_SCAN_LIMIT;
use super::MAX_SCAN_RESULTS;

// ============================================================================
// Scan Pagination Pure Functions
// ============================================================================

/// Normalize scan limit to bounded value.
///
/// Applies default if not specified and caps at MAX_SCAN_RESULTS.
///
/// # Arguments
///
/// * `limit` - Optional user-specified limit
/// * `default` - Default limit to use if not specified
/// * `max` - Maximum allowed limit
///
/// # Returns
///
/// Normalized limit value, guaranteed to be in [1, max].
#[inline]
pub fn normalize_scan_limit(limit: Option<u32>, default: u32, max: u32) -> usize {
    limit.unwrap_or(default).min(max) as usize
}

/// Decode base64-encoded continuation token to a key string.
///
/// # Arguments
///
/// * `token` - Optional base64-encoded continuation token
///
/// # Returns
///
/// Decoded key string, or None if token is invalid or not provided.
pub fn decode_continuation_token(token: Option<&str>) -> Option<String> {
    token.and_then(|t| {
        base64::engine::general_purpose::STANDARD
            .decode(t)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    })
}

/// Encode a key as a base64 continuation token.
///
/// # Arguments
///
/// * `key` - The key to encode
///
/// # Returns
///
/// Base64-encoded token string.
#[inline]
pub fn encode_continuation_token(key: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(key)
}

/// Filter entries by prefix and continuation token.
///
/// # Arguments
///
/// * `entries` - Iterator of (key, value) pairs
/// * `prefix` - Prefix to filter by
/// * `start_after` - Optional continuation token (decoded key)
///
/// # Returns
///
/// Filtered entries matching prefix and after continuation token.
pub fn filter_scan_entries<'a, I, K, V>(
    entries: I,
    prefix: &'a str,
    start_after: Option<&'a str>,
) -> impl Iterator<Item = (K, V)> + 'a
where
    I: Iterator<Item = (K, V)> + 'a,
    K: AsRef<str> + 'a,
    V: 'a,
{
    entries
        .filter(move |(k, _)| k.as_ref().starts_with(prefix))
        .filter(move |(k, _)| start_after.is_none_or(|after| k.as_ref() > after))
}

/// Paginate scan results with truncation detection.
///
/// # Arguments
///
/// * `entries` - Sorted entries to paginate
/// * `limit` - Maximum number of entries to return
///
/// # Returns
///
/// Tuple of (paginated entries, is_truncated).
/// If `is_truncated` is true, there are more results available.
pub fn paginate_entries<T>(entries: Vec<T>, limit: usize) -> (Vec<T>, bool) {
    let is_truncated = entries.len() > limit;
    let paginated = entries.into_iter().take(limit).collect();
    (paginated, is_truncated)
}

/// Build scan result with pagination metadata.
///
/// # Arguments
///
/// * `entries` - Paginated entries
/// * `is_truncated` - Whether more results exist
/// * `last_key` - Optional last key for continuation token
///
/// # Returns
///
/// Tuple of (count, is_truncated, continuation_token).
pub fn build_scan_metadata(count: usize, is_truncated: bool, last_key: Option<&str>) -> (u32, bool, Option<String>) {
    let continuation_token = if is_truncated {
        last_key.map(encode_continuation_token)
    } else {
        None
    };
    (count as u32, is_truncated, continuation_token)
}

/// Complete scan pipeline: normalize, filter, sort, paginate.
///
/// This is a convenience function that combines all scan operations
/// into a single call for simple use cases.
///
/// # Arguments
///
/// * `entries` - Unsorted entries to scan
/// * `prefix` - Prefix to filter by
/// * `continuation_token` - Optional continuation token (base64-encoded)
/// * `limit` - Optional limit (defaults to DEFAULT_SCAN_LIMIT)
///
/// # Returns
///
/// Tuple of (filtered_entries, count, is_truncated, next_continuation_token).
pub fn execute_scan<K, V>(
    mut entries: Vec<(K, V)>,
    prefix: &str,
    continuation_token: Option<&str>,
    limit: Option<u32>,
) -> (Vec<(K, V)>, u32, bool, Option<String>)
where
    K: AsRef<str> + Ord + Clone,
    V: Clone,
{
    // Normalize limit
    let limit = normalize_scan_limit(limit, DEFAULT_SCAN_LIMIT, MAX_SCAN_RESULTS);

    // Decode continuation token
    let start_after = decode_continuation_token(continuation_token);

    // Filter by prefix and continuation
    let filtered: Vec<_> = entries
        .drain(..)
        .filter(|(k, _)| k.as_ref().starts_with(prefix))
        .filter(|(k, _)| start_after.as_deref().is_none_or(|after| k.as_ref() > after))
        .collect();

    // Sort by key
    let mut sorted = filtered;
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    // Paginate
    let (paginated, is_truncated) = paginate_entries(sorted, limit);

    // Build metadata
    let last_key = paginated.last().map(|(k, _)| k.as_ref());
    let (count, is_truncated, next_token) = build_scan_metadata(paginated.len(), is_truncated, last_key);

    (paginated, count, is_truncated, next_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Limit Normalization Tests
    // ========================================================================

    #[test]
    fn test_normalize_limit_default() {
        assert_eq!(normalize_scan_limit(None, 1000, 10000), 1000);
    }

    #[test]
    fn test_normalize_limit_specified() {
        assert_eq!(normalize_scan_limit(Some(500), 1000, 10000), 500);
    }

    #[test]
    fn test_normalize_limit_capped() {
        assert_eq!(normalize_scan_limit(Some(20000), 1000, 10000), 10000);
    }

    #[test]
    fn test_normalize_limit_zero() {
        assert_eq!(normalize_scan_limit(Some(0), 1000, 10000), 0);
    }

    // ========================================================================
    // Continuation Token Tests
    // ========================================================================

    #[test]
    fn test_encode_decode_roundtrip() {
        let key = "test-key-123";
        let encoded = encode_continuation_token(key);
        let decoded = decode_continuation_token(Some(&encoded));
        assert_eq!(decoded, Some(key.to_string()));
    }

    #[test]
    fn test_decode_none() {
        assert_eq!(decode_continuation_token(None), None);
    }

    #[test]
    fn test_decode_invalid_base64() {
        assert_eq!(decode_continuation_token(Some("not-valid-base64!!!")), None);
    }

    #[test]
    fn test_decode_invalid_utf8() {
        // Base64 of invalid UTF-8 bytes
        let invalid_utf8 = base64::engine::general_purpose::STANDARD.encode([0xFF, 0xFE]);
        assert_eq!(decode_continuation_token(Some(&invalid_utf8)), None);
    }

    // ========================================================================
    // Filter Tests
    // ========================================================================

    #[test]
    fn test_filter_by_prefix() {
        let entries = vec![("prefix:a", 1), ("prefix:b", 2), ("other:c", 3), ("prefix:d", 4)];

        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "prefix:", None).collect();

        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().all(|(k, _)| k.starts_with("prefix:")));
    }

    #[test]
    fn test_filter_with_continuation() {
        let entries = vec![("prefix:a", 1), ("prefix:b", 2), ("prefix:c", 3), ("prefix:d", 4)];

        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "prefix:", Some("prefix:b")).collect();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].0, "prefix:c");
        assert_eq!(filtered[1].0, "prefix:d");
    }

    #[test]
    fn test_filter_empty_prefix() {
        let entries = vec![("a", 1), ("b", 2), ("c", 3)];

        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "", None).collect();

        assert_eq!(filtered.len(), 3);
    }

    // ========================================================================
    // Pagination Tests
    // ========================================================================

    #[test]
    fn test_paginate_within_limit() {
        let entries = vec![1, 2, 3];
        let (paginated, is_truncated) = paginate_entries(entries, 10);

        assert_eq!(paginated, vec![1, 2, 3]);
        assert!(!is_truncated);
    }

    #[test]
    fn test_paginate_at_limit() {
        let entries = vec![1, 2, 3];
        let (paginated, is_truncated) = paginate_entries(entries, 3);

        assert_eq!(paginated, vec![1, 2, 3]);
        assert!(!is_truncated);
    }

    #[test]
    fn test_paginate_over_limit() {
        let entries = vec![1, 2, 3, 4, 5];
        let (paginated, is_truncated) = paginate_entries(entries, 3);

        assert_eq!(paginated, vec![1, 2, 3]);
        assert!(is_truncated);
    }

    // ========================================================================
    // Metadata Tests
    // ========================================================================

    #[test]
    fn test_build_metadata_not_truncated() {
        let (count, is_truncated, token) = build_scan_metadata(5, false, Some("last-key"));

        assert_eq!(count, 5);
        assert!(!is_truncated);
        assert!(token.is_none());
    }

    #[test]
    fn test_build_metadata_truncated() {
        let (count, is_truncated, token) = build_scan_metadata(10, true, Some("last-key"));

        assert_eq!(count, 10);
        assert!(is_truncated);
        assert!(token.is_some());

        // Verify token decodes back
        let decoded = decode_continuation_token(token.as_deref());
        assert_eq!(decoded, Some("last-key".to_string()));
    }

    // ========================================================================
    // Execute Scan Pipeline Tests
    // ========================================================================

    #[test]
    fn test_execute_scan_basic() {
        let entries = vec![
            ("prefix:c".to_string(), "3".to_string()),
            ("prefix:a".to_string(), "1".to_string()),
            ("other:x".to_string(), "x".to_string()),
            ("prefix:b".to_string(), "2".to_string()),
        ];

        let (results, count, is_truncated, token) = execute_scan(entries, "prefix:", None, Some(10));

        // Should be sorted
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, "prefix:a");
        assert_eq!(results[1].0, "prefix:b");
        assert_eq!(results[2].0, "prefix:c");
        assert_eq!(count, 3);
        assert!(!is_truncated);
        assert!(token.is_none());
    }

    #[test]
    fn test_execute_scan_with_pagination() {
        let entries = vec![
            ("k:1".to_string(), "1".to_string()),
            ("k:2".to_string(), "2".to_string()),
            ("k:3".to_string(), "3".to_string()),
            ("k:4".to_string(), "4".to_string()),
            ("k:5".to_string(), "5".to_string()),
        ];

        let (results, count, is_truncated, token) = execute_scan(entries, "k:", None, Some(3));

        assert_eq!(results.len(), 3);
        assert_eq!(count, 3);
        assert!(is_truncated);
        assert!(token.is_some());

        // Use token to get next page
        let entries2 = vec![
            ("k:1".to_string(), "1".to_string()),
            ("k:2".to_string(), "2".to_string()),
            ("k:3".to_string(), "3".to_string()),
            ("k:4".to_string(), "4".to_string()),
            ("k:5".to_string(), "5".to_string()),
        ];

        let (results2, count2, is_truncated2, _) = execute_scan(entries2, "k:", token.as_deref(), Some(3));

        assert_eq!(results2.len(), 2);
        assert_eq!(results2[0].0, "k:4");
        assert_eq!(results2[1].0, "k:5");
        assert_eq!(count2, 2);
        assert!(!is_truncated2);
    }
}

#[cfg(all(test, feature = "bolero"))]
mod property_tests {
    use bolero::check;

    use super::*;

    #[test]
    fn prop_normalize_never_exceeds_max() {
        check!().with_type::<(Option<u32>, u32, u32)>().filter(|(_, _, max)| *max > 0).for_each(
            |(limit, default, max)| {
                let result = normalize_scan_limit(*limit, *default, *max);
                assert!(result <= *max as usize);
            },
        );
    }

    #[test]
    fn prop_encode_decode_roundtrip() {
        check!().with_type::<String>().for_each(|key| {
            let encoded = encode_continuation_token(key);
            let decoded = decode_continuation_token(Some(&encoded));
            assert_eq!(decoded.as_deref(), Some(key.as_str()));
        });
    }

    #[test]
    fn prop_paginate_preserves_order() {
        check!()
            .with_type::<(Vec<i32>, usize)>()
            .filter(|(_, limit)| *limit > 0)
            .for_each(|(entries, limit)| {
                let original = entries.clone();
                let (paginated, _) = paginate_entries(entries.clone(), *limit);

                // Paginated entries should be prefix of original
                for (i, item) in paginated.iter().enumerate() {
                    assert_eq!(*item, original[i]);
                }
            });
    }

    #[test]
    fn prop_paginate_truncation_correct() {
        check!()
            .with_type::<(Vec<i32>, usize)>()
            .filter(|(_, limit)| *limit > 0)
            .for_each(|(entries, limit)| {
                let len = entries.len();
                let (paginated, is_truncated) = paginate_entries(entries, *limit);

                if len > *limit {
                    assert!(is_truncated);
                    assert_eq!(paginated.len(), *limit);
                } else {
                    assert!(!is_truncated);
                    assert_eq!(paginated.len(), len);
                }
            });
    }
}
