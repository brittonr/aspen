//! Pure functions for scan operations.
//!
//! This module extracts pure business logic from API implementations
//! following the "Functional Core, Imperative Shell" pattern.

use base64::Engine;

use crate::constants::DEFAULT_SCAN_LIMIT;
use crate::constants::MAX_SCAN_RESULTS;

/// Normalize scan limit to bounded value.
///
/// Applies default if not specified and caps at MAX_SCAN_RESULTS.
#[inline]
pub fn normalize_scan_limit(limit: Option<u32>, default: u32, max: u32) -> u32 {
    limit.unwrap_or(default).min(max)
}

/// Decode base64-encoded continuation token to a key string.
pub fn decode_continuation_token(token: Option<&str>) -> Option<String> {
    token.and_then(|t| {
        base64::engine::general_purpose::STANDARD
            .decode(t)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    })
}

/// Encode a key as a base64 continuation token.
#[inline]
pub fn encode_continuation_token(key: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(key)
}

/// Filter entries by prefix and continuation token.
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
pub fn paginate_entries<T>(entries: Vec<T>, limit: u32) -> (Vec<T>, bool) {
    let is_truncated = entries.len() > limit as usize;
    let paginated = entries.into_iter().take(limit as usize).collect();
    (paginated, is_truncated)
}

/// Build scan result with pagination metadata.
pub fn build_scan_metadata(count: u32, is_truncated: bool, last_key: Option<&str>) -> (u32, bool, Option<String>) {
    let continuation_token = if is_truncated {
        last_key.map(encode_continuation_token)
    } else {
        None
    };
    (count, is_truncated, continuation_token)
}

