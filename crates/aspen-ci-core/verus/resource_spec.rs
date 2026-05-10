use vstd::prelude::*;

verus! {

pub open spec fn capped_u64_spec(requested: u64, max: u64) -> u64 {
    if requested > max { max } else { requested }
}

pub open spec fn capped_u32_spec(requested: u32, max: u32) -> u32 {
    if requested > max { max } else { requested }
}

pub open spec fn effective_memory_limit_spec(
    requested_bytes: Option<u64>,
    max_bytes: u64,
    default_bytes: u64,
) -> u64 {
    match requested_bytes {
        Some(requested) => capped_u64_spec(requested, max_bytes),
        None => default_bytes,
    }
}

pub open spec fn effective_cpu_weight_spec(
    requested_weight: Option<u32>,
    max_weight: u32,
    default_weight: u32,
) -> u32 {
    match requested_weight {
        Some(requested) => capped_u32_spec(requested, max_weight),
        None => default_weight,
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

pub open spec fn option_u32_is_some(value: Option<u32>) -> bool {
    match value {
        Some(_) => true,
        None => false,
    }
}

pub open spec fn option_u32_is_none(value: Option<u32>) -> bool {
    match value {
        Some(_) => false,
        None => true,
    }
}

pub open spec fn effective_pid_limit_spec(
    requested_pids: Option<u32>,
    max_pids: u32,
    default_pids: u32,
) -> u32 {
    match requested_pids {
        Some(requested) => capped_u32_spec(requested, max_pids),
        None => default_pids,
    }
}

pub open spec fn memory_high_watermark_spec(max_bytes: u64, high_percentage: u32) -> u64 {
    if high_percentage >= 100 {
        max_bytes
    } else {
        ((max_bytes / 100) as int * high_percentage as int) as u64
    }
}

pub open spec fn resource_limits_valid_spec(
    memory_bytes: u64,
    pids: u32,
    min_memory_bytes: u64,
    max_memory_bytes: u64,
    max_pids: u32,
) -> bool {
    memory_bytes >= min_memory_bytes && memory_bytes <= max_memory_bytes && pids <= max_pids
}

pub fn compute_effective_memory_limit_exec(
    requested_bytes: Option<u64>,
    max_bytes: u64,
    default_bytes: u64,
) -> (result: u64)
    ensures
        result == effective_memory_limit_spec(requested_bytes, max_bytes, default_bytes),
        option_u64_is_some(requested_bytes) ==> result <= max_bytes,
        option_u64_is_none(requested_bytes) ==> result == default_bytes,
{
    match requested_bytes {
        Some(requested) => {
            if requested > max_bytes {
                max_bytes
            } else {
                requested
            }
        }
        None => default_bytes,
    }
}

pub fn compute_effective_cpu_weight_exec(
    requested_weight: Option<u32>,
    max_weight: u32,
    default_weight: u32,
) -> (result: u32)
    ensures
        result == effective_cpu_weight_spec(requested_weight, max_weight, default_weight),
        option_u32_is_some(requested_weight) ==> result <= max_weight,
        option_u32_is_none(requested_weight) ==> result == default_weight,
{
    match requested_weight {
        Some(requested) => {
            if requested > max_weight {
                max_weight
            } else {
                requested
            }
        }
        None => default_weight,
    }
}

pub fn compute_effective_pid_limit_exec(
    requested_pids: Option<u32>,
    max_pids: u32,
    default_pids: u32,
) -> (result: u32)
    ensures
        result == effective_pid_limit_spec(requested_pids, max_pids, default_pids),
        option_u32_is_some(requested_pids) ==> result <= max_pids,
        option_u32_is_none(requested_pids) ==> result == default_pids,
{
    match requested_pids {
        Some(requested) => {
            if requested > max_pids {
                max_pids
            } else {
                requested
            }
        }
        None => default_pids,
    }
}

pub fn compute_memory_high_watermark_exec(max_bytes: u64, high_percentage: u32) -> (result: u64)
    ensures
        result == memory_high_watermark_spec(max_bytes, high_percentage),
        result <= max_bytes,
        high_percentage >= 100 ==> result == max_bytes,
        high_percentage == 0 ==> result == 0,
{
    if high_percentage >= 100 {
        max_bytes
    } else {
        let base = max_bytes / 100;
        let percentage = high_percentage as u64;
        assert(percentage < 100);
        assert(base <= max_bytes);
        assert(base * percentage <= max_bytes) by(nonlinear_arith)
            requires
                base == max_bytes / 100,
                percentage < 100,
                base <= max_bytes,
        ;
        assert(percentage == 0 ==> base * percentage == 0) by(nonlinear_arith);
        base * percentage
    }
}

pub fn are_resource_limits_valid_exec(
    memory_bytes: u64,
    pids: u32,
    min_memory_bytes: u64,
    max_memory_bytes: u64,
    max_pids: u32,
) -> (result: bool)
    ensures result == resource_limits_valid_spec(memory_bytes, pids, min_memory_bytes, max_memory_bytes, max_pids)
{
    let has_sufficient_memory = memory_bytes >= min_memory_bytes;
    let is_memory_allowed = memory_bytes <= max_memory_bytes;
    let is_pid_allowed = pids <= max_pids;

    has_sufficient_memory && is_memory_allowed && is_pid_allowed
}

pub proof fn requested_memory_is_capped(requested: u64, max_bytes: u64, default_bytes: u64)
    ensures effective_memory_limit_spec(Some(requested), max_bytes, default_bytes) <= max_bytes,
{
}

pub proof fn requested_cpu_weight_is_capped(requested: u32, max_weight: u32, default_weight: u32)
    ensures effective_cpu_weight_spec(Some(requested), max_weight, default_weight) <= max_weight,
{
}

pub proof fn requested_pid_limit_is_capped(requested: u32, max_pids: u32, default_pids: u32)
    ensures effective_pid_limit_spec(Some(requested), max_pids, default_pids) <= max_pids,
{
}

pub proof fn valid_resource_limits_are_within_bounds(
    memory_bytes: u64,
    pids: u32,
    min_memory_bytes: u64,
    max_memory_bytes: u64,
    max_pids: u32,
)
    ensures
        resource_limits_valid_spec(memory_bytes, pids, min_memory_bytes, max_memory_bytes, max_pids) ==> memory_bytes >= min_memory_bytes,
        resource_limits_valid_spec(memory_bytes, pids, min_memory_bytes, max_memory_bytes, max_pids) ==> memory_bytes <= max_memory_bytes,
        resource_limits_valid_spec(memory_bytes, pids, min_memory_bytes, max_memory_bytes, max_pids) ==> pids <= max_pids,
{
}

} // verus!
