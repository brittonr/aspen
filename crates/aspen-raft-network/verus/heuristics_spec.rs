//! Verus specs for raft-network heuristic arithmetic and state transitions.

use vstd::prelude::*;

verus! {

pub const HEALTH_HEALTHY: u8 = 0;
pub const HEALTH_DEGRADED: u8 = 1;
pub const HEALTH_FAILED: u8 = 2;
pub const DRIFT_NORMAL: u8 = 0;
pub const DRIFT_WARNING: u8 = 1;
pub const DRIFT_ALERT: u8 = 2;
pub const MAX_BACKOFF_MS: u64 = 60000;
pub const I64_SAFE_QUARTER: u64 = 2305843009213693951;

pub open spec fn abs_i64_spec(value: i64) -> u64 {
    if value < 0 { (-value) as u64 } else { value as u64 }
}

pub open spec fn classify_drift_severity_spec(abs_offset_ms: u64, warning_threshold_ms: u64, alert_threshold_ms: u64) -> u8 {
    if abs_offset_ms >= alert_threshold_ms {
        DRIFT_ALERT
    } else if abs_offset_ms >= warning_threshold_ms {
        DRIFT_WARNING
    } else {
        DRIFT_NORMAL
    }
}

pub open spec fn ntp_offset_spec(client_send_ms: u64, server_recv_ms: u64, server_send_ms: u64, client_recv_ms: u64) -> i64
    recommends
        client_send_ms <= I64_SAFE_QUARTER,
        server_recv_ms <= I64_SAFE_QUARTER,
        server_send_ms <= I64_SAFE_QUARTER,
        client_recv_ms <= I64_SAFE_QUARTER,
{
    ((((server_recv_ms as i64) - (client_send_ms as i64)) + ((server_send_ms as i64) - (client_recv_ms as i64))) / 2) as i64
}

pub open spec fn ntp_rtt_spec(client_send_ms: u64, server_recv_ms: u64, server_send_ms: u64, client_recv_ms: u64) -> i64
    recommends
        client_send_ms <= I64_SAFE_QUARTER,
        server_recv_ms <= I64_SAFE_QUARTER,
        server_send_ms <= I64_SAFE_QUARTER,
        client_recv_ms <= I64_SAFE_QUARTER,
{
    (((client_recv_ms as i64) - (client_send_ms as i64)) - ((server_send_ms as i64) - (server_recv_ms as i64))) as i64
}

pub open spec fn transition_connection_health_spec(current_kind: u8, current_failures: u32, operation_succeeded: bool, max_retries: u32) -> (u8, u32) {
    if operation_succeeded {
        if current_kind == HEALTH_FAILED {
            (HEALTH_FAILED, current_failures)
        } else {
            (HEALTH_HEALTHY, 0)
        }
    } else {
        if current_kind == HEALTH_HEALTHY {
            (HEALTH_DEGRADED, 1)
        } else if current_kind == HEALTH_DEGRADED {
            if current_failures >= max_retries {
                (HEALTH_FAILED, current_failures)
            } else {
                (HEALTH_DEGRADED, (current_failures + 1) as u32)
            }
        } else {
            (HEALTH_FAILED, current_failures)
        }
    }
}

pub open spec fn saturating_sub_one_spec(attempt: u32) -> u32 {
    if attempt == 0 { 0 } else { (attempt - 1) as u32 }
}

pub open spec fn backoff_shift_spec(attempt: u32) -> u32 {
    let raw = saturating_sub_one_spec(attempt);
    if raw > 63 { 63 } else { raw }
}

pub open spec fn cap_backoff_spec(raw_ms: u64) -> u64 {
    if raw_ms > MAX_BACKOFF_MS { MAX_BACKOFF_MS } else { raw_ms }
}

pub open spec fn should_evict_oldest_unreachable_spec(current_count: nat, max_nodes: nat, new_node_already_tracked: bool) -> bool {
    !new_node_already_tracked && current_count >= max_nodes
}

pub fn abs_i64_exec(value: i64) -> (out: u64)
    requires value > i64::MIN
    ensures out == abs_i64_spec(value)
{
    if value < 0 {
        (-value) as u64
    } else {
        value as u64
    }
}

pub fn classify_drift_severity_exec(abs_offset_ms: u64, warning_threshold_ms: u64, alert_threshold_ms: u64) -> (severity: u8)
    ensures severity == classify_drift_severity_spec(abs_offset_ms, warning_threshold_ms, alert_threshold_ms)
{
    if abs_offset_ms >= alert_threshold_ms {
        DRIFT_ALERT
    } else if abs_offset_ms >= warning_threshold_ms {
        DRIFT_WARNING
    } else {
        DRIFT_NORMAL
    }
}

pub fn transition_connection_health_exec(current_kind: u8, current_failures: u32, operation_succeeded: bool, max_retries: u32) -> (next: (u8, u32))
    requires current_kind != HEALTH_DEGRADED || current_failures < u32::MAX
    ensures next == transition_connection_health_spec(current_kind, current_failures, operation_succeeded, max_retries)
{
    if operation_succeeded {
        if current_kind == HEALTH_FAILED {
            (HEALTH_FAILED, current_failures)
        } else {
            (HEALTH_HEALTHY, 0)
        }
    } else if current_kind == HEALTH_HEALTHY {
        (HEALTH_DEGRADED, 1)
    } else if current_kind == HEALTH_DEGRADED {
        if current_failures >= max_retries {
            (HEALTH_FAILED, current_failures)
        } else {
            (HEALTH_DEGRADED, current_failures + 1)
        }
    } else {
        (HEALTH_FAILED, current_failures)
    }
}

pub fn backoff_shift_exec(attempt: u32) -> (shift: u32)
    ensures shift == backoff_shift_spec(attempt), shift <= 63
{
    let raw = if attempt == 0 { 0 } else { attempt - 1 };
    if raw > 63 { 63 } else { raw }
}

pub fn cap_backoff_exec(raw_ms: u64) -> (out: u64)
    ensures out == cap_backoff_spec(raw_ms), out <= MAX_BACKOFF_MS
{
    if raw_ms > MAX_BACKOFF_MS { MAX_BACKOFF_MS } else { raw_ms }
}

pub fn should_evict_oldest_unreachable_exec(current_count: usize, max_nodes: usize, new_node_already_tracked: bool) -> (evict: bool)
    ensures evict == should_evict_oldest_unreachable_spec(current_count as nat, max_nodes as nat, new_node_already_tracked)
{
    !new_node_already_tracked && current_count >= max_nodes
}

pub proof fn drift_alert_precedes_warning(abs_offset_ms: u64, warning_threshold_ms: u64, alert_threshold_ms: u64)
    requires abs_offset_ms >= alert_threshold_ms
    ensures classify_drift_severity_spec(abs_offset_ms, warning_threshold_ms, alert_threshold_ms) == DRIFT_ALERT
{
}

pub proof fn ntp_zero_delay_has_zero_offset_and_rtt()
    ensures
        ntp_offset_spec(1000u64, 1000u64, 1000u64, 1000u64) == 0,
        ntp_rtt_spec(1000u64, 1000u64, 1000u64, 1000u64) == 0,
{
}

pub proof fn ntp_symmetric_example_matches_runtime_test()
    ensures
        ntp_offset_spec(1000u64, 1100u64, 1150u64, 1200u64) == 25,
        ntp_rtt_spec(1000u64, 1100u64, 1150u64, 1200u64) == 150,
{
}

pub proof fn drift_warning_between_thresholds(abs_offset_ms: u64, warning_threshold_ms: u64, alert_threshold_ms: u64)
    requires abs_offset_ms < alert_threshold_ms, abs_offset_ms >= warning_threshold_ms
    ensures classify_drift_severity_spec(abs_offset_ms, warning_threshold_ms, alert_threshold_ms) == DRIFT_WARNING
{
}

pub proof fn drift_normal_below_warning(abs_offset_ms: u64, warning_threshold_ms: u64, alert_threshold_ms: u64)
    requires abs_offset_ms < alert_threshold_ms, abs_offset_ms < warning_threshold_ms
    ensures classify_drift_severity_spec(abs_offset_ms, warning_threshold_ms, alert_threshold_ms) == DRIFT_NORMAL
{
}

pub proof fn failed_health_is_terminal(current_failures: u32, operation_succeeded: bool, max_retries: u32)
    ensures transition_connection_health_spec(HEALTH_FAILED, current_failures, operation_succeeded, max_retries) == (HEALTH_FAILED, current_failures)
{
}

pub proof fn healthy_failure_degrades(max_retries: u32)
    ensures transition_connection_health_spec(HEALTH_HEALTHY, 0, false, max_retries) == (HEALTH_DEGRADED, 1u32)
{
}

pub proof fn degraded_success_recovers(current_failures: u32, max_retries: u32)
    ensures transition_connection_health_spec(HEALTH_DEGRADED, current_failures, true, max_retries) == (HEALTH_HEALTHY, 0u32)
{
}

pub proof fn degraded_failure_at_or_over_max_fails(current_failures: u32, max_retries: u32)
    requires current_failures >= max_retries
    ensures transition_connection_health_spec(HEALTH_DEGRADED, current_failures, false, max_retries) == (HEALTH_FAILED, current_failures)
{
}

pub proof fn degraded_failure_below_max_increments(current_failures: u32, max_retries: u32)
    requires current_failures < max_retries, current_failures < u32::MAX
    ensures transition_connection_health_spec(HEALTH_DEGRADED, current_failures, false, max_retries) == (HEALTH_DEGRADED, (current_failures + 1) as u32)
{
}

pub proof fn backoff_shift_is_capped(attempt: u32)
    ensures backoff_shift_spec(attempt) <= 63
{
}

pub proof fn backoff_cap_never_exceeds_max(raw_ms: u64)
    ensures cap_backoff_spec(raw_ms) <= MAX_BACKOFF_MS
{
}

pub proof fn eviction_truth_table(current_count: nat, max_nodes: nat)
    ensures
        should_evict_oldest_unreachable_spec(current_count, max_nodes, true) == false,
        should_evict_oldest_unreachable_spec(current_count, max_nodes, false) == (current_count >= max_nodes),
{
}

} // verus!
