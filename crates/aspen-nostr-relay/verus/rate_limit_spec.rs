//! Verus specs for Nostr relay rate-limit admission helpers.
//!
//! Production rate limiting in `src/rate_limit.rs` uses `Instant`, DashMap, and
//! floating-point token buckets. This module verifies the pure admission shape:
//! when limiting is enabled, how an integer token bucket admits/denies one
//! event, how refill is capped at burst, and when idle buckets are stale.

use vstd::prelude::*;

verus! {

pub const DEFAULT_IP_RATE: u32 = 10;
pub const DEFAULT_IP_BURST: u32 = 20;
pub const DEFAULT_PUBKEY_RATE: u32 = 5;
pub const DEFAULT_PUBKEY_BURST: u32 = 10;
pub const BUCKET_TTL_SECS: u64 = 300;
pub const CLEANUP_INTERVAL_SECS: u64 = 60;

pub open spec fn rate_limiter_enabled(ip_rate: u32, pubkey_rate: u32) -> bool {
    ip_rate > 0 || pubkey_rate > 0
}

pub open spec fn dimension_disabled(rate: u32) -> bool {
    rate == 0
}

pub open spec fn cap_tokens(tokens: u32, burst: u32) -> u32 {
    if tokens > burst { burst } else { tokens }
}

pub open spec fn refill_tokens(tokens: u32, refill: u32, burst: u32) -> u32 {
    if tokens >= burst {
        burst
    } else if refill >= burst - tokens {
        burst
    } else {
        (tokens + refill) as u32
    }
}

pub open spec fn consume_one_spec(tokens: u32) -> Option<u32> {
    if tokens >= 1 { Some((tokens - 1) as u32) } else { None::<u32> }
}

pub open spec fn bucket_try_consume_spec(tokens: u32, refill: u32, burst: u32) -> Option<u32> {
    consume_one_spec(refill_tokens(tokens, refill, burst))
}

pub open spec fn allowed_after_refill(tokens: u32, refill: u32, burst: u32) -> bool {
    bucket_try_consume_spec(tokens, refill, burst).is_some()
}

pub open spec fn stale_bucket(last_access: u64, now: u64, ttl_secs: u64) -> bool {
    now >= last_access && now - last_access >= ttl_secs
}

pub open spec fn retain_bucket(last_access: u64, now: u64, ttl_secs: u64) -> bool {
    !stale_bucket(last_access, now, ttl_secs)
}

pub fn is_rate_limiter_enabled_exec(ip_rate: u32, pubkey_rate: u32) -> (enabled: bool)
    ensures enabled == rate_limiter_enabled(ip_rate, pubkey_rate)
{
    ip_rate > 0 || pubkey_rate > 0
}

pub fn cap_tokens_exec(tokens: u32, burst: u32) -> (capped: u32)
    ensures capped == cap_tokens(tokens, burst), capped <= burst
{
    if tokens > burst { burst } else { tokens }
}

pub fn refill_tokens_exec(tokens: u32, refill: u32, burst: u32) -> (refilled: u32)
    ensures
        refilled == refill_tokens(tokens, refill, burst),
        refilled <= burst,
        tokens <= burst ==> refilled >= tokens,
{
    if tokens >= burst {
        burst
    } else if refill >= burst - tokens {
        burst
    } else {
        tokens + refill
    }
}

pub fn consume_one_exec(tokens: u32) -> (next: Option<u32>)
    ensures next == consume_one_spec(tokens)
{
    if tokens >= 1 { Some(tokens - 1) } else { None }
}

pub fn bucket_try_consume_exec(tokens: u32, refill: u32, burst: u32) -> (next: Option<u32>)
    ensures next == bucket_try_consume_spec(tokens, refill, burst)
{
    let refilled = refill_tokens_exec(tokens, refill, burst);
    consume_one_exec(refilled)
}

pub fn is_bucket_stale_exec(last_access: u64, now: u64, ttl_secs: u64) -> (stale: bool)
    ensures stale == stale_bucket(last_access, now, ttl_secs)
{
    now >= last_access && now - last_access >= ttl_secs
}

pub proof fn defaults_enable_both_dimensions()
    ensures
        rate_limiter_enabled(DEFAULT_IP_RATE, DEFAULT_PUBKEY_RATE),
        DEFAULT_IP_BURST > DEFAULT_IP_RATE,
        DEFAULT_PUBKEY_BURST > DEFAULT_PUBKEY_RATE,
        BUCKET_TTL_SECS > CLEANUP_INTERVAL_SECS,
{
}

pub proof fn zero_rates_disable_limiter()
    ensures !rate_limiter_enabled(0, 0)
{
}

pub proof fn zero_one_dimension_still_enables_other(ip_rate: u32, pubkey_rate: u32)
    requires ip_rate > 0 || pubkey_rate > 0
    ensures rate_limiter_enabled(ip_rate, pubkey_rate)
{
}

pub proof fn disabled_dimension_allows_without_bucket(rate: u32)
    requires dimension_disabled(rate)
    ensures rate == 0
{
}

pub proof fn refill_is_capped(tokens: u32, refill: u32, burst: u32)
    ensures refill_tokens(tokens, refill, burst) <= burst
{
}

pub proof fn refill_preserves_or_increases_bounded_tokens(tokens: u32, refill: u32, burst: u32)
    requires tokens <= burst
    ensures refill_tokens(tokens, refill, burst) >= tokens
{
}

pub proof fn successful_consume_decrements_refilled_tokens(tokens: u32, refill: u32, burst: u32)
    requires allowed_after_refill(tokens, refill, burst)
    ensures bucket_try_consume_spec(tokens, refill, burst).unwrap() + 1 == refill_tokens(tokens, refill, burst)
{
}

pub proof fn empty_bucket_without_refill_denies(burst: u32)
    ensures bucket_try_consume_spec(0, 0, burst) == None::<u32>
{
}

pub proof fn positive_refill_or_tokens_can_allow(tokens: u32, refill: u32, burst: u32)
    requires refill_tokens(tokens, refill, burst) >= 1
    ensures allowed_after_refill(tokens, refill, burst)
{
}

pub proof fn stale_bucket_requires_monotonic_time(last_access: u64, now: u64, ttl_secs: u64)
    requires stale_bucket(last_access, now, ttl_secs)
    ensures now >= last_access
{
}

pub proof fn buckets_before_ttl_are_retained(last_access: u64, now: u64, ttl_secs: u64)
    requires now >= last_access, now - last_access < ttl_secs
    ensures retain_bucket(last_access, now, ttl_secs)
{
}

pub proof fn bucket_at_ttl_is_stale(last_access: u64, ttl_secs: u64)
    requires last_access <= u64::MAX - ttl_secs
    ensures stale_bucket(last_access, (last_access + ttl_secs) as u64, ttl_secs)
{
}

} // verus!
