//! Selected profile refs and effective transport settings.

pub(super) fn selected_topology_profile_ref<'a>(input: &'a super::offers::ControlLiveSendInput<'a>) -> Option<&'a str> {
    input.topology_profile.map(|profile| profile.profile_ref)
}

pub(super) fn selected_transport_profile_ref<'a>(
    input: &'a super::offers::ControlLiveSendInput<'a>,
) -> Option<&'a str> {
    input.transport_profile.map(|profile| profile.profile_ref)
}

pub(super) fn effective_live_send_max_attempts(input: &super::offers::ControlLiveSendInput<'_>) -> u64 {
    input.transport_profile.map_or(input.max_attempts, |profile| profile.max_attempts)
}

pub(super) fn effective_live_send_join_timeout_ms(input: &super::offers::ControlLiveSendInput<'_>) -> u64 {
    input.transport_profile.map_or(input.join_timeout_ms, |profile| profile.join_timeout_ms)
}

pub(super) fn selected_apply_topology_profile_ref<'a>(
    input: &'a super::bundling::ControlLiveWorkflowBundleApplyInput<'a>,
) -> Option<&'a str> {
    input.topology_profile.map(|profile| profile.profile_ref)
}

pub(super) fn selected_apply_transport_profile_ref<'a>(
    input: &'a super::bundling::ControlLiveWorkflowBundleApplyInput<'a>,
) -> Option<&'a str> {
    input.transport_profile.map(|profile| profile.profile_ref)
}

pub(super) fn effective_live_apply_max_attempts(
    input: &super::bundling::ControlLiveWorkflowBundleApplyInput<'_>,
) -> u64 {
    input.transport_profile.map_or(input.max_attempts, |profile| profile.max_attempts)
}

pub(super) fn effective_live_apply_join_timeout_ms(
    input: &super::bundling::ControlLiveWorkflowBundleApplyInput<'_>,
) -> u64 {
    input.transport_profile.map_or(input.join_timeout_ms, |profile| profile.join_timeout_ms)
}

pub(super) fn live_profile_ref_records(
    topology_profile_ref: Option<&str>,
    transport_profile_ref: Option<&str>,
) -> preserves::IOValue {
    let caveat = if topology_profile_ref.is_none() && transport_profile_ref.is_none() {
        "explicit-flags-no-profile"
    } else {
        "reviewed-live-profile-input"
    };
    crate::preserves_rail::record("live-profiles", vec![
        crate::preserves_rail::record("topology", vec![super::fields::optional_string(topology_profile_ref)]),
        crate::preserves_rail::record("transport", vec![super::fields::optional_string(transport_profile_ref)]),
        crate::preserves_rail::record("caveat", vec![crate::preserves_rail::string(caveat)]),
    ])
}

pub(super) fn live_effective_transport_record(max_attempts: u64, join_timeout_ms: u64) -> preserves::IOValue {
    crate::preserves_rail::record("effective-transport", vec![
        crate::preserves_rail::record("max-attempts", vec![crate::preserves_rail::string(max_attempts.to_string())]),
        crate::preserves_rail::record("join-timeout-ms", vec![crate::preserves_rail::string(
            join_timeout_ms.to_string(),
        )]),
    ])
}

pub(super) fn live_effective_transport_optional_record(
    max_attempts: Option<u64>,
    join_timeout_ms: Option<u64>,
) -> preserves::IOValue {
    crate::preserves_rail::record("effective-transport", vec![
        crate::preserves_rail::record("max-attempts", vec![super::fields::optional_string(
            max_attempts.as_ref().map(ToString::to_string).as_deref(),
        )]),
        crate::preserves_rail::record("join-timeout-ms", vec![super::fields::optional_string(
            join_timeout_ms.as_ref().map(ToString::to_string).as_deref(),
        )]),
    ])
}
