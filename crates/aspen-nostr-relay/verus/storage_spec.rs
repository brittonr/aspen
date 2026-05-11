//! Verus specs for Nostr relay storage scalar helpers.
//!
//! Production storage in `src/storage.rs` performs async KV I/O, JSON
//! serialization, and Nostr tag/key formatting. This module verifies the pure
//! admission/counting kernel: replaceable-kind classification, CAS retry budget
//! shape, saturating event-count updates, and the count effect of store/delete
//! decisions around duplicate, replacement, and eviction cases.

use vstd::prelude::*;

verus! {

pub const KIND_METADATA: u16 = 0;
pub const KIND_CONTACT_LIST: u16 = 3;
pub const REPLACEABLE_KIND_START: u16 = 10_000;
pub const REPLACEABLE_KIND_END: u16 = 20_000;
pub const PARAMETERIZED_REPLACEABLE_KIND_START: u16 = 30_000;
pub const PARAMETERIZED_REPLACEABLE_KIND_END: u16 = 40_000;
pub const MAX_STORED_EVENTS: u32 = 100_000;
pub const MAX_CAS_RETRIES: u32 = 5;

pub open spec fn is_replaceable_kind(kind: u16) -> bool {
    kind == KIND_METADATA
        || kind == KIND_CONTACT_LIST
        || (REPLACEABLE_KIND_START <= kind && kind < REPLACEABLE_KIND_END)
}

pub open spec fn is_parameterized_replaceable_kind(kind: u16) -> bool {
    PARAMETERIZED_REPLACEABLE_KIND_START <= kind && kind < PARAMETERIZED_REPLACEABLE_KIND_END
}

pub open spec fn count_increment(count: u32) -> u32 {
    if count == u32::MAX { u32::MAX } else { (count + 1) as u32 }
}

pub open spec fn count_decrement(count: u32) -> u32 {
    if count == 0 { 0 } else { (count - 1) as u32 }
}

pub open spec fn should_evict_before_insert(count: u32) -> bool {
    count >= MAX_STORED_EVENTS
}

pub open spec fn count_after_successful_insert(count: u32) -> u32 {
    if should_evict_before_insert(count) {
        count_increment(count_decrement(count))
    } else {
        count_increment(count)
    }
}

pub open spec fn store_count_result(
    count: u32,
    duplicate: bool,
    replaced_existing: bool,
) -> u32 {
    if duplicate {
        count
    } else if replaced_existing {
        count_after_successful_insert(count_decrement(count))
    } else {
        count_after_successful_insert(count)
    }
}

pub open spec fn delete_count_result(count: u32, existed: bool) -> u32 {
    if existed { count_decrement(count) } else { count }
}

pub open spec fn cas_attempts_exhausted(failed_attempts: u32) -> bool {
    failed_attempts >= MAX_CAS_RETRIES
}

pub open spec fn cas_can_retry(failed_attempts: u32) -> bool {
    failed_attempts < MAX_CAS_RETRIES
}

pub fn is_replaceable_kind_exec(kind: u16) -> (replaceable: bool)
    ensures replaceable == is_replaceable_kind(kind)
{
    kind == KIND_METADATA
        || kind == KIND_CONTACT_LIST
        || (kind >= REPLACEABLE_KIND_START && kind < REPLACEABLE_KIND_END)
}

pub fn is_parameterized_replaceable_kind_exec(kind: u16) -> (replaceable: bool)
    ensures replaceable == is_parameterized_replaceable_kind(kind)
{
    kind >= PARAMETERIZED_REPLACEABLE_KIND_START && kind < PARAMETERIZED_REPLACEABLE_KIND_END
}

pub fn count_increment_exec(count: u32) -> (next: u32)
    ensures next == count_increment(count), next >= count
{
    count.saturating_add(1)
}

pub fn count_decrement_exec(count: u32) -> (next: u32)
    ensures next == count_decrement(count), next <= count
{
    count.saturating_sub(1)
}

pub fn should_evict_before_insert_exec(count: u32) -> (evict: bool)
    ensures evict == should_evict_before_insert(count)
{
    count >= MAX_STORED_EVENTS
}

pub fn delete_count_result_exec(count: u32, existed: bool) -> (next: u32)
    ensures next == delete_count_result(count, existed), next <= count
{
    if existed { count_decrement_exec(count) } else { count }
}

pub proof fn default_storage_bounds_are_positive()
    ensures MAX_STORED_EVENTS > 0, MAX_CAS_RETRIES > 0
{
}

pub proof fn replaceable_fixed_kinds_are_admitted()
    ensures
        is_replaceable_kind(KIND_METADATA),
        is_replaceable_kind(KIND_CONTACT_LIST),
{
}

pub proof fn replaceable_range_start_is_admitted()
    ensures is_replaceable_kind(REPLACEABLE_KIND_START)
{
}

pub proof fn replaceable_range_end_is_excluded()
    ensures !is_replaceable_kind(REPLACEABLE_KIND_END)
{
}

pub proof fn parameterized_range_start_is_admitted()
    ensures is_parameterized_replaceable_kind(PARAMETERIZED_REPLACEABLE_KIND_START)
{
}

pub proof fn parameterized_range_end_is_excluded()
    ensures !is_parameterized_replaceable_kind(PARAMETERIZED_REPLACEABLE_KIND_END)
{
}

pub proof fn replaceable_classes_are_disjoint(kind: u16)
    requires is_replaceable_kind(kind)
    ensures !is_parameterized_replaceable_kind(kind)
{
}

pub proof fn parameterized_classes_are_disjoint(kind: u16)
    requires is_parameterized_replaceable_kind(kind)
    ensures !is_replaceable_kind(kind)
{
}

pub proof fn increment_saturates_at_u32_max()
    ensures count_increment(u32::MAX) == u32::MAX
{
}

pub proof fn increment_adds_one_below_max(count: u32)
    requires count < u32::MAX
    ensures count_increment(count) == count + 1
{
}

pub proof fn decrement_saturates_at_zero()
    ensures count_decrement(0) == 0
{
}

pub proof fn decrement_subtracts_one_above_zero(count: u32)
    requires count > 0
    ensures count_decrement(count) == count - 1
{
}

pub proof fn duplicate_store_preserves_count(count: u32, replaced_existing: bool)
    ensures store_count_result(count, true, replaced_existing) == count
{
}

pub proof fn delete_missing_preserves_count(count: u32)
    ensures delete_count_result(count, false) == count
{
}

pub proof fn delete_existing_never_increases_count(count: u32)
    ensures delete_count_result(count, true) <= count
{
}

pub proof fn store_new_below_limit_increments(count: u32)
    requires count < MAX_STORED_EVENTS, count < u32::MAX
    ensures store_count_result(count, false, false) == count + 1
{
}

pub proof fn store_at_limit_preserves_count_after_eviction()
    ensures store_count_result(MAX_STORED_EVENTS, false, false) == MAX_STORED_EVENTS
{
}

pub proof fn replacing_single_existing_preserves_count_below_limit(count: u32)
    requires count > 0, count <= MAX_STORED_EVENTS, count < u32::MAX
    ensures store_count_result(count, false, true) == count
{
}

pub proof fn eviction_gate_matches_limit(count: u32)
    ensures should_evict_before_insert(count) == (count >= MAX_STORED_EVENTS)
{
}

pub proof fn cas_retry_budget_not_exhausted_initially()
    ensures cas_can_retry(0), !cas_attempts_exhausted(0)
{
}

pub proof fn cas_retry_budget_exhausts_at_limit()
    ensures cas_attempts_exhausted(MAX_CAS_RETRIES), !cas_can_retry(MAX_CAS_RETRIES)
{
}

pub proof fn cas_failed_attempts_partition(failed_attempts: u32)
    ensures cas_can_retry(failed_attempts) == !cas_attempts_exhausted(failed_attempts)
{
}

} // verus!
