//! Verus specifications for KV operation admission and lease/TTL arithmetic.

use vstd::prelude::*;

verus! {

pub const MILLIS_PER_SECOND: u64 = 1000;
pub const U64_MAX_VALUE: u64 = 18446744073709551615u64;
pub const U32_MAX_VALUE: u32 = 4294967295u32;

// ========================================================================
// TTL / lease arithmetic
// ========================================================================

pub open spec fn ttl_ms_spec(ttl_seconds: u32) -> u64 {
    ((ttl_seconds as u64) * MILLIS_PER_SECOND) as u64
}

pub open spec fn expires_at_ms_spec(now_ms: u64, ttl_seconds: u32) -> u64 {
    let ttl_ms = ttl_ms_spec(ttl_seconds);
    if now_ms as int + ttl_ms as int > U64_MAX_VALUE as int {
        U64_MAX_VALUE
    } else {
        (now_ms + ttl_ms) as u64
    }
}

pub open spec fn key_expiration_spec(ttl_seconds: Option<u32>, now_ms: u64) -> Option<u64> {
    match ttl_seconds {
        None => None,
        Some(ttl) => if ttl == 0 { None } else { Some(expires_at_ms_spec(now_ms, ttl)) },
    }
}

pub open spec fn lease_expired_spec(expires_at_ms: u64, now_ms: u64) -> bool {
    now_ms > expires_at_ms
}

pub fn calculate_expires_at_ms_exec(now_ms: u64, ttl_seconds: u32) -> (result: u64)
    ensures
        result == expires_at_ms_spec(now_ms, ttl_seconds),
        result >= now_ms,
        result == U64_MAX_VALUE || result == now_ms + ttl_ms_spec(ttl_seconds),
{
    let ttl_ms = (ttl_seconds as u64) * MILLIS_PER_SECOND;
    assert(ttl_ms == ttl_ms_spec(ttl_seconds));
    if now_ms > U64_MAX_VALUE - ttl_ms {
        U64_MAX_VALUE
    } else {
        now_ms + ttl_ms
    }
}

pub fn compute_key_expiration_exec(ttl_seconds: Option<u32>, now_ms: u64) -> (result: Option<u64>)
    ensures result == key_expiration_spec(ttl_seconds, now_ms)
{
    match ttl_seconds {
        None => None,
        Some(ttl) => {
            if ttl == 0 {
                None
            } else {
                Some(calculate_expires_at_ms_exec(now_ms, ttl))
            }
        }
    }
}

pub fn compute_lease_refresh_exec(ttl_seconds: u32, now_ms: u64) -> (result: u64)
    ensures result == expires_at_ms_spec(now_ms, ttl_seconds)
{
    calculate_expires_at_ms_exec(now_ms, ttl_seconds)
}

pub fn is_lease_expired_exec(expires_at_ms: u64, now_ms: u64) -> (result: bool)
    ensures result == lease_expired_spec(expires_at_ms, now_ms)
{
    now_ms > expires_at_ms
}

// ========================================================================
// KV version arithmetic over the non-wrapping Raft-index domain
// ========================================================================

pub struct KvVersionsSpec {
    pub create_revision: i64,
    pub mod_revision: i64,
    pub version: i64,
}

pub open spec fn i64_saturating_add_one_spec(value: i64) -> i64 {
    if value == i64::MAX { i64::MAX } else { (value + 1) as i64 }
}

pub open spec fn kv_versions_spec(existing_version: Option<(i64, i64)>, log_index: u64) -> KvVersionsSpec
    recommends log_index <= i64::MAX as u64
{
    match existing_version {
        Some((create_revision, version)) => KvVersionsSpec {
            create_revision,
            mod_revision: log_index as i64,
            version: i64_saturating_add_one_spec(version),
        },
        None => KvVersionsSpec {
            create_revision: log_index as i64,
            mod_revision: log_index as i64,
            version: 1,
        },
    }
}

pub fn i64_saturating_add_one_exec(value: i64) -> (result: i64)
    ensures result == i64_saturating_add_one_spec(value)
{
    if value == i64::MAX {
        i64::MAX
    } else {
        value + 1
    }
}

pub fn compute_kv_versions_exec(
    existing_version: Option<(i64, i64)>,
    log_index: u64,
) -> (result: KvVersionsSpec)
    requires log_index <= i64::MAX as u64
    ensures
        result.create_revision == kv_versions_spec(existing_version, log_index).create_revision,
        result.mod_revision == kv_versions_spec(existing_version, log_index).mod_revision,
        result.version == kv_versions_spec(existing_version, log_index).version,
{
    match existing_version {
        Some((create_revision, version)) => KvVersionsSpec {
            create_revision,
            mod_revision: log_index as i64,
            version: i64_saturating_add_one_exec(version),
        },
        None => KvVersionsSpec {
            create_revision: log_index as i64,
            mod_revision: log_index as i64,
            version: 1,
        },
    }
}

// ========================================================================
// CAS admission decision table
// ========================================================================

pub enum CasValidationResultSpec {
    Ok,
    KeyNotFound,
    ValueMismatch { expected_len: u32, actual_len: u32 },
}

pub open spec fn bounded_u32_len_spec(len: nat) -> u32 {
    if len > U32_MAX_VALUE as nat { U32_MAX_VALUE } else { len as u32 }
}

pub open spec fn cas_precondition_spec(
    actual_present: bool,
    values_match: bool,
    actual_len: nat,
    expected_len: nat,
) -> CasValidationResultSpec {
    if !actual_present {
        CasValidationResultSpec::KeyNotFound
    } else if values_match {
        CasValidationResultSpec::Ok
    } else {
        CasValidationResultSpec::ValueMismatch {
            expected_len: bounded_u32_len_spec(expected_len),
            actual_len: bounded_u32_len_spec(actual_len),
        }
    }
}

pub open spec fn cas_condition_spec(expected_present: bool, current_present: bool, values_match: bool) -> bool {
    (!expected_present && !current_present) || (expected_present && current_present && values_match)
}

pub fn bounded_u32_len_exec(len: u64) -> (result: u32)
    ensures result == bounded_u32_len_spec(len as nat)
{
    if len > U32_MAX_VALUE as u64 {
        U32_MAX_VALUE
    } else {
        len as u32
    }
}

pub fn validate_cas_precondition_exec(
    actual_present: bool,
    values_match: bool,
    actual_len: u64,
    expected_len: u64,
) -> (result: CasValidationResultSpec)
    ensures result == cas_precondition_spec(actual_present, values_match, actual_len as nat, expected_len as nat)
{
    if !actual_present {
        CasValidationResultSpec::KeyNotFound
    } else if values_match {
        CasValidationResultSpec::Ok
    } else {
        CasValidationResultSpec::ValueMismatch {
            expected_len: bounded_u32_len_exec(expected_len),
            actual_len: bounded_u32_len_exec(actual_len),
        }
    }
}

pub fn check_cas_condition_exec(
    expected_present: bool,
    current_present: bool,
    values_match: bool,
) -> (result: bool)
    ensures result == cas_condition_spec(expected_present, current_present, values_match)
{
    match (expected_present, current_present) {
        (false, false) => true,
        (true, true) => values_match,
        _ => false,
    }
}

// ========================================================================
// Proof facts used by reviews/specs
// ========================================================================

pub proof fn zero_ttl_has_no_key_expiration(now_ms: u64)
    ensures key_expiration_spec(Some(0), now_ms) == None::<u64>
{
}

pub proof fn no_ttl_has_no_key_expiration(now_ms: u64)
    ensures key_expiration_spec(None, now_ms) == None::<u64>
{
}

pub proof fn nonzero_ttl_has_expiration(now_ms: u64, ttl_seconds: u32)
    requires ttl_seconds > 0
    ensures key_expiration_spec(Some(ttl_seconds), now_ms) == Some(expires_at_ms_spec(now_ms, ttl_seconds))
{
}

pub proof fn lease_boundary_is_not_expired(expires_at_ms: u64)
    ensures !lease_expired_spec(expires_at_ms, expires_at_ms)
{
}

pub proof fn lease_after_boundary_is_expired(expires_at_ms: u64, now_ms: u64)
    requires now_ms > expires_at_ms
    ensures lease_expired_spec(expires_at_ms, now_ms)
{
}

pub proof fn cas_absent_actual_is_key_not_found(values_match: bool, actual_len: nat, expected_len: nat)
    ensures cas_precondition_spec(false, values_match, actual_len, expected_len) == CasValidationResultSpec::KeyNotFound
{
}

pub proof fn cas_present_matching_values_ok(actual_len: nat, expected_len: nat)
    ensures cas_precondition_spec(true, true, actual_len, expected_len) == CasValidationResultSpec::Ok
{
}

pub proof fn cas_condition_truth_table(expected_present: bool, current_present: bool, values_match: bool)
    ensures
        cas_condition_spec(expected_present, current_present, values_match)
            == ((!expected_present && !current_present) || (expected_present && current_present && values_match)),
{
}

pub proof fn new_key_versions_start_at_one(log_index: u64)
    requires log_index <= i64::MAX as u64
    ensures
        kv_versions_spec(None, log_index).create_revision == log_index as i64,
        kv_versions_spec(None, log_index).mod_revision == log_index as i64,
        kv_versions_spec(None, log_index).version == 1,
{
}

pub proof fn existing_key_preserves_create_revision(create_revision: i64, version: i64, log_index: u64)
    requires log_index <= i64::MAX as u64
    ensures
        kv_versions_spec(Some((create_revision, version)), log_index).create_revision == create_revision,
        kv_versions_spec(Some((create_revision, version)), log_index).mod_revision == log_index as i64,
        kv_versions_spec(Some((create_revision, version)), log_index).version == i64_saturating_add_one_spec(version),
{
}

} // verus!
