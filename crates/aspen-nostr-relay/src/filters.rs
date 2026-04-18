//! NIP-01 filter matching utilities.
//!
//! The `nostr` crate provides `Filter::match_event()` for in-memory matching.
//! This module adds KV scan prefix extraction and result processing.

use nostr::prelude::*;

use crate::constants;

/// Extract KV prefixes to scan for candidate events matching a filter.
///
/// Returns the most selective set of prefixes to scan. The caller
/// loads events from these prefixes and then applies full `Filter::match_event()`
/// for precise matching.
pub fn filter_scan_prefixes(filter: &Filter) -> Vec<String> {
    debug_assert!(!constants::KV_PREFIX_EVENT.is_empty(), "event prefix must not be empty");
    debug_assert!(!constants::KV_PREFIX_KIND.is_empty(), "kind prefix must not be empty");

    // Direct event ID lookups — most selective
    if let Some(ref ids) = filter.ids {
        return ids.iter().map(|id| format!("{}{}", constants::KV_PREFIX_EVENT, id.to_hex())).collect();
    }

    // Kind-scoped scans
    if let Some(ref kinds) = filter.kinds {
        return kinds.iter().map(|kind| format!("{}{:06}:", constants::KV_PREFIX_KIND, kind.as_u16())).collect();
    }

    // Author-scoped scans
    if let Some(ref authors) = filter.authors {
        return authors
            .iter()
            .map(|author| format!("{}{}:", constants::KV_PREFIX_AUTHOR, author.to_hex()))
            .collect();
    }

    // Tag-scoped scans
    if !filter.generic_tags.is_empty() {
        let tag_count: usize = filter.generic_tags.values().map(|vs| vs.len()).sum();
        let mut prefixes = Vec::with_capacity(tag_count);
        for (tag, values) in filter.generic_tags.iter() {
            for value in values {
                prefixes.push(format!("{}{}:{}:", constants::KV_PREFIX_TAG, tag.as_char(), value));
            }
        }
        if !prefixes.is_empty() {
            return prefixes;
        }
    }

    // Full scan fallback
    vec![constants::KV_PREFIX_EVENT.to_string()]
}

/// A scan range: start prefix (inclusive) and optional end prefix (exclusive).
///
/// When `end` is `None`, the scan covers all keys starting with `start`.
/// When `end` is `Some(bound)`, keys are only returned while `key < bound`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRange {
    pub start: String,
    pub end: Option<String>,
}

/// Extract timestamp-bounded KV scan ranges for a filter.
///
/// Like `filter_scan_prefixes`, but incorporates `since`/`until` to narrow
/// kind and author index scans. Tag and event-ID scans are unaffected
/// (tags don't embed timestamps; event IDs are direct lookups).
pub fn filter_scan_ranges(filter: &Filter) -> Vec<ScanRange> {
    debug_assert!(filter.since.is_none_or(|s| filter.until.is_none_or(|u| s <= u)), "since must not exceed until");
    let since_pad = filter.since.map(|t| format!("{:016}", t.as_secs()));
    let until_pad = filter.until.map(|t| format!("{:016}", t.as_secs()));

    // Direct event ID lookups — no timestamp bounding possible
    if let Some(ref ids) = filter.ids {
        return ids
            .iter()
            .map(|id| ScanRange {
                start: format!("{}{}", constants::KV_PREFIX_EVENT, id.to_hex()),
                end: None,
            })
            .collect();
    }

    // Kind-scoped scans with timestamp bounds
    if let Some(ref kinds) = filter.kinds {
        return kinds
            .iter()
            .map(|kind| {
                let kind_prefix = format!("{}{:06}:", constants::KV_PREFIX_KIND, kind.as_u16());
                bounded_range(&kind_prefix, since_pad.as_deref(), until_pad.as_deref())
            })
            .collect();
    }

    // Author-scoped scans with timestamp bounds
    if let Some(ref authors) = filter.authors {
        return authors
            .iter()
            .map(|author| {
                let author_prefix = format!("{}{}:", constants::KV_PREFIX_AUTHOR, author.to_hex());
                bounded_range(&author_prefix, since_pad.as_deref(), until_pad.as_deref())
            })
            .collect();
    }

    // Tag-scoped scans — no timestamp in key, can't bound
    if !filter.generic_tags.is_empty() {
        let tag_count: usize = filter.generic_tags.values().map(|vs| vs.len()).sum();
        let mut ranges = Vec::with_capacity(tag_count);
        for (tag, values) in filter.generic_tags.iter() {
            for value in values {
                ranges.push(ScanRange {
                    start: format!("{}{}:{}:", constants::KV_PREFIX_TAG, tag.as_char(), value),
                    end: None,
                });
            }
        }
        if !ranges.is_empty() {
            return ranges;
        }
    }

    // Full scan fallback
    vec![ScanRange {
        start: constants::KV_PREFIX_EVENT.to_string(),
        end: None,
    }]
}

