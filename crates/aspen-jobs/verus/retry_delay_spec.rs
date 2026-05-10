//! Verus specification for pure retry-delay helper functions.
//!
//! This verifies the integer/indexing parts of `src/verified/retry.rs`:
//! fixed-delay admission, custom-delay lookup, retry-limit checks, and the
//! cap semantics shared by exponential retry delay calculation. Floating-point
//! multiplier evaluation remains a runtime boundary; the post-cap admission
//! kernel is verified here.

use vstd::prelude::*;

verus! {

pub open spec fn fixed_retry_delay_spec(delay_ms: u64, attempt: u32, max_attempts: u32) -> Option<u64> {
    if attempt > max_attempts { None } else { Some(delay_ms) }
}

pub open spec fn retry_limit_exceeded_spec(attempts: u32, max_attempts: u32) -> bool {
    attempts > max_attempts
}

pub open spec fn custom_retry_index_spec(attempt: u32) -> int {
    if attempt == 0 { 0 } else { (attempt - 1) as int }
}

pub open spec fn custom_retry_delay_spec(delays: Seq<u64>, attempt: u32, fallback_ms: u64) -> u64 {
    let index = custom_retry_index_spec(attempt);
    if 0 <= index < delays.len() {
        delays[index]
    } else {
        fallback_ms
    }
}

pub open spec fn cap_delay_spec(delay: u64, max_delay_ms: Option<u64>) -> u64 {
    match max_delay_ms {
        Some(max) => if delay <= max { delay } else { max },
        None => delay,
    }
}

pub fn compute_fixed_retry_delay_ms_spec(delay_ms: u64, attempt: u32, max_attempts: u32) -> (result: Option<u64>)
    ensures result == fixed_retry_delay_spec(delay_ms, attempt, max_attempts)
{
    if attempt > max_attempts { None } else { Some(delay_ms) }
}

pub fn has_exceeded_retry_limit_spec(attempts: u32, max_attempts: u32) -> (result: bool)
    ensures result == retry_limit_exceeded_spec(attempts, max_attempts)
{
    attempts > max_attempts
}

pub fn custom_retry_index(attempt: u32) -> (index: usize)
    ensures index as int == custom_retry_index_spec(attempt)
{
    if attempt == 0 {
        0
    } else {
        (attempt - 1) as usize
    }
}

pub fn get_custom_retry_delay_ms_spec(delays_ms: &[u64], attempt: u32, fallback_ms: u64) -> (result: u64)
    ensures result == custom_retry_delay_spec(delays_ms@, attempt, fallback_ms)
{
    let index = custom_retry_index(attempt);
    if index < delays_ms.len() {
        delays_ms[index]
    } else {
        fallback_ms
    }
}

pub fn cap_retry_delay(delay: u64, max_delay_ms: Option<u64>) -> (result: u64)
    ensures
        result == cap_delay_spec(delay, max_delay_ms),
        max_delay_ms.is_some() ==> result <= max_delay_ms.unwrap(),
        max_delay_ms.is_none() ==> result == delay,
        result <= delay || max_delay_ms.is_none(),
{
    match max_delay_ms {
        Some(max) => if delay <= max { delay } else { max },
        None => delay,
    }
}

pub proof fn fixed_retry_some_iff_within_limit(delay_ms: u64, attempt: u32, max_attempts: u32)
    ensures fixed_retry_delay_spec(delay_ms, attempt, max_attempts).is_some() == (attempt <= max_attempts)
{
}

pub proof fn fixed_retry_none_iff_exceeded(delay_ms: u64, attempt: u32, max_attempts: u32)
    ensures fixed_retry_delay_spec(delay_ms, attempt, max_attempts).is_none() == retry_limit_exceeded_spec(attempt, max_attempts)
{
}

pub proof fn custom_retry_zero_uses_first_when_present(delays: Seq<u64>, fallback_ms: u64)
    requires delays.len() > 0
    ensures custom_retry_delay_spec(delays, 0, fallback_ms) == delays[0]
{
}

pub proof fn custom_retry_out_of_range_uses_fallback(delays: Seq<u64>, attempt: u32, fallback_ms: u64)
    requires custom_retry_index_spec(attempt) >= delays.len()
    ensures custom_retry_delay_spec(delays, attempt, fallback_ms) == fallback_ms
{
}

pub proof fn capped_retry_never_exceeds_max(delay: u64, max: u64)
    ensures cap_delay_spec(delay, Some(max)) <= max
{
}

pub proof fn uncapped_retry_preserves_delay(delay: u64)
    ensures cap_delay_spec(delay, None) == delay
{
}

}
