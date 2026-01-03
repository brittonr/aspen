//! Pure functions for API operations.
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
pub fn normalize_scan_limit(limit: Option<u32>, default: u32, max: u32) -> usize {
    limit.unwrap_or(default).min(max) as usize
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
pub fn paginate_entries<T>(entries: Vec<T>, limit: usize) -> (Vec<T>, bool) {
    let is_truncated = entries.len() > limit;
    let paginated = entries.into_iter().take(limit).collect();
    (paginated, is_truncated)
}

/// Build scan result with pagination metadata.
pub fn build_scan_metadata(count: usize, is_truncated: bool, last_key: Option<&str>) -> (u32, bool, Option<String>) {
    let continuation_token = if is_truncated {
        last_key.map(encode_continuation_token)
    } else {
        None
    };
    (count as u32, is_truncated, continuation_token)
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
    let (count, is_truncated, next_token) = build_scan_metadata(paginated.len(), is_truncated, last_key);

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
}
