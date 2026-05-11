//! Queue visibility and expiration helper specifications.
//!
//! Formal specs for the pure helpers in
//! `src/verified/queue/{visibility,expiration}.rs`.

use vstd::prelude::*;

verus! {
    pub const MAX_VISIBILITY_EXTENSION_MS: u64 = 3_600_000;

    pub open spec fn saturating_add_u64_spec(lhs: u64, rhs: u64) -> u64 {
        if lhs as int + rhs as int > 0xFFFF_FFFF_FFFF_FFFFu64 as int {
            0xFFFF_FFFF_FFFF_FFFFu64
        } else {
            (lhs + rhs) as u64
        }
    }

    pub open spec fn saturating_sub_u64_spec(lhs: u64, rhs: u64) -> u64 {
        if lhs >= rhs {
            (lhs - rhs) as u64
        } else {
            0u64
        }
    }

    pub fn compute_visibility_deadline(
        current_time_ms: u64,
        visibility_timeout_ms: u64,
    ) -> (deadline: u64)
        ensures
            deadline == saturating_add_u64_spec(current_time_ms, visibility_timeout_ms),
            deadline >= current_time_ms,
            current_time_ms as int + visibility_timeout_ms as int <= 0xFFFF_FFFF_FFFF_FFFFu64 as int ==> deadline == current_time_ms + visibility_timeout_ms,
            current_time_ms as int + visibility_timeout_ms as int > 0xFFFF_FFFF_FFFF_FFFFu64 as int ==> deadline == 0xFFFF_FFFF_FFFF_FFFFu64,
    {
        current_time_ms.saturating_add(visibility_timeout_ms)
    }

    pub fn calculate_visibility_deadline(
        dequeue_time_ms: u64,
        visibility_timeout_ms: u64,
    ) -> (deadline: u64)
        ensures
            deadline == saturating_add_u64_spec(dequeue_time_ms, visibility_timeout_ms),
            deadline >= dequeue_time_ms,
    {
        compute_visibility_deadline(dequeue_time_ms, visibility_timeout_ms)
    }

    pub fn compute_effective_visibility_timeout(
        requested_ms: u64,
        max_timeout_ms: u64,
    ) -> (effective: u64)
        ensures
            effective == if requested_ms <= max_timeout_ms { requested_ms } else { max_timeout_ms },
            effective <= requested_ms,
            effective <= max_timeout_ms,
            requested_ms <= max_timeout_ms ==> effective == requested_ms,
            requested_ms > max_timeout_ms ==> effective == max_timeout_ms,
    {
        if requested_ms <= max_timeout_ms {
            requested_ms
        } else {
            max_timeout_ms
        }
    }

    pub open spec fn visibility_expired_spec(visibility_deadline_ms: u64, now_ms: u64) -> bool {
        now_ms > visibility_deadline_ms
    }

    pub fn is_visibility_expired(
        visibility_deadline_ms: u64,
        now_ms: u64,
    ) -> (expired: bool)
        ensures
            expired == visibility_expired_spec(visibility_deadline_ms, now_ms),
            expired ==> now_ms > visibility_deadline_ms,
            !expired ==> now_ms <= visibility_deadline_ms,
    {
        now_ms > visibility_deadline_ms
    }

    pub fn is_visibility_expired_exec(
        visibility_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (expired: bool)
        ensures expired == visibility_expired_spec(visibility_deadline_ms, current_time_ms)
    {
        is_visibility_expired(visibility_deadline_ms, current_time_ms)
    }

    pub fn is_visibility_timeout_expired(
        visibility_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (expired: bool)
        ensures expired == visibility_expired_spec(visibility_deadline_ms, current_time_ms)
    {
        is_visibility_expired(visibility_deadline_ms, current_time_ms)
    }

    pub fn time_until_visibility_expires(
        visibility_deadline_ms: u64,
        current_time_ms: u64,
    ) -> (remaining: u64)
        ensures
            remaining == saturating_sub_u64_spec(visibility_deadline_ms, current_time_ms),
            remaining == 0 <==> current_time_ms >= visibility_deadline_ms,
            current_time_ms < visibility_deadline_ms ==> remaining == visibility_deadline_ms - current_time_ms,
            remaining <= visibility_deadline_ms,
    {
        if visibility_deadline_ms >= current_time_ms {
            (visibility_deadline_ms - current_time_ms) as u64
        } else {
            0u64
        }
    }

    pub open spec fn queue_item_expired_spec(expires_at_ms: u64, now_ms: u64) -> bool {
        expires_at_ms > 0 && now_ms > expires_at_ms
    }

    pub fn is_queue_item_expired(expires_at_ms: u64, now_ms: u64) -> (expired: bool)
        ensures
            expired == queue_item_expired_spec(expires_at_ms, now_ms),
            expires_at_ms == 0 ==> !expired,
            expired ==> expires_at_ms > 0 && now_ms > expires_at_ms,
            expires_at_ms > 0 && now_ms <= expires_at_ms ==> !expired,
    {
        expires_at_ms > 0 && now_ms > expires_at_ms
    }

    pub fn is_item_expired(expires_at_ms: u64, current_time_ms: u64) -> (expired: bool)
        ensures expired == queue_item_expired_spec(expires_at_ms, current_time_ms)
    {
        is_queue_item_expired(expires_at_ms, current_time_ms)
    }

    pub open spec fn dedup_entry_expired_spec(expires_at_ms: u64, now_ms: u64) -> bool {
        now_ms > expires_at_ms
    }

    pub fn is_dedup_entry_expired(expires_at_ms: u64, now_ms: u64) -> (expired: bool)
        ensures expired == dedup_entry_expired_spec(expires_at_ms, now_ms)
    {
        now_ms > expires_at_ms
    }

    pub fn is_dedup_expired(dedup_expires_at_ms: u64, current_time_ms: u64) -> (expired: bool)
        ensures expired == dedup_entry_expired_spec(dedup_expires_at_ms, current_time_ms)
    {
        is_dedup_entry_expired(dedup_expires_at_ms, current_time_ms)
    }

    pub open spec fn effective_ttl_spec(ttl_ms: u64, default_ttl_ms: u64, max_ttl_ms: u64) -> u64 {
        let effective_ttl = if ttl_ms > 0 { ttl_ms } else { default_ttl_ms };
        if effective_ttl <= max_ttl_ms { effective_ttl } else { max_ttl_ms }
    }

    pub open spec fn item_expiration_spec(
        ttl_ms: u64,
        default_ttl_ms: u64,
        max_ttl_ms: u64,
        now_ms: u64,
    ) -> u64 {
        let capped_ttl = effective_ttl_spec(ttl_ms, default_ttl_ms, max_ttl_ms);
        if capped_ttl > 0 {
            saturating_add_u64_spec(now_ms, capped_ttl)
        } else {
            0u64
        }
    }

    pub fn compute_item_expiration(
        ttl_ms: u64,
        default_ttl_ms: u64,
        max_ttl_ms: u64,
        now_ms: u64,
    ) -> (expires_at: u64)
        ensures
            expires_at == item_expiration_spec(ttl_ms, default_ttl_ms, max_ttl_ms, now_ms),
            effective_ttl_spec(ttl_ms, default_ttl_ms, max_ttl_ms) == 0 ==> expires_at == 0,
            effective_ttl_spec(ttl_ms, default_ttl_ms, max_ttl_ms) > 0 ==> expires_at >= now_ms,
            effective_ttl_spec(ttl_ms, default_ttl_ms, max_ttl_ms) > 0 &&
                now_ms as int + effective_ttl_spec(ttl_ms, default_ttl_ms, max_ttl_ms) as int <= 0xFFFF_FFFF_FFFF_FFFFu64 as int ==>
                expires_at == now_ms + effective_ttl_spec(ttl_ms, default_ttl_ms, max_ttl_ms),
    {
        let effective_ttl = if ttl_ms > 0 { ttl_ms } else { default_ttl_ms };
        let capped_ttl = if effective_ttl <= max_ttl_ms { effective_ttl } else { max_ttl_ms };

        if capped_ttl > 0 {
            now_ms.saturating_add(capped_ttl)
        } else {
            0u64
        }
    }

    pub fn can_compute_ttl(current_time_ms: u64, ttl_ms: u64) -> (can_compute: bool)
        ensures
            can_compute == (ttl_ms == 0 || current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - ttl_ms),
            can_compute && ttl_ms > 0 ==> saturating_add_u64_spec(current_time_ms, ttl_ms) == current_time_ms + ttl_ms,
            !can_compute ==> ttl_ms > 0 && saturating_add_u64_spec(current_time_ms, ttl_ms) == 0xFFFF_FFFF_FFFF_FFFFu64,
    {
        ttl_ms == 0 || current_time_ms <= 0xFFFF_FFFF_FFFF_FFFFu64 - ttl_ms
    }

    pub open spec fn can_extend_visibility_spec(
        is_inflight: bool,
        receipt_matches: bool,
        additional_timeout_ms: u64,
    ) -> bool {
        is_inflight && receipt_matches && additional_timeout_ms > 0 && additional_timeout_ms <= MAX_VISIBILITY_EXTENSION_MS
    }

    pub fn can_extend_visibility(
        is_inflight: bool,
        receipt_matches: bool,
        additional_timeout_ms: u64,
    ) -> (can_extend: bool)
        ensures
            can_extend == can_extend_visibility_spec(is_inflight, receipt_matches, additional_timeout_ms),
            can_extend ==> is_inflight && receipt_matches,
            can_extend ==> 0 < additional_timeout_ms <= MAX_VISIBILITY_EXTENSION_MS,
            !is_inflight ==> !can_extend,
            !receipt_matches ==> !can_extend,
            additional_timeout_ms == 0 ==> !can_extend,
            additional_timeout_ms > MAX_VISIBILITY_EXTENSION_MS ==> !can_extend,
    {
        let message_eligible = is_inflight && receipt_matches;
        let extension_valid = additional_timeout_ms > 0 && additional_timeout_ms <= MAX_VISIBILITY_EXTENSION_MS;
        message_eligible && extension_valid
    }

    pub open spec fn extend_visibility_valid_spec(
        current_deadline_ms: u64,
        requested_extension_ms: u64,
        max_visibility_ms: u64,
        current_time_ms: u64,
    ) -> bool {
        requested_extension_ms <= max_visibility_ms && current_deadline_ms > current_time_ms
    }

    pub fn is_extend_visibility_valid(
        current_deadline_ms: u64,
        requested_extension_ms: u64,
        max_visibility_ms: u64,
        current_time_ms: u64,
    ) -> (valid: bool)
        ensures
            valid == extend_visibility_valid_spec(current_deadline_ms, requested_extension_ms, max_visibility_ms, current_time_ms),
            valid ==> requested_extension_ms <= max_visibility_ms,
            valid ==> current_deadline_ms > current_time_ms,
            current_deadline_ms <= current_time_ms ==> !valid,
            requested_extension_ms > max_visibility_ms ==> !valid,
    {
        requested_extension_ms <= max_visibility_ms && current_deadline_ms > current_time_ms
    }

    pub fn calculate_extended_deadline(current_time_ms: u64, extension_ms: u64) -> (deadline: u64)
        ensures
            deadline == saturating_add_u64_spec(current_time_ms, extension_ms),
            deadline >= current_time_ms,
    {
        compute_visibility_deadline(current_time_ms, extension_ms)
    }

    pub fn compute_extended_deadline(current_time_ms: u64, additional_timeout_ms: u64) -> (deadline: u64)
        ensures
            deadline == saturating_add_u64_spec(current_time_ms, additional_timeout_ms),
            deadline >= current_time_ms,
    {
        compute_visibility_deadline(current_time_ms, additional_timeout_ms)
    }

    pub proof fn no_expiration_zero_ttl(default_ttl_ms: u64, max_ttl_ms: u64, now_ms: u64)
        requires
            default_ttl_ms == 0 || max_ttl_ms == 0,
        ensures
            item_expiration_spec(0u64, default_ttl_ms, max_ttl_ms, now_ms) == 0u64,
    {
    }

    pub proof fn expired_items_must_have_deadline(expires_at_ms: u64, now_ms: u64)
        ensures
            queue_item_expired_spec(expires_at_ms, now_ms) ==> expires_at_ms > 0,
            queue_item_expired_spec(0u64, now_ms) == false,
    {
    }

    pub proof fn zero_extension_never_valid(is_inflight: bool, receipt_matches: bool)
        ensures
            !can_extend_visibility_spec(is_inflight, receipt_matches, 0u64),
    {
    }
}
