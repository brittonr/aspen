use vstd::prelude::*;

verus! {

pub open spec fn normalize_timeout_u64_spec(timeout_ms: u64) -> Option<u64> {
    if timeout_ms == 0 { None } else { Some(timeout_ms) }
}

pub open spec fn normalize_timeout_u32_spec(timeout_ms: u32) -> Option<u32> {
    if timeout_ms == 0 { None } else { Some(timeout_ms) }
}

pub open spec fn is_indefinite_timeout_spec(timeout_ms: u64) -> bool {
    timeout_ms == 0
}

pub open spec fn option_u64_is_none(value: Option<u64>) -> bool {
    matches!(value, None)
}

pub open spec fn option_u64_is_some_value(value: Option<u64>, expected: u64) -> bool {
    matches!(value, Some(actual) if actual == expected)
}

pub open spec fn option_u32_is_none(value: Option<u32>) -> bool {
    matches!(value, None)
}

pub open spec fn option_u32_is_some_value(value: Option<u32>, expected: u32) -> bool {
    matches!(value, Some(actual) if actual == expected)
}

pub fn normalize_timeout_ms(timeout_ms: u64) -> (result: Option<u64>)
    ensures
        result == normalize_timeout_u64_spec(timeout_ms),
        timeout_ms == 0 ==> option_u64_is_none(result),
        timeout_ms != 0 ==> option_u64_is_some_value(result, timeout_ms),
{
    if timeout_ms == 0 { None } else { Some(timeout_ms) }
}

pub fn normalize_timeout_ms_u32(timeout_ms: u32) -> (result: Option<u32>)
    ensures
        result == normalize_timeout_u32_spec(timeout_ms),
        timeout_ms == 0 ==> option_u32_is_none(result),
        timeout_ms != 0 ==> option_u32_is_some_value(result, timeout_ms),
{
    if timeout_ms == 0 { None } else { Some(timeout_ms) }
}

pub fn is_indefinite_timeout(timeout_ms: u64) -> (result: bool)
    ensures
        result == is_indefinite_timeout_spec(timeout_ms),
        result <==> timeout_ms == 0,
{
    timeout_ms == 0
}

pub proof fn zero_timeout_normalizes_to_none()
    ensures normalize_timeout_u64_spec(0) == None::<u64>,
{
}

pub proof fn nonzero_timeout_normalizes_to_some(timeout_ms: u64)
    requires timeout_ms != 0
    ensures normalize_timeout_u64_spec(timeout_ms) == Some(timeout_ms),
{
}

pub proof fn indefinite_timeout_equivalent_to_zero(timeout_ms: u64)
    ensures is_indefinite_timeout_spec(timeout_ms) <==> timeout_ms == 0,
{
}

} // verus!
