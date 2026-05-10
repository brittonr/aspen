use vstd::prelude::*;

verus! {

pub const PRESSURE_NORMAL: u8 = 0;
pub const PRESSURE_WARNING: u8 = 1;
pub const PRESSURE_CRITICAL: u8 = 2;

pub open spec fn should_invalidate_snapshot_spec(
    restore_failures_consecutive: u32,
    max_restore_failures: u32,
) -> bool {
    restore_failures_consecutive >= max_restore_failures
}

pub open spec fn should_allow_restore_spec(pressure_level: u8, active_fork_count: u32) -> bool {
    pressure_level < PRESSURE_CRITICAL
}

pub open spec fn max_one_u32(value: u32) -> u32 {
    if value < 1 { 1 } else { value }
}

pub open spec fn min_u32(value: u32, max: u32) -> u32 {
    if value > max { max } else { value }
}

pub open spec fn adaptive_fork_count_before_cap_spec(requested_count: u32, pressure_level: u8) -> u32 {
    if pressure_level == PRESSURE_CRITICAL {
        1
    } else if pressure_level == PRESSURE_WARNING {
        max_one_u32(requested_count / 2)
    } else {
        requested_count
    }
}

pub open spec fn adaptive_fork_count_spec(
    requested_count: u32,
    max_count: u32,
    pressure_level: u8,
) -> u32 {
    min_u32(adaptive_fork_count_before_cap_spec(requested_count, pressure_level), max_count)
}

pub fn should_invalidate_snapshot_exec(
    restore_failures_consecutive: u32,
    max_restore_failures: u32,
) -> (result: bool)
    ensures result == should_invalidate_snapshot_spec(restore_failures_consecutive, max_restore_failures)
{
    restore_failures_consecutive >= max_restore_failures
}

pub fn should_allow_restore_exec(pressure_level: u8, active_fork_count: u32) -> (result: bool)
    ensures
        result == should_allow_restore_spec(pressure_level, active_fork_count),
        result == (pressure_level < PRESSURE_CRITICAL),
{
    pressure_level < PRESSURE_CRITICAL
}

pub fn max_one_u32_exec(value: u32) -> (result: u32)
    ensures
        result == max_one_u32(value),
        result >= 1,
        result == value || result == 1,
{
    if value < 1 { 1 } else { value }
}

pub fn min_u32_exec(value: u32, max: u32) -> (result: u32)
    ensures
        result == min_u32(value, max),
        result <= max,
        result == value || result == max,
{
    if value > max { max } else { value }
}

pub fn adaptive_fork_count_before_cap_exec(requested_count: u32, pressure_level: u8) -> (result: u32)
    ensures
        result == adaptive_fork_count_before_cap_spec(requested_count, pressure_level),
        pressure_level == PRESSURE_CRITICAL ==> result == 1,
        pressure_level == PRESSURE_WARNING ==> result >= 1,
        pressure_level != PRESSURE_CRITICAL && pressure_level != PRESSURE_WARNING ==> result == requested_count,
{
    if pressure_level == PRESSURE_CRITICAL {
        1
    } else if pressure_level == PRESSURE_WARNING {
        max_one_u32_exec(requested_count / 2)
    } else {
        requested_count
    }
}

pub fn compute_adaptive_fork_count_exec(
    requested_count: u32,
    max_count: u32,
    pressure_level: u8,
) -> (result: u32)
    ensures
        result == adaptive_fork_count_spec(requested_count, max_count, pressure_level),
        result <= max_count,
        max_count == 0 ==> result == 0,
        pressure_level == PRESSURE_CRITICAL && max_count >= 1 ==> result == 1,
        pressure_level == PRESSURE_WARNING && max_count >= 1 ==> result >= 1,
{
    let count = adaptive_fork_count_before_cap_exec(requested_count, pressure_level);
    min_u32_exec(count, max_count)
}

pub proof fn invalidation_threshold_is_inclusive(
    restore_failures_consecutive: u32,
    max_restore_failures: u32,
)
    ensures
        should_invalidate_snapshot_spec(restore_failures_consecutive, max_restore_failures)
            <==> restore_failures_consecutive >= max_restore_failures,
{
}

pub proof fn critical_pressure_blocks_restore(active_fork_count: u32)
    ensures !should_allow_restore_spec(PRESSURE_CRITICAL, active_fork_count)
{
}

pub proof fn normal_and_warning_pressure_allow_restore(active_fork_count: u32)
    ensures
        should_allow_restore_spec(PRESSURE_NORMAL, active_fork_count),
        should_allow_restore_spec(PRESSURE_WARNING, active_fork_count),
{
}

pub proof fn adaptive_fork_count_is_capped(
    requested_count: u32,
    max_count: u32,
    pressure_level: u8,
)
    ensures adaptive_fork_count_spec(requested_count, max_count, pressure_level) <= max_count
{
}

pub proof fn critical_fork_count_is_one_when_cap_allows(requested_count: u32, max_count: u32)
    ensures max_count >= 1 ==> adaptive_fork_count_spec(requested_count, max_count, PRESSURE_CRITICAL) == 1
{
}

} // verus!
