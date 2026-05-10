use vstd::prelude::*;

verus! {

pub const MILLIS_PER_SECOND: u64 = 1000;
pub const U64_MAX_VALUE: u64 = 18446744073709551615u64;

pub open spec fn secs_to_ms_spec(secs: u64) -> u64 {
    if secs as int > U64_MAX_VALUE as int / MILLIS_PER_SECOND as int {
        U64_MAX_VALUE
    } else {
        (secs * MILLIS_PER_SECOND) as u64
    }
}

pub open spec fn deadline_spec(start_time_ms: u64, timeout_secs: u64) -> u64 {
    let timeout_ms = secs_to_ms_spec(timeout_secs);
    if start_time_ms as int + timeout_ms as int > U64_MAX_VALUE as int {
        U64_MAX_VALUE
    } else {
        (start_time_ms + timeout_ms) as u64
    }
}

pub open spec fn deadline_exceeded_spec(deadline_ms: u64, now_ms: u64) -> bool {
    now_ms >= deadline_ms
}

pub open spec fn remaining_time_spec(deadline_ms: u64, now_ms: u64) -> u64 {
    if now_ms >= deadline_ms {
        0
    } else {
        (deadline_ms - now_ms) as u64
    }
}

pub open spec fn option_u64_is_some(value: Option<u64>) -> bool {
    match value {
        Some(_) => true,
        None => false,
    }
}

pub open spec fn option_u64_is_none(value: Option<u64>) -> bool {
    match value {
        Some(_) => false,
        None => true,
    }
}

pub open spec fn effective_timeout_spec(
    user_timeout_secs: Option<u64>,
    default_timeout_secs: u64,
    max_timeout_secs: u64,
) -> u64 {
    match user_timeout_secs {
        Some(timeout) => if timeout > max_timeout_secs { max_timeout_secs } else { timeout },
        None => default_timeout_secs,
    }
}

pub fn secs_to_ms_exec(secs: u64) -> (result: u64)
    ensures
        result == secs_to_ms_spec(secs),
        result >= secs,
        result == U64_MAX_VALUE || result == secs * MILLIS_PER_SECOND,
{
    if secs > U64_MAX_VALUE / MILLIS_PER_SECOND {
        U64_MAX_VALUE
    } else {
        secs * MILLIS_PER_SECOND
    }
}

pub fn compute_deadline_ms_exec(start_time_ms: u64, timeout_secs: u64) -> (result: u64)
    ensures
        result == deadline_spec(start_time_ms, timeout_secs),
        result >= start_time_ms,
        result == U64_MAX_VALUE || result == start_time_ms + secs_to_ms_spec(timeout_secs),
{
    let timeout_ms = secs_to_ms_exec(timeout_secs);
    if start_time_ms > U64_MAX_VALUE - timeout_ms {
        U64_MAX_VALUE
    } else {
        start_time_ms + timeout_ms
    }
}

pub fn is_deadline_exceeded_exec(deadline_ms: u64, now_ms: u64) -> (result: bool)
    ensures result == deadline_exceeded_spec(deadline_ms, now_ms)
{
    now_ms >= deadline_ms
}

pub fn remaining_time_ms_exec(deadline_ms: u64, now_ms: u64) -> (result: u64)
    ensures
        result == remaining_time_spec(deadline_ms, now_ms),
        result == 0 <==> now_ms >= deadline_ms,
        now_ms < deadline_ms ==> result + now_ms == deadline_ms,
{
    if now_ms >= deadline_ms {
        0
    } else {
        deadline_ms - now_ms
    }
}

pub fn compute_effective_timeout_secs_exec(
    user_timeout_secs: Option<u64>,
    default_timeout_secs: u64,
    max_timeout_secs: u64,
) -> (result: u64)
    ensures
        result == effective_timeout_spec(user_timeout_secs, default_timeout_secs, max_timeout_secs),
        option_u64_is_some(user_timeout_secs) ==> result <= max_timeout_secs,
        option_u64_is_none(user_timeout_secs) ==> result == default_timeout_secs,
{
    match user_timeout_secs {
        Some(timeout) => {
            if timeout > max_timeout_secs {
                max_timeout_secs
            } else {
                timeout
            }
        }
        None => default_timeout_secs,
    }
}

pub proof fn deadline_remaining_partition(deadline_ms: u64, now_ms: u64)
    ensures
        deadline_exceeded_spec(deadline_ms, now_ms) ==> remaining_time_spec(deadline_ms, now_ms) == 0,
        !deadline_exceeded_spec(deadline_ms, now_ms) ==> remaining_time_spec(deadline_ms, now_ms) + now_ms == deadline_ms,
{
}

pub proof fn effective_timeout_user_requests_are_capped(
    requested: u64,
    default_timeout_secs: u64,
    max_timeout_secs: u64,
)
    ensures effective_timeout_spec(Some(requested), default_timeout_secs, max_timeout_secs) <= max_timeout_secs,
{
}

} // verus!
