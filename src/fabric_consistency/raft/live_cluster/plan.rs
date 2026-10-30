//! Pure live-cluster replica plan builders shared by fresh and recovered startup.
use super::*;

pub(super) fn bind_state_profile(
    state: &mut ReplicaState,
    protocol_ref: &str,
    durable_log_ref: &str,
    snapshot_store_ref: &str,
    timer_profile_ref: &str,
    entropy_profile_ref: &str,
    supervision_ref: &str,
) {
    state.profile.protocol_ref = protocol_ref.to_string();
    state.profile.durable_log_ref = durable_log_ref.to_string();
    state.profile.snapshot_store_ref = snapshot_store_ref.to_string();
    state.profile.timer_profile_ref = timer_profile_ref.to_string();
    state.profile.entropy_profile_ref = entropy_profile_ref.to_string();
    state.profile.supervision_ref = supervision_ref.to_string();
}

pub(super) fn runtime_identity(
    group: &crate::fabric_consistency::Group,
    state: &ReplicaState,
    fabric_binding_refs: Vec<String>,
) -> ReplicaRuntimePortIdentity {
    ReplicaRuntimePortIdentity {
        service_id: group.service_id.clone(),
        service_generation: state.profile.service_generation,
        group_binding_ref: state.profile.group_binding_ref.clone(),
        application_manifest_ref: group.application_manifest_ref.clone(),
        protocol_ref: state.profile.protocol_ref.clone(),
        durable_log_ref: state.profile.durable_log_ref.clone(),
        snapshot_store_ref: state.profile.snapshot_store_ref.clone(),
        timer_profile_ref: state.profile.timer_profile_ref.clone(),
        entropy_profile_ref: state.profile.entropy_profile_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        placement_ref: state.profile.placement_ref.clone(),
        fencing_ref: state.profile.fencing_ref.clone(),
        supervision_ref: state.profile.supervision_ref.clone(),
        resource_profile_ref: state.profile.resource_profile_ref.clone(),
        fabric_binding_refs,
    }
}

pub(super) fn startup(
    group: &crate::fabric_consistency::Group,
    state: ReplicaState,
    port_binding_refs: Vec<String>,
) -> ReplicaStartPlan {
    let timer_ref = state.active_election_timer_ref.clone();
    ReplicaStartPlan {
        state,
        service_id: group.service_id.clone(),
        application_manifest_ref: group.application_manifest_ref.clone(),
        initial_effects: vec![
            ReplicaEffect::PersistHardState {
                term: INITIAL_TERM,
                voted_for: None,
                is_recovery_required: false,
            },
            ReplicaEffect::ArmElectionTimer { timer_ref },
        ],
        port_binding_refs,
        production_admitted: false,
    }
}