/// Build a `ScanRange` from a prefix and optional since/until timestamp strings.
///
/// Index keys have the shape `{prefix}{timestamp}:{event_id}`.
///
/// `since` bounds are applied via the start prefix — but only if the KV store
/// supports lexicographic range scanning. Since `ScanRequest` does prefix-based
/// scanning, `since` narrows the start prefix so keys with earlier timestamps
/// are never returned by the prefix scan (only exact prefix matches).
///
/// `until` bounds are applied as an end marker during iteration (early
/// termination on sorted keys).
///
/// NOTE: `since` bounding via prefix scan only works correctly when the
/// timestamp IS the next component after the prefix. The scan returns all
/// keys whose string representation starts with `{prefix}{since_padded}` or
/// comes lexicographically after it within the prefix namespace. Since
/// `ScanRequest` only does prefix matching (not range), we use the base
/// prefix for `since` and rely on in-memory filtering + `until` for
/// early termination.
pub fn bounded_range(prefix: &str, _since: Option<&str>, until: Option<&str>) -> ScanRange {
    // `since` cannot narrow a prefix scan — keys with timestamps > since
    // don't share the since prefix string. We filter in-memory via
    // Filter::match_event instead.
    let start = prefix.to_string();
    let end = until.map(|ts| format!("{prefix}{ts}{}", '\u{00ff}'));
    ScanRange { start, end }
}

/// Apply limit and sort ordering to query results.
///
/// Events are sorted by `created_at` descending (newest first),
/// then truncated to the limit.
pub fn apply_limit_and_sort(events: &mut Vec<Event>, limit: Option<u32>) {
    events.sort_by_key(|e| std::cmp::Reverse(e.created_at));
    if let Some(limit) = limit {
        events.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    }
}

/// Deduplicate events by event ID, preserving order.
pub fn dedup_events(events: &mut Vec<Event>) {
    let mut seen = std::collections::HashSet::new();
    events.retain(|e| seen.insert(e.id));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kind_prefix_format() {
        let prefixes = filter_scan_prefixes(&Filter::new().kind(Kind::Custom(30617)));
        assert_eq!(prefixes.len(), 1);
        assert!(prefixes[0].starts_with("nostr:ki:030617:"));
    }

    #[test]
    fn test_empty_filter_full_scan() {
        let prefixes = filter_scan_prefixes(&Filter::new());
        assert_eq!(prefixes, vec!["nostr:ev:"]);
    }

    #[test]
    fn test_dedup() {
        let keys = Keys::generate();
        let event = EventBuilder::text_note("hello").sign_with_keys(&keys).unwrap();
        let mut events = vec![event.clone(), event.clone()];
        dedup_events(&mut events);
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn test_bounded_range_kind_with_since() {
        let filter = Filter::new().kind(Kind::Custom(30617)).since(Timestamp::from_secs(1700000000));
        let ranges = filter_scan_ranges(&filter);
        assert_eq!(ranges.len(), 1);
        // `since` doesn't narrow prefix (prefix scan can't do range start)
        assert_eq!(ranges[0].start, "nostr:ki:030617:");
        assert!(ranges[0].end.is_none());
    }

    #[test]
    fn test_bounded_range_kind_with_until() {
        let filter = Filter::new().kind(Kind::Custom(30617)).until(Timestamp::from_secs(1700100000));
        let ranges = filter_scan_ranges(&filter);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, "nostr:ki:030617:");
        let expected_end = format!("nostr:ki:030617:0000001700100000{}", '\u{00ff}');
        assert_eq!(ranges[0].end.as_deref(), Some(expected_end.as_str()));
    }

    #[test]
    fn test_bounded_range_kind_with_since_and_until() {
        let filter = Filter::new()
            .kind(Kind::Custom(1))
            .since(Timestamp::from_secs(1000))
            .until(Timestamp::from_secs(2000));
        let ranges = filter_scan_ranges(&filter);
        assert_eq!(ranges.len(), 1);
        // since doesn't narrow prefix; until provides end bound
        assert_eq!(ranges[0].start, "nostr:ki:000001:");
        let expected_end = format!("nostr:ki:000001:0000000000002000{}", '\u{00ff}');
        assert_eq!(ranges[0].end.as_deref(), Some(expected_end.as_str()));
    }

    #[test]
    fn test_bounded_range_author_with_since() {
        let keys = Keys::generate();
        let filter = Filter::new().author(keys.public_key()).since(Timestamp::from_secs(5000));
        let ranges = filter_scan_ranges(&filter);
        assert_eq!(ranges.len(), 1);
        // since doesn't narrow prefix
        let expected_start = format!("nostr:au:{}:", keys.public_key().to_hex());
        assert_eq!(ranges[0].start, expected_start);
    }

    #[test]
    fn test_bounded_range_no_time_bounds() {
        let filter = Filter::new().kind(Kind::TextNote);
        let ranges = filter_scan_ranges(&filter);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start, "nostr:ki:000001:");
        assert!(ranges[0].end.is_none());
    }

    #[test]
    fn test_bounded_range_event_ids_ignores_time() {
        let id = EventId::from_hex("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        let filter = Filter::new().id(id).since(Timestamp::from_secs(1000));
        let ranges = filter_scan_ranges(&filter);
        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start.starts_with("nostr:ev:"));
        assert!(ranges[0].end.is_none());
    }

    #[test]
    fn test_bounded_range_tag_ignores_time() {
        let filter = Filter::new()
            .custom_tag(SingleLetterTag::lowercase(Alphabet::D), "my-repo")
            .since(Timestamp::from_secs(1000));
        let ranges = filter_scan_ranges(&filter);
        assert_eq!(ranges.len(), 1);
        assert!(ranges[0].start.starts_with("nostr:tg:d:"));
        assert!(ranges[0].end.is_none());
    }
}