/// Complete scan pipeline: normalize, filter, sort, paginate.
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
    let limit = normalize_scan_limit(limit, DEFAULT_SCAN_LIMIT, MAX_SCAN_RESULTS);
    let start_after = decode_continuation_token(continuation_token);

    let filtered: Vec<_> = entries
        .drain(..)
        .filter(|(k, _)| k.as_ref().starts_with(prefix))
        .filter(|(k, _)| start_after.as_deref().is_none_or(|after| k.as_ref() > after))
        .collect();

    let mut sorted = filtered;
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let (paginated, is_truncated) = paginate_entries(sorted, limit);

    let last_key = paginated.last().map(|(k, _)| k.as_ref());
    let (count, is_truncated, next_token) = build_scan_metadata(paginated.len() as u32, is_truncated, last_key);

    (paginated, count, is_truncated, next_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_limit_default() {
        assert_eq!(normalize_scan_limit(None, 1000, 10000), 1000);
    }

    #[test]
    fn test_normalize_limit_capped() {
        assert_eq!(normalize_scan_limit(Some(20000), 1000, 10000), 10000);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let key = "test-key-123";
        let encoded = encode_continuation_token(key);
        let decoded = decode_continuation_token(Some(&encoded));
        assert_eq!(decoded, Some(key.to_string()));
    }

    #[test]
    fn test_paginate_over_limit() {
        let entries = vec![1, 2, 3, 4, 5];
        let (paginated, is_truncated) = paginate_entries(entries, 3);
        assert_eq!(paginated, vec![1, 2, 3]);
        assert!(is_truncated);
    }

    // =========================================================================
    // Normalize Scan Limit Tests
    // =========================================================================

    #[test]
    fn test_normalize_limit_specified_under_max() {
        assert_eq!(normalize_scan_limit(Some(500), 1000, 10000), 500);
    }

    #[test]
    fn test_normalize_limit_zero() {
        assert_eq!(normalize_scan_limit(Some(0), 1000, 10000), 0);
    }

    #[test]
    fn test_normalize_limit_equal_to_max() {
        assert_eq!(normalize_scan_limit(Some(10000), 1000, 10000), 10000);
    }

    #[test]
    fn test_normalize_limit_default_exceeds_max() {
        // Default higher than max should be capped
        assert_eq!(normalize_scan_limit(None, 20000, 10000), 10000);
    }

    // =========================================================================
    // Continuation Token Tests
    // =========================================================================

    #[test]
    fn test_decode_none_token() {
        assert_eq!(decode_continuation_token(None), None);
    }

    #[test]
    fn test_decode_invalid_base64() {
        // Invalid base64 should return None
        assert_eq!(decode_continuation_token(Some("not-valid-base64!!!")), None);
    }

    #[test]
    fn test_decode_valid_base64_but_invalid_utf8() {
        // Valid base64 encoding of invalid UTF-8 bytes
        let invalid_utf8 = base64::engine::general_purpose::STANDARD.encode([0xFF, 0xFE]);
        assert_eq!(decode_continuation_token(Some(&invalid_utf8)), None);
    }

    #[test]
    fn test_encode_decode_empty_string() {
        let key = "";
        let encoded = encode_continuation_token(key);
        let decoded = decode_continuation_token(Some(&encoded));
        assert_eq!(decoded, Some(key.to_string()));
    }

    #[test]
    fn test_encode_decode_unicode() {
        let key = "test-unicode";
        let encoded = encode_continuation_token(key);
        let decoded = decode_continuation_token(Some(&encoded));
        assert_eq!(decoded, Some(key.to_string()));
    }

    #[test]
    fn test_encode_decode_special_characters() {
        let key = "key/with/slashes/and=equals&and?question";
        let encoded = encode_continuation_token(key);
        let decoded = decode_continuation_token(Some(&encoded));
        assert_eq!(decoded, Some(key.to_string()));
    }

    #[test]
    fn test_encode_decode_long_key() {
        let key = "x".repeat(10000);
        let encoded = encode_continuation_token(&key);
        let decoded = decode_continuation_token(Some(&encoded));
        assert_eq!(decoded, Some(key));
    }

    // =========================================================================
    // Filter Scan Entries Tests
    // =========================================================================

    #[test]
    fn test_filter_by_prefix_only() {
        let entries = vec![
            ("app/users/1".to_string(), "v1"),
            ("app/users/2".to_string(), "v2"),
            ("app/orders/1".to_string(), "v3"),
            ("other/data".to_string(), "v4"),
        ];

        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "app/users/", None).collect();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].0, "app/users/1");
        assert_eq!(filtered[1].0, "app/users/2");
    }

    #[test]
    fn test_filter_by_start_after_only() {
        let entries = vec![
            ("a".to_string(), "v1"),
            ("b".to_string(), "v2"),
            ("c".to_string(), "v3"),
            ("d".to_string(), "v4"),
        ];

        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "", Some("b")).collect();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].0, "c");
        assert_eq!(filtered[1].0, "d");
    }

    #[test]
    fn test_filter_by_prefix_and_start_after() {
        let entries = vec![
            ("prefix/a".to_string(), "v1"),
            ("prefix/b".to_string(), "v2"),
            ("prefix/c".to_string(), "v3"),
            ("other/d".to_string(), "v4"),
        ];

        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "prefix/", Some("prefix/a")).collect();

        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].0, "prefix/b");
        assert_eq!(filtered[1].0, "prefix/c");
    }

    #[test]
    fn test_filter_empty_input() {
        let entries: Vec<(String, &str)> = vec![];
        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "any", None).collect();
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_no_matches() {
        let entries = vec![("other/key".to_string(), "v1")];
        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "prefix/", None).collect();
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_start_after_exact_match() {
        // start_after is exclusive, so exact match should be excluded
        let entries = vec![
            ("key1".to_string(), "v1"),
            ("key2".to_string(), "v2"),
            ("key3".to_string(), "v3"),
        ];

        let filtered: Vec<_> = filter_scan_entries(entries.into_iter(), "", Some("key2")).collect();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "key3");
    }

    // =========================================================================
    // Paginate Entries Tests
    // =========================================================================

    #[test]
    fn test_paginate_under_limit() {
        let entries = vec![1, 2, 3];
        let (paginated, is_truncated) = paginate_entries(entries, 10);
        assert_eq!(paginated, vec![1, 2, 3]);
        assert!(!is_truncated);
    }

    #[test]
    fn test_paginate_at_exact_limit() {
        let entries = vec![1, 2, 3];
        let (paginated, is_truncated) = paginate_entries(entries, 3);
        assert_eq!(paginated, vec![1, 2, 3]);
        assert!(!is_truncated);
    }

    #[test]
    fn test_paginate_one_over_limit() {
        let entries = vec![1, 2, 3, 4];
        let (paginated, is_truncated) = paginate_entries(entries, 3);
        assert_eq!(paginated, vec![1, 2, 3]);
        assert!(is_truncated);
    }

    #[test]
    fn test_paginate_empty() {
        let entries: Vec<i32> = vec![];
        let (paginated, is_truncated) = paginate_entries(entries, 10);
        assert!(paginated.is_empty());
        assert!(!is_truncated);
    }

    #[test]
    fn test_paginate_zero_limit() {
        let entries = vec![1, 2, 3];
        let (paginated, is_truncated) = paginate_entries(entries, 0);
        assert!(paginated.is_empty());
        assert!(is_truncated);
    }

    // =========================================================================
    // Build Scan Metadata Tests
    // =========================================================================

    #[test]
    fn test_build_scan_metadata_not_truncated() {
        let (count, is_truncated, token) = build_scan_metadata(5, false, Some("last-key"));
        assert_eq!(count, 5);
        assert!(!is_truncated);
        assert!(token.is_none());
    }

    #[test]
    fn test_build_scan_metadata_truncated_with_last_key() {
        let (count, is_truncated, token) = build_scan_metadata(10, true, Some("last-key"));
        assert_eq!(count, 10);
        assert!(is_truncated);
        assert!(token.is_some());

        // Token should decode back to last-key
        let decoded = decode_continuation_token(token.as_deref());
        assert_eq!(decoded, Some("last-key".to_string()));
    }

    #[test]
    fn test_build_scan_metadata_truncated_no_last_key() {
        let (count, is_truncated, token) = build_scan_metadata(0, true, None);
        assert_eq!(count, 0);
        assert!(is_truncated);
        assert!(token.is_none());
    }

    #[test]
    fn test_build_scan_metadata_empty_result() {
        let (count, is_truncated, token) = build_scan_metadata(0, false, None);
        assert_eq!(count, 0);
        assert!(!is_truncated);
        assert!(token.is_none());
    }

    // =========================================================================
    // Execute Scan Pipeline Tests
    // =========================================================================

    #[test]
    fn test_execute_scan_basic() {
        let entries = vec![
            ("b".to_string(), "v2"),
            ("a".to_string(), "v1"),
            ("c".to_string(), "v3"),
        ];

        let (result, count, is_truncated, token) = execute_scan(entries, "", None, Some(10));

        assert_eq!(count, 3);
        assert!(!is_truncated);
        assert!(token.is_none());

        // Should be sorted
        assert_eq!(result[0].0, "a");
        assert_eq!(result[1].0, "b");
        assert_eq!(result[2].0, "c");
    }

    #[test]
    fn test_execute_scan_with_prefix_filter() {
        let entries = vec![
            ("app/b".to_string(), "v2"),
            ("app/a".to_string(), "v1"),
            ("other/c".to_string(), "v3"),
        ];

        let (result, count, is_truncated, _token) = execute_scan(entries, "app/", None, Some(10));

        assert_eq!(count, 2);
        assert!(!is_truncated);
        assert_eq!(result[0].0, "app/a");
        assert_eq!(result[1].0, "app/b");
    }

    #[test]
    fn test_execute_scan_with_pagination() {
        let entries = vec![
            ("a".to_string(), "v1"),
            ("b".to_string(), "v2"),
            ("c".to_string(), "v3"),
            ("d".to_string(), "v4"),
            ("e".to_string(), "v5"),
        ];

        let (result, count, is_truncated, token) = execute_scan(entries, "", None, Some(3));

        assert_eq!(count, 3);
        assert!(is_truncated);
        assert!(token.is_some());
        assert_eq!(result[0].0, "a");
        assert_eq!(result[1].0, "b");
        assert_eq!(result[2].0, "c");

        // Token should point to "c"
        let decoded = decode_continuation_token(token.as_deref());
        assert_eq!(decoded, Some("c".to_string()));
    }

    #[test]
    fn test_execute_scan_with_continuation() {
        let entries = vec![
            ("a".to_string(), "v1"),
            ("b".to_string(), "v2"),
            ("c".to_string(), "v3"),
            ("d".to_string(), "v4"),
            ("e".to_string(), "v5"),
        ];

        // First page
        let (result1, _, _, token1) = execute_scan(entries.clone(), "", None, Some(2));
        assert_eq!(result1.len(), 2);

        // Second page using continuation token
        let (result2, count2, is_truncated2, token2) = execute_scan(entries.clone(), "", token1.as_deref(), Some(2));
        assert_eq!(count2, 2);
        assert!(is_truncated2);
        assert_eq!(result2[0].0, "c");
        assert_eq!(result2[1].0, "d");

        // Third page
        let (result3, count3, is_truncated3, token3) = execute_scan(entries, "", token2.as_deref(), Some(2));
        assert_eq!(count3, 1);
        assert!(!is_truncated3);
        assert!(token3.is_none());
        assert_eq!(result3[0].0, "e");
    }

    #[test]
    fn test_execute_scan_empty_result() {
        let entries: Vec<(String, &str)> = vec![];

        let (result, count, is_truncated, token) = execute_scan(entries, "any", None, Some(10));

        assert!(result.is_empty());
        assert_eq!(count, 0);
        assert!(!is_truncated);
        assert!(token.is_none());
    }

    #[test]
    fn test_execute_scan_no_matches_for_prefix() {
        let entries = vec![("other/a".to_string(), "v1"), ("other/b".to_string(), "v2")];

        let (result, count, is_truncated, token) = execute_scan(entries, "app/", None, Some(10));

        assert!(result.is_empty());
        assert_eq!(count, 0);
        assert!(!is_truncated);
        assert!(token.is_none());
    }

    #[test]
    fn test_execute_scan_default_limit() {
        // When limit is None, should use DEFAULT_SCAN_LIMIT
        let entries: Vec<(String, &str)> = (0..2000).map(|i| (format!("key{:04}", i), "value")).collect();

        let (result, count, is_truncated, _) = execute_scan(entries, "", None, None);

        assert_eq!(count, DEFAULT_SCAN_LIMIT);
        assert_eq!(result.len(), DEFAULT_SCAN_LIMIT as usize);
        assert!(is_truncated);
    }

    #[test]
    fn test_execute_scan_limit_capped_at_max() {
        // When limit exceeds MAX_SCAN_RESULTS, should be capped
        let entries: Vec<(String, &str)> = (0..15000).map(|i| (format!("key{:05}", i), "value")).collect();

        let (result, count, is_truncated, _) = execute_scan(entries, "", None, Some(20000));

        assert_eq!(count, MAX_SCAN_RESULTS);
        assert_eq!(result.len(), MAX_SCAN_RESULTS as usize);
        assert!(is_truncated);
    }

    #[test]
    fn test_execute_scan_preserves_values() {
        let entries = vec![("key1".to_string(), "value-one"), ("key2".to_string(), "value-two")];

        let (result, _, _, _) = execute_scan(entries, "", None, Some(10));

        assert_eq!(result[0], ("key1".to_string(), "value-one"));
        assert_eq!(result[1], ("key2".to_string(), "value-two"));
    }

    #[test]
    fn test_execute_scan_unsorted_input() {
        let entries = vec![
            ("z".to_string(), "last"),
            ("m".to_string(), "middle"),
            ("a".to_string(), "first"),
        ];

        let (result, _, _, _) = execute_scan(entries, "", None, Some(10));

        // Output should be sorted regardless of input order
        assert_eq!(result[0].0, "a");
        assert_eq!(result[1].0, "m");
        assert_eq!(result[2].0, "z");
    }

    // =========================================================================
    // Integration / Edge Case Tests
    // =========================================================================

    #[test]
    fn test_full_pagination_sequence() {
        let entries: Vec<(String, &str)> = (0..10).map(|i| (format!("{:02}", i), "v")).collect();

        let mut all_results = vec![];
        let mut token: Option<String> = None;

        loop {
            let (page, count, is_truncated, next_token) = execute_scan(entries.clone(), "", token.as_deref(), Some(3));

            all_results.extend(page);

            if !is_truncated || count == 0 {
                break;
            }
            token = next_token;
        }

        assert_eq!(all_results.len(), 10);

        // Verify all keys are present and in order
        for (i, (key, _)) in all_results.iter().enumerate() {
            assert_eq!(key, &format!("{:02}", i));
        }
    }

    #[test]
    fn test_prefix_with_special_regex_chars() {
        // Prefix matching should be literal, not regex
        let entries = vec![
            ("prefix.*.test".to_string(), "v1"),
            ("prefix.actual.test".to_string(), "v2"),
            ("other".to_string(), "v3"),
        ];

        let (result, _, _, _) = execute_scan(entries, "prefix.*.", None, Some(10));

        // Should only match the literal "prefix.*." prefix
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "prefix.*.test");
    }
}
