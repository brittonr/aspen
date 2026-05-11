//! Verus specs for NIP-42 auth admission and write-policy gating.
//!
//! Production auth verifies Nostr signatures, scans event tags, and mutates
//! per-connection state in the Rust shell. This module verifies the pure
//! scalar contract around that shell: auth kind constants, challenge length,
//! timestamp-window admission, optional relay-tag enforcement, first-failure
//! ordering, and the shared WebSocket/Iroh write-policy truth table.

use vstd::prelude::*;

verus! {

pub const AUTH_EVENT_KIND: u16 = 22242;
pub const AUTH_CHALLENGE_BYTES: u32 = 32;
pub const AUTH_CHALLENGE_HEX_LEN: u32 = 64;
pub const AUTH_TIMESTAMP_WINDOW_SECS: u64 = 60;

pub enum WritePolicySpec {
    Open,
    AuthRequired,
    ReadOnly,
}

pub enum AuthFailureSpec {
    WrongKind,
    InvalidChallenge,
    InvalidRelayUrl,
    TimestampOutOfRange,
    InvalidSignature,
}

pub enum AuthAdmissionSpec {
    Accept,
    Reject(AuthFailureSpec),
}

pub enum WriteAdmissionSpec {
    Allow,
    RejectReadOnly,
    RejectAuthRequired,
}

pub open spec fn challenge_hex_len(bytes: u32) -> u32 {
    (bytes * 2) as u32
}

pub open spec fn auth_constants_valid() -> bool {
    AUTH_EVENT_KIND == 22242
        && AUTH_CHALLENGE_BYTES == 32
        && AUTH_CHALLENGE_HEX_LEN == challenge_hex_len(AUTH_CHALLENGE_BYTES)
        && AUTH_TIMESTAMP_WINDOW_SECS > 0
}

pub open spec fn abs_diff_u64_spec(a: u64, b: u64) -> u64 {
    if a >= b { (a - b) as u64 } else { (b - a) as u64 }
}

pub open spec fn timestamp_in_auth_window(created_secs: u64, now_secs: u64, window_secs: u64) -> bool {
    abs_diff_u64_spec(created_secs, now_secs) <= window_secs
}

pub open spec fn relay_tag_admitted(relay_url_configured: bool, relay_tag_matches: bool) -> bool {
    !relay_url_configured || relay_tag_matches
}

pub open spec fn challenge_tag_admitted(challenge_tag_matches: bool) -> bool {
    challenge_tag_matches
}

pub open spec fn auth_admission(
    kind_matches: bool,
    challenge_tag_matches: bool,
    relay_url_configured: bool,
    relay_tag_matches: bool,
    created_secs: u64,
    now_secs: u64,
    signature_valid: bool,
) -> AuthAdmissionSpec {
    if !kind_matches {
        AuthAdmissionSpec::Reject(AuthFailureSpec::WrongKind)
    } else if !challenge_tag_matches {
        AuthAdmissionSpec::Reject(AuthFailureSpec::InvalidChallenge)
    } else if !relay_tag_admitted(relay_url_configured, relay_tag_matches) {
        AuthAdmissionSpec::Reject(AuthFailureSpec::InvalidRelayUrl)
    } else if !timestamp_in_auth_window(created_secs, now_secs, AUTH_TIMESTAMP_WINDOW_SECS) {
        AuthAdmissionSpec::Reject(AuthFailureSpec::TimestampOutOfRange)
    } else if !signature_valid {
        AuthAdmissionSpec::Reject(AuthFailureSpec::InvalidSignature)
    } else {
        AuthAdmissionSpec::Accept
    }
}

pub open spec fn auth_succeeds(
    kind_matches: bool,
    challenge_tag_matches: bool,
    relay_url_configured: bool,
    relay_tag_matches: bool,
    created_secs: u64,
    now_secs: u64,
    signature_valid: bool,
) -> bool {
    auth_admission(
        kind_matches,
        challenge_tag_matches,
        relay_url_configured,
        relay_tag_matches,
        created_secs,
        now_secs,
        signature_valid,
    ) == AuthAdmissionSpec::Accept
}

pub open spec fn write_admission(policy: WritePolicySpec, authenticated: bool) -> WriteAdmissionSpec {
    match policy {
        WritePolicySpec::ReadOnly => WriteAdmissionSpec::RejectReadOnly,
        WritePolicySpec::AuthRequired => if authenticated {
            WriteAdmissionSpec::Allow
        } else {
            WriteAdmissionSpec::RejectAuthRequired
        },
        WritePolicySpec::Open => WriteAdmissionSpec::Allow,
    }
}

pub open spec fn write_allowed(policy: WritePolicySpec, authenticated: bool) -> bool {
    write_admission(policy, authenticated) == WriteAdmissionSpec::Allow
}

pub fn abs_diff_u64_exec(a: u64, b: u64) -> (delta: u64)
    ensures delta == abs_diff_u64_spec(a, b)
{
    if a >= b { a - b } else { b - a }
}

pub fn timestamp_in_auth_window_exec(created_secs: u64, now_secs: u64, window_secs: u64) -> (ok: bool)
    ensures ok == timestamp_in_auth_window(created_secs, now_secs, window_secs)
{
    let delta = abs_diff_u64_exec(created_secs, now_secs);
    delta <= window_secs
}

pub fn write_admission_exec(policy: WritePolicySpec, authenticated: bool) -> (admission: WriteAdmissionSpec)
    ensures admission == write_admission(policy, authenticated)
{
    match policy {
        WritePolicySpec::ReadOnly => WriteAdmissionSpec::RejectReadOnly,
        WritePolicySpec::AuthRequired => if authenticated {
            WriteAdmissionSpec::Allow
        } else {
            WriteAdmissionSpec::RejectAuthRequired
        },
        WritePolicySpec::Open => WriteAdmissionSpec::Allow,
    }
}

pub proof fn auth_constants_are_valid()
    ensures auth_constants_valid()
{
}

pub proof fn challenge_hex_length_matches_random_bytes()
    ensures challenge_hex_len(AUTH_CHALLENGE_BYTES) == AUTH_CHALLENGE_HEX_LEN
{
}

pub proof fn timestamp_equal_is_admitted(now_secs: u64, window_secs: u64)
    ensures timestamp_in_auth_window(now_secs, now_secs, window_secs)
{
}

pub proof fn timestamp_at_past_boundary_is_admitted(now_secs: u64, window_secs: u64)
    requires now_secs >= window_secs
    ensures timestamp_in_auth_window((now_secs - window_secs) as u64, now_secs, window_secs)
{
}

pub proof fn timestamp_at_future_boundary_is_admitted(now_secs: u64, window_secs: u64)
    requires now_secs <= u64::MAX - window_secs
    ensures timestamp_in_auth_window((now_secs + window_secs) as u64, now_secs, window_secs)
{
}

pub proof fn timestamp_before_window_is_rejected(now_secs: u64, window_secs: u64, extra_secs: u64)
    requires
        now_secs >= window_secs + extra_secs,
        extra_secs > 0,
    ensures !timestamp_in_auth_window((now_secs - window_secs - extra_secs) as u64, now_secs, window_secs)
{
}

pub proof fn timestamp_after_window_is_rejected(now_secs: u64, window_secs: u64, extra_secs: u64)
    requires
        now_secs <= u64::MAX - window_secs - extra_secs,
        extra_secs > 0,
    ensures !timestamp_in_auth_window((now_secs + window_secs + extra_secs) as u64, now_secs, window_secs)
{
}

pub proof fn relay_tag_skipped_when_unconfigured(relay_tag_matches: bool)
    ensures relay_tag_admitted(false, relay_tag_matches)
{
}

pub proof fn configured_relay_requires_match()
    ensures
        relay_tag_admitted(true, true),
        !relay_tag_admitted(true, false),
{
}

pub proof fn valid_auth_inputs_accept(created_secs: u64, now_secs: u64)
    requires timestamp_in_auth_window(created_secs, now_secs, AUTH_TIMESTAMP_WINDOW_SECS)
    ensures auth_admission(true, true, true, true, created_secs, now_secs, true) == AuthAdmissionSpec::Accept
{
}

pub proof fn wrong_kind_rejected_first(created_secs: u64, now_secs: u64)
    ensures auth_admission(false, false, false, false, created_secs, now_secs, false)
        == AuthAdmissionSpec::Reject(AuthFailureSpec::WrongKind)
{
}

pub proof fn wrong_challenge_rejected_after_kind(created_secs: u64, now_secs: u64)
    ensures auth_admission(true, false, false, false, created_secs, now_secs, false)
        == AuthAdmissionSpec::Reject(AuthFailureSpec::InvalidChallenge)
{
}

pub proof fn wrong_relay_rejected_after_challenge(created_secs: u64, now_secs: u64)
    ensures auth_admission(true, true, true, false, created_secs, now_secs, false)
        == AuthAdmissionSpec::Reject(AuthFailureSpec::InvalidRelayUrl)
{
}

pub proof fn stale_timestamp_rejected_after_tags(created_secs: u64, now_secs: u64)
    requires !timestamp_in_auth_window(created_secs, now_secs, AUTH_TIMESTAMP_WINDOW_SECS)
    ensures auth_admission(true, true, false, false, created_secs, now_secs, true)
        == AuthAdmissionSpec::Reject(AuthFailureSpec::TimestampOutOfRange)
{
}

pub proof fn invalid_signature_rejected_last(created_secs: u64, now_secs: u64)
    requires timestamp_in_auth_window(created_secs, now_secs, AUTH_TIMESTAMP_WINDOW_SECS)
    ensures auth_admission(true, true, false, false, created_secs, now_secs, false)
        == AuthAdmissionSpec::Reject(AuthFailureSpec::InvalidSignature)
{
}

pub proof fn open_policy_allows_regardless_of_auth(authenticated: bool)
    ensures write_allowed(WritePolicySpec::Open, authenticated)
{
}

pub proof fn read_only_policy_rejects_regardless_of_auth(authenticated: bool)
    ensures
        !write_allowed(WritePolicySpec::ReadOnly, authenticated),
        write_admission(WritePolicySpec::ReadOnly, authenticated) == WriteAdmissionSpec::RejectReadOnly,
{
}

pub proof fn auth_required_rejects_unauthenticated()
    ensures
        !write_allowed(WritePolicySpec::AuthRequired, false),
        write_admission(WritePolicySpec::AuthRequired, false) == WriteAdmissionSpec::RejectAuthRequired,
{
}

pub proof fn auth_required_allows_authenticated()
    ensures write_allowed(WritePolicySpec::AuthRequired, true)
{
}

} // verus!
