//! Verus specification for pure cluster rate-limiter decisions.
//!
//! The production module uses floating-point token rates for runtime buckets.
//! This proof slice verifies the deterministic integer/control-flow kernel around
//! that runtime shell: monotonic timestamps, eviction admission, token-consume
//! classification, and two-tier gossip admission ordering.

use vstd::prelude::*;

verus! {

pub struct BucketSpec {
    pub tokens: u64,
    pub last_update_ms: u64,
}

pub struct PeerRateSpec {
    pub bucket: BucketSpec,
    pub last_access_ms: u64,
}

pub enum ConsumeSpec {
    Consumed { remaining: u64 },
    Denied { available: u64 },
}

pub enum DenialReasonSpec {
    Global,
    PerPeer,
}

pub struct AdmissionSpec {
    pub allowed: bool,
    pub reason: Option<DenialReasonSpec>,
    pub next_global: BucketSpec,
    pub next_peer: Option<PeerRateSpec>,
}

pub open spec fn max_u64_spec(a: u64, b: u64) -> u64 {
    if a >= b { a } else { b }
}

pub fn monotonic_timestamp_ms_exec(previous_ms: u64, requested_ms: u64) -> (result: u64)
    ensures
        result == max_u64_spec(previous_ms, requested_ms),
        result >= previous_ms,
        result >= requested_ms,
{
    if previous_ms >= requested_ms { previous_ms } else { requested_ms }
}

pub fn update_last_access_ms_exec(previous_ms: u64, requested_ms: u64) -> (result: u64)
    ensures
        result == max_u64_spec(previous_ms, requested_ms),
        result >= previous_ms,
        result >= requested_ms,
{
    monotonic_timestamp_ms_exec(previous_ms, requested_ms)
}

pub open spec fn should_evict_oldest_spec(current_len: u32, max_capacity: u32) -> bool {
    current_len >= max_capacity
}

pub fn should_evict_oldest_exec(current_len: u32, max_capacity: u32) -> (result: bool)
    ensures result == should_evict_oldest_spec(current_len, max_capacity)
{
    current_len >= max_capacity
}

pub proof fn evicts_at_or_over_capacity(current_len: u32, max_capacity: u32)
    ensures
        should_evict_oldest_spec(current_len, max_capacity) <==> current_len >= max_capacity,
        !should_evict_oldest_spec(current_len, max_capacity) <==> current_len < max_capacity,
{
}

pub open spec fn can_consume_token_spec(available_tokens: u64) -> ConsumeSpec {
    if available_tokens >= 1 {
        ConsumeSpec::Consumed { remaining: (available_tokens - 1) as u64 }
    } else {
        ConsumeSpec::Denied { available: available_tokens }
    }
}

pub open spec fn is_consumed_spec(result: ConsumeSpec) -> bool {
    match result {
        ConsumeSpec::Consumed { remaining: _ } => true,
        ConsumeSpec::Denied { available: _ } => false,
    }
}

pub fn can_consume_token_exec(available_tokens: u64) -> (result: ConsumeSpec)
    ensures result == can_consume_token_spec(available_tokens)
{
    if available_tokens >= 1 {
        ConsumeSpec::Consumed { remaining: (available_tokens - 1) as u64 }
    } else {
        ConsumeSpec::Denied { available: available_tokens }
    }
}

pub proof fn consume_classification_truth_table(available_tokens: u64)
    ensures
        is_consumed_spec(can_consume_token_spec(available_tokens)) <==> available_tokens >= 1,
        !is_consumed_spec(can_consume_token_spec(available_tokens)) <==> available_tokens < 1,
{
}

pub open spec fn advance_bucket_control_spec(state: BucketSpec, now_ms: u64) -> BucketSpec {
    BucketSpec {
        tokens: state.tokens,
        last_update_ms: max_u64_spec(state.last_update_ms, now_ms),
    }
}

pub fn advance_bucket_control_exec(state: BucketSpec, now_ms: u64) -> (result: BucketSpec)
    ensures
        result == advance_bucket_control_spec(state, now_ms),
        result.tokens == state.tokens,
        result.last_update_ms >= state.last_update_ms,
        result.last_update_ms >= now_ms,
{
    BucketSpec {
        tokens: state.tokens,
        last_update_ms: monotonic_timestamp_ms_exec(state.last_update_ms, now_ms),
    }
}

pub open spec fn try_consume_control_spec(state: BucketSpec, now_ms: u64) -> (BucketSpec, ConsumeSpec) {
    let advanced = advance_bucket_control_spec(state, now_ms);
    let consume = can_consume_token_spec(advanced.tokens);
    let next_tokens = match consume {
        ConsumeSpec::Consumed { remaining } => remaining,
        ConsumeSpec::Denied { available } => available,
    };
    (BucketSpec { tokens: next_tokens, last_update_ms: advanced.last_update_ms }, consume)
}

pub fn try_consume_control_exec(state: BucketSpec, now_ms: u64) -> (result: (BucketSpec, ConsumeSpec))
    ensures result == try_consume_control_spec(state, now_ms)
{
    let advanced = advance_bucket_control_exec(state, now_ms);
    let consume = can_consume_token_exec(advanced.tokens);
    let next_tokens = match consume {
        ConsumeSpec::Consumed { remaining } => remaining,
        ConsumeSpec::Denied { available } => available,
    };
    (BucketSpec { tokens: next_tokens, last_update_ms: advanced.last_update_ms }, consume)
}

pub open spec fn existing_peer_admission_spec(
    global_state: BucketSpec,
    peer_state: PeerRateSpec,
    now_ms: u64,
) -> AdmissionSpec {
    let peer_transition = try_consume_control_spec(peer_state.bucket, now_ms);
    let next_peer = PeerRateSpec {
        bucket: peer_transition.0,
        last_access_ms: max_u64_spec(peer_state.last_access_ms, now_ms),
    };
    if !is_consumed_spec(peer_transition.1) {
        AdmissionSpec {
            allowed: false,
            reason: Some(DenialReasonSpec::PerPeer),
            next_global: advance_bucket_control_spec(global_state, now_ms),
            next_peer: Some(next_peer),
        }
    } else {
        let global_transition = try_consume_control_spec(global_state, now_ms);
        if !is_consumed_spec(global_transition.1) {
            AdmissionSpec {
                allowed: false,
                reason: Some(DenialReasonSpec::Global),
                next_global: global_transition.0,
                next_peer: None,
            }
        } else {
            AdmissionSpec {
                allowed: true,
                reason: None,
                next_global: global_transition.0,
                next_peer: Some(next_peer),
            }
        }
    }
}

pub fn existing_peer_admission_exec(
    global_state: BucketSpec,
    peer_state: PeerRateSpec,
    now_ms: u64,
) -> (result: AdmissionSpec)
    ensures result == existing_peer_admission_spec(global_state, peer_state, now_ms)
{
    let peer_transition = try_consume_control_exec(peer_state.bucket, now_ms);
    let next_peer = PeerRateSpec {
        bucket: peer_transition.0,
        last_access_ms: update_last_access_ms_exec(peer_state.last_access_ms, now_ms),
    };
    let peer_consumed = match peer_transition.1 {
        ConsumeSpec::Consumed { remaining: _ } => true,
        ConsumeSpec::Denied { available: _ } => false,
    };
    if !peer_consumed {
        AdmissionSpec {
            allowed: false,
            reason: Some(DenialReasonSpec::PerPeer),
            next_global: advance_bucket_control_exec(global_state, now_ms),
            next_peer: Some(next_peer),
        }
    } else {
        let global_transition = try_consume_control_exec(global_state, now_ms);
        let global_consumed = match global_transition.1 {
            ConsumeSpec::Consumed { remaining: _ } => true,
            ConsumeSpec::Denied { available: _ } => false,
        };
        if !global_consumed {
            AdmissionSpec {
                allowed: false,
                reason: Some(DenialReasonSpec::Global),
                next_global: global_transition.0,
                next_peer: None,
            }
        } else {
            AdmissionSpec {
                allowed: true,
                reason: None,
                next_global: global_transition.0,
                next_peer: Some(next_peer),
            }
        }
    }
}

pub proof fn existing_peer_denial_preserves_global_consumption(
    global_state: BucketSpec,
    peer_state: PeerRateSpec,
    now_ms: u64,
)
    requires peer_state.bucket.tokens == 0
    ensures
        existing_peer_admission_spec(global_state, peer_state, now_ms).allowed == false,
        existing_peer_admission_spec(global_state, peer_state, now_ms).reason == Some(DenialReasonSpec::PerPeer),
        existing_peer_admission_spec(global_state, peer_state, now_ms).next_global.tokens == global_state.tokens,
        matches!(existing_peer_admission_spec(global_state, peer_state, now_ms).next_peer, Some(_)),
{
}

pub proof fn existing_peer_global_denial_discards_peer_update(
    global_state: BucketSpec,
    peer_state: PeerRateSpec,
    now_ms: u64,
)
    requires
        peer_state.bucket.tokens >= 1,
        global_state.tokens == 0,
    ensures
        existing_peer_admission_spec(global_state, peer_state, now_ms).allowed == false,
        existing_peer_admission_spec(global_state, peer_state, now_ms).reason == Some(DenialReasonSpec::Global),
        matches!(existing_peer_admission_spec(global_state, peer_state, now_ms).next_peer, None),
{
}

pub proof fn existing_peer_success_consumes_both_budgets(
    global_state: BucketSpec,
    peer_state: PeerRateSpec,
    now_ms: u64,
)
    requires
        peer_state.bucket.tokens >= 1,
        global_state.tokens >= 1,
    ensures
        existing_peer_admission_spec(global_state, peer_state, now_ms).allowed == true,
        existing_peer_admission_spec(global_state, peer_state, now_ms).reason == None::<DenialReasonSpec>,
        existing_peer_admission_spec(global_state, peer_state, now_ms).next_global.tokens == global_state.tokens - 1,
        matches!(existing_peer_admission_spec(global_state, peer_state, now_ms).next_peer, Some(_)),
{
}

} // verus!
