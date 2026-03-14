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
        let mut prefixes = Vec::new();
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

/// Apply limit and sort ordering to query results.
///
/// Events are sorted by `created_at` descending (newest first),
/// then truncated to the limit.
pub fn apply_limit_and_sort(events: &mut Vec<Event>, limit: Option<usize>) {
    events.sort_by_key(|e| std::cmp::Reverse(e.created_at));
    if let Some(limit) = limit {
        events.truncate(limit);
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
}
