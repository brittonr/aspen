//! Verus specs for Nostr relay filter scan selection and bounded ranges.
//!
//! The production code in `src/filters.rs` maps NIP-01 filters to KV scan
//! prefixes/ranges, then relies on `Filter::match_event()` for precise event
//! admission. This module verifies the pure scalar/control-flow contract: scan
//! source priority, empty-tag fallback, timestamp range admission, and limit
//! truncation shape.

use vstd::prelude::*;

verus! {

pub const PADDED_TIMESTAMP_LEN: u64 = 16;
pub const END_SENTINEL_UTF8_LEN: u64 = 2;

pub enum ScanSource {
    EventIds,
    Kinds,
    Authors,
    Tags,
    FullScan,
}

pub struct RangeShape {
    pub start_len: u64,
    pub end_len: Option<u64>,
}

pub open spec fn source_is_event_ids(source: ScanSource) -> bool {
    matches!(source, ScanSource::EventIds)
}

pub open spec fn source_is_kinds(source: ScanSource) -> bool {
    matches!(source, ScanSource::Kinds)
}

pub open spec fn source_is_authors(source: ScanSource) -> bool {
    matches!(source, ScanSource::Authors)
}

pub open spec fn source_is_tags(source: ScanSource) -> bool {
    matches!(source, ScanSource::Tags)
}

pub open spec fn source_is_full_scan(source: ScanSource) -> bool {
    matches!(source, ScanSource::FullScan)
}

pub open spec fn choose_scan_source(
    has_ids: bool,
    has_kinds: bool,
    has_authors: bool,
    tag_value_count: u32,
) -> ScanSource {
    if has_ids {
        ScanSource::EventIds
    } else if has_kinds {
        ScanSource::Kinds
    } else if has_authors {
        ScanSource::Authors
    } else if tag_value_count > 0 {
        ScanSource::Tags
    } else {
        ScanSource::FullScan
    }
}

pub open spec fn source_uses_bounded_range(source: ScanSource) -> bool {
    source_is_kinds(source) || source_is_authors(source)
}

pub open spec fn source_ignores_time_bounds(source: ScanSource) -> bool {
    source_is_event_ids(source) || source_is_tags(source) || source_is_full_scan(source)
}

pub open spec fn result_count_for_source(source: ScanSource, item_count: u32, tag_value_count: u32) -> u32 {
    match source {
        ScanSource::EventIds => item_count,
        ScanSource::Kinds => item_count,
        ScanSource::Authors => item_count,
        ScanSource::Tags => tag_value_count,
        ScanSource::FullScan => 1,
    }
}

pub open spec fn bounded_range_shape(prefix_len: u64, until_present: bool) -> RangeShape {
    RangeShape {
        start_len: prefix_len,
        end_len: if until_present {
            Some((prefix_len + PADDED_TIMESTAMP_LEN + END_SENTINEL_UTF8_LEN) as u64)
        } else {
            None::<u64>
        },
    }
}

pub open spec fn has_end_bound(range: RangeShape) -> bool {
    range.end_len.is_some()
}

pub open spec fn range_preserves_start_prefix(prefix_len: u64, until_present: bool) -> bool {
    bounded_range_shape(prefix_len, until_present).start_len == prefix_len
}

pub open spec fn truncate_len(input_len: u64, limit: Option<u32>) -> u64 {
    match limit {
        Some(limit_value) => if input_len <= limit_value as u64 { input_len } else { limit_value as u64 },
        None => input_len,
    }
}

pub fn choose_scan_source_exec(
    has_ids: bool,
    has_kinds: bool,
    has_authors: bool,
    tag_value_count: u32,
) -> (source: ScanSource)
    ensures source == choose_scan_source(has_ids, has_kinds, has_authors, tag_value_count)
{
    if has_ids {
        ScanSource::EventIds
    } else if has_kinds {
        ScanSource::Kinds
    } else if has_authors {
        ScanSource::Authors
    } else if tag_value_count > 0 {
        ScanSource::Tags
    } else {
        ScanSource::FullScan
    }
}

pub fn bounded_range_shape_exec(prefix_len: u64, until_present: bool) -> (range: RangeShape)
    requires prefix_len <= u64::MAX - PADDED_TIMESTAMP_LEN - END_SENTINEL_UTF8_LEN
    ensures
        range.start_len == prefix_len,
        range.end_len == bounded_range_shape(prefix_len, until_present).end_len,
        has_end_bound(range) == until_present,
{
    RangeShape {
        start_len: prefix_len,
        end_len: if until_present {
            Some(prefix_len + PADDED_TIMESTAMP_LEN + END_SENTINEL_UTF8_LEN)
        } else {
            None
        },
    }
}

pub fn truncate_len_exec(input_len: u64, limit: Option<u32>) -> (len: u64)
    ensures
        len == truncate_len(input_len, limit),
        len <= input_len,
        match limit { Some(limit_value) => len <= limit_value as u64, None => len == input_len },
{
    match limit {
        Some(limit_value) => {
            let limit_u64 = limit_value as u64;
            if input_len <= limit_u64 { input_len } else { limit_u64 }
        },
        None => input_len,
    }
}

pub proof fn event_ids_have_highest_scan_priority(
    has_kinds: bool,
    has_authors: bool,
    tag_value_count: u32,
)
    ensures source_is_event_ids(choose_scan_source(true, has_kinds, has_authors, tag_value_count))
{
}

pub proof fn kinds_beat_authors_and_tags(has_authors: bool, tag_value_count: u32)
    ensures source_is_kinds(choose_scan_source(false, true, has_authors, tag_value_count))
{
}

pub proof fn authors_beat_tags(tag_value_count: u32)
    ensures source_is_authors(choose_scan_source(false, false, true, tag_value_count))
{
}

pub proof fn nonempty_tags_beat_full_scan(tag_value_count: u32)
    requires tag_value_count > 0
    ensures source_is_tags(choose_scan_source(false, false, false, tag_value_count))
{
}

pub proof fn empty_filter_uses_full_scan()
    ensures source_is_full_scan(choose_scan_source(false, false, false, 0))
{
}

pub proof fn only_kind_and_author_sources_use_bounded_ranges(source: ScanSource)
    ensures source_uses_bounded_range(source) == (source_is_kinds(source) || source_is_authors(source))
{
}

pub proof fn direct_event_and_tag_scans_ignore_time_bounds(source: ScanSource)
    requires source_ignores_time_bounds(source)
    ensures !source_uses_bounded_range(source)
{
}

pub proof fn bounded_range_since_does_not_change_start(prefix_len: u64, until_present: bool)
    ensures range_preserves_start_prefix(prefix_len, until_present)
{
}

pub proof fn until_presence_controls_end_bound(prefix_len: u64, until_present: bool)
    ensures has_end_bound(bounded_range_shape(prefix_len, until_present)) == until_present
{
}

pub proof fn until_end_extends_prefix(prefix_len: u64)
    requires prefix_len <= u64::MAX - PADDED_TIMESTAMP_LEN - END_SENTINEL_UTF8_LEN
    ensures bounded_range_shape(prefix_len, true).end_len.unwrap() > prefix_len
{
}

pub proof fn no_until_has_no_end(prefix_len: u64)
    ensures bounded_range_shape(prefix_len, false).end_len == None::<u64>
{
}

pub proof fn full_scan_has_single_prefix(item_count: u32, tag_value_count: u32)
    ensures result_count_for_source(ScanSource::FullScan, item_count, tag_value_count) == 1
{
}

pub proof fn tag_result_count_tracks_tag_values(item_count: u32, tag_value_count: u32)
    ensures result_count_for_source(ScanSource::Tags, item_count, tag_value_count) == tag_value_count
{
}

pub proof fn id_kind_author_result_count_tracks_items(source: ScanSource, item_count: u32, tag_value_count: u32)
    requires source_is_event_ids(source) || source_is_kinds(source) || source_is_authors(source)
    ensures result_count_for_source(source, item_count, tag_value_count) == item_count
{
}

pub proof fn limit_none_preserves_length(input_len: u64)
    ensures truncate_len(input_len, None::<u32>) == input_len
{
}

pub proof fn limit_some_never_expands(input_len: u64, limit: u32)
    ensures truncate_len(input_len, Some(limit)) <= input_len
{
}

pub proof fn limit_some_respects_bound(input_len: u64, limit: u32)
    ensures truncate_len(input_len, Some(limit)) <= limit as u64
{
}

pub proof fn zero_limit_returns_zero(input_len: u64)
    ensures truncate_len(input_len, Some(0)) == 0
{
}

} // verus!
