//! Verus specs for NIP-11 relay information scalar fields.
//!
//! Production `relay_info_json` builds JSON and injects runtime identity/version
//! data in the Rust shell. This module verifies the pure policy/control-flow
//! kernel: supported-NIP constants, limitation-object inclusion, auth/read-only
//! flags, and the relationship between advertised write policy and EVENT write
//! admission.

use vstd::prelude::*;

verus! {

pub const NIP_BASIC_PROTOCOL: u16 = 1;
pub const NIP_RELAY_INFO: u16 = 11;
pub const NIP_GIT_PATCHES: u16 = 34;
pub const NIP_AUTH: u16 = 42;
pub const SUPPORTED_NIP_COUNT: u32 = 4;

pub enum WritePolicySpec {
    Open,
    AuthRequired,
    ReadOnly,
}

pub struct Nip11LimitationSpec {
    pub present: bool,
    pub auth_required: bool,
    pub read_only: bool,
}

pub enum EventWriteAdmissionSpec {
    Allow,
    RejectAuthRequired,
    RejectReadOnly,
}

pub open spec fn supported_nip_at(index: u32) -> Option<u16> {
    if index == 0 {
        Some(NIP_BASIC_PROTOCOL)
    } else if index == 1 {
        Some(NIP_RELAY_INFO)
    } else if index == 2 {
        Some(NIP_GIT_PATCHES)
    } else if index == 3 {
        Some(NIP_AUTH)
    } else {
        None
    }
}

pub open spec fn supports_nip(nip: u16) -> bool {
    nip == NIP_BASIC_PROTOCOL || nip == NIP_RELAY_INFO || nip == NIP_GIT_PATCHES || nip == NIP_AUTH
}

pub open spec fn nip11_limitation_for_policy(policy: WritePolicySpec) -> Nip11LimitationSpec {
    match policy {
        WritePolicySpec::Open => Nip11LimitationSpec {
            present: false,
            auth_required: false,
            read_only: false,
        },
        WritePolicySpec::AuthRequired => Nip11LimitationSpec {
            present: true,
            auth_required: true,
            read_only: false,
        },
        WritePolicySpec::ReadOnly => Nip11LimitationSpec {
            present: true,
            auth_required: true,
            read_only: true,
        },
    }
}

pub open spec fn nip11_limitation_present(policy: WritePolicySpec) -> bool {
    nip11_limitation_for_policy(policy).present
}

pub open spec fn nip11_auth_required(policy: WritePolicySpec) -> bool {
    nip11_limitation_for_policy(policy).auth_required
}

pub open spec fn nip11_read_only(policy: WritePolicySpec) -> bool {
    nip11_limitation_for_policy(policy).read_only
}

pub open spec fn event_write_admission(policy: WritePolicySpec, authenticated: bool) -> EventWriteAdmissionSpec {
    match policy {
        WritePolicySpec::Open => EventWriteAdmissionSpec::Allow,
        WritePolicySpec::AuthRequired => if authenticated {
            EventWriteAdmissionSpec::Allow
        } else {
            EventWriteAdmissionSpec::RejectAuthRequired
        },
        WritePolicySpec::ReadOnly => EventWriteAdmissionSpec::RejectReadOnly,
    }
}

pub open spec fn event_write_allowed(policy: WritePolicySpec, authenticated: bool) -> bool {
    event_write_admission(policy, authenticated) == EventWriteAdmissionSpec::Allow
}

pub fn supported_nip_at_exec(index: u32) -> (nip: Option<u16>)
    ensures nip == supported_nip_at(index)
{
    if index == 0 {
        Some(NIP_BASIC_PROTOCOL)
    } else if index == 1 {
        Some(NIP_RELAY_INFO)
    } else if index == 2 {
        Some(NIP_GIT_PATCHES)
    } else if index == 3 {
        Some(NIP_AUTH)
    } else {
        None
    }
}

pub fn supports_nip_exec(nip: u16) -> (supported: bool)
    ensures supported == supports_nip(nip)
{
    nip == NIP_BASIC_PROTOCOL || nip == NIP_RELAY_INFO || nip == NIP_GIT_PATCHES || nip == NIP_AUTH
}

pub fn event_write_admission_exec(policy: WritePolicySpec, authenticated: bool) -> (admission: EventWriteAdmissionSpec)
    ensures admission == event_write_admission(policy, authenticated)
{
    match policy {
        WritePolicySpec::Open => EventWriteAdmissionSpec::Allow,
        WritePolicySpec::AuthRequired => if authenticated {
            EventWriteAdmissionSpec::Allow
        } else {
            EventWriteAdmissionSpec::RejectAuthRequired
        },
        WritePolicySpec::ReadOnly => EventWriteAdmissionSpec::RejectReadOnly,
    }
}

pub proof fn supported_nip_constants_match_relay_info()
    ensures
        supported_nip_at(0) == Some(NIP_BASIC_PROTOCOL),
        supported_nip_at(1) == Some(NIP_RELAY_INFO),
        supported_nip_at(2) == Some(NIP_GIT_PATCHES),
        supported_nip_at(3) == Some(NIP_AUTH),
        supported_nip_at(SUPPORTED_NIP_COUNT) == None::<u16>,
{
}

pub proof fn advertised_nips_are_supported()
    ensures
        supports_nip(NIP_BASIC_PROTOCOL),
        supports_nip(NIP_RELAY_INFO),
        supports_nip(NIP_GIT_PATCHES),
        supports_nip(NIP_AUTH),
{
}

pub proof fn unsupported_neighbor_nips_are_excluded()
    ensures
        !supports_nip(0),
        !supports_nip(12),
        !supports_nip(41),
        !supports_nip(43),
{
}

pub proof fn open_policy_omits_limitation()
    ensures
        !nip11_limitation_present(WritePolicySpec::Open),
        !nip11_auth_required(WritePolicySpec::Open),
        !nip11_read_only(WritePolicySpec::Open),
{
}

pub proof fn auth_required_policy_advertises_auth_only()
    ensures
        nip11_limitation_present(WritePolicySpec::AuthRequired),
        nip11_auth_required(WritePolicySpec::AuthRequired),
        !nip11_read_only(WritePolicySpec::AuthRequired),
{
}

pub proof fn read_only_policy_advertises_auth_and_read_only()
    ensures
        nip11_limitation_present(WritePolicySpec::ReadOnly),
        nip11_auth_required(WritePolicySpec::ReadOnly),
        nip11_read_only(WritePolicySpec::ReadOnly),
{
}

pub proof fn read_only_implies_auth_required(policy: WritePolicySpec)
    requires nip11_read_only(policy)
    ensures nip11_auth_required(policy)
{
}

pub proof fn limitation_present_exactly_for_restricted_policies(policy: WritePolicySpec)
    ensures nip11_limitation_present(policy) == (policy != WritePolicySpec::Open)
{
}

pub proof fn advertised_auth_required_matches_unauthenticated_write_rejection(policy: WritePolicySpec)
    ensures nip11_auth_required(policy) == (event_write_admission(policy, false) != EventWriteAdmissionSpec::Allow)
{
}

pub proof fn advertised_read_only_matches_authenticated_write_rejection(policy: WritePolicySpec)
    ensures nip11_read_only(policy) == (event_write_admission(policy, true) == EventWriteAdmissionSpec::RejectReadOnly)
{
}

pub proof fn open_policy_allows_authenticated_and_unauthenticated_writes()
    ensures
        event_write_allowed(WritePolicySpec::Open, false),
        event_write_allowed(WritePolicySpec::Open, true),
{
}

pub proof fn auth_required_policy_allows_only_authenticated_writes()
    ensures
        !event_write_allowed(WritePolicySpec::AuthRequired, false),
        event_write_allowed(WritePolicySpec::AuthRequired, true),
{
}

pub proof fn read_only_policy_rejects_all_external_writes(authenticated: bool)
    ensures !event_write_allowed(WritePolicySpec::ReadOnly, authenticated)
{
}

} // verus!
