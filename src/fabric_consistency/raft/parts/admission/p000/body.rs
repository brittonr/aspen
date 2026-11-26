pub const LIVE_RAFT_ALGORITHM_PROFILE: &str = "raft";
pub const LIVE_RAFT_IMPLEMENTATION_PROFILE: &str = "live-raft-static-v1";

pub(crate) const REQUIRED_REPLICA_PORTS: [(&str, &str); molten_raft_core::admission::REQUIRED_REPLICA_PORT_COUNT] = [
    (
        crate::fabric_transport::FABRIC_TRANSPORT_PORT_ID,
        crate::fabric_transport::FABRIC_TRANSPORT_PORT_VERSION,
    ),
    (
        crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID,
        crate::fabric_durability::FABRIC_DURABILITY_PORT_VERSION,
    ),
    (
        crate::fabric_durability::FABRIC_SNAPSHOT_PORT_ID,
        crate::fabric_durability::FABRIC_DURABILITY_PORT_VERSION,
    ),
    (crate::fabric_time::FABRIC_TIMER_PORT_ID, crate::fabric_time::FABRIC_TIME_PORT_VERSION),
    (crate::fabric_time::FABRIC_ENTROPY_PORT_ID, crate::fabric_time::FABRIC_TIME_PORT_VERSION),
    (
        crate::fabric_membership::FABRIC_MEMBERSHIP_PORT_ID,
        crate::fabric_membership::FABRIC_MEMBERSHIP_PORT_VERSION,
    ),
    (
        crate::fabric_membership::FABRIC_PLACEMENT_PORT_ID,
        crate::fabric_membership::FABRIC_MEMBERSHIP_PORT_VERSION,
    ),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplicaPortBinding {
    pub port_id: String,
    pub version: String,
    pub implementation_profile: String,
    pub binding_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplicaStartInput {
    pub group: crate::fabric_consistency::Group,
    pub node_id: String,
    pub membership: super::StaticMembership,
    pub profile: super::ReplicaProfile,
    pub port_bindings: Vec<ReplicaPortBinding>,
}

/// Admit a replica start: the group binding and fabric port cohort here, then the kernel's
/// profile, membership, and initial-state admission.
// r[impl molten.fabric_consistency.live_service_ports]
pub(crate) fn plan_live_replica_start(input: ReplicaStartInput) -> crate::error::Result<super::ReplicaStartPlan> {
    validate_group(&input.group, &input.profile)?;
    molten_raft_core::admission::validate_profile(&input.profile).map_err(super::replica_failure)?;
    let membership = molten_raft_core::admission::validate_membership(
        &input.node_id,
        input.membership,
        &molten_raft_core::admission::MembershipBinding {
            membership_ref: &input.group.membership_ref,
            config_epoch: input.group.config_epoch,
        },
    )
    .map_err(super::replica_failure)?;
    let port_bindings = validate_port_bindings(input.port_bindings)?;
    let port_binding_refs = port_bindings.into_iter().map(|binding| binding.binding_ref).collect();
    debug_assert_eq!(input.profile.group_binding_ref, input.group.binding_ref);
    let state = molten_raft_core::admission::initial_state(
        input.node_id,
        membership,
        input.profile,
        &super::CanonicalReplicaIdentity,
    )
    .map_err(super::replica_failure)?;
    let initial_effects = molten_raft_core::admission::initial_effects(&state).map_err(super::replica_failure)?;
    Ok(super::ReplicaStartPlan {
        state,
        service_id: input.group.service_id,
        application_manifest_ref: input.group.application_manifest_ref,
        initial_effects,
        port_binding_refs,
        production_admitted: false,
    })
}

fn validate_group(
    group: &crate::fabric_consistency::Group,
    profile: &super::ReplicaProfile,
) -> crate::error::Result<()> {
    validate_group_integrity(group)?;
    if group.lifecycle != crate::fabric_consistency::GroupLifecycle::Active {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft startup requires an active consistency group",
        ));
    }
    if group.engine_algorithm_profile != LIVE_RAFT_ALGORITHM_PROFILE
        || group.engine_implementation_profile != LIVE_RAFT_IMPLEMENTATION_PROFILE
    {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft startup requires the exact admitted algorithm and implementation profile",
        ));
    }
    if profile.group_binding_ref != group.binding_ref {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft profile uses a substituted consistency-group binding",
        ));
    }
    if profile.service_generation != group.service_generation {
        return Err(crate::error::Failure::invalid_harness("live Raft profile uses a stale service generation"));
    }
    if profile.placement_ref != group.placement_ref
        || profile.fencing_ref != group.fencing_ref
        || profile.fencing_epoch != group.fencing_epoch
        || profile.resource_profile_ref != group.resource_profile_ref
    {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft profile placement, fencing, or resource binding does not match the group",
        ));
    }
    Ok(())
}

fn validate_group_integrity(group: &crate::fabric_consistency::Group) -> crate::error::Result<()> {
    let input = crate::fabric_consistency::GroupInput {
        group_id: group.group_id.clone(),
        extension_id: group.extension_id.clone(),
        service_id: group.service_id.clone(),
        service_generation: group.service_generation,
        application_manifest_ref: group.application_manifest_ref.clone(),
        engine_algorithm_profile: group.engine_algorithm_profile.clone(),
        engine_implementation_profile: group.engine_implementation_profile.clone(),
        membership_ref: group.membership_ref.clone(),
        config_epoch: group.config_epoch,
        placement_ref: group.placement_ref.clone(),
        fencing_ref: group.fencing_ref.clone(),
        fencing_epoch: group.fencing_epoch,
        resource_profile_ref: group.resource_profile_ref.clone(),
        policy_refs: group.policy_refs.clone(),
        non_claims: group.non_claims.clone(),
        supported_read_modes: group.supported_read_modes.clone(),
        max_command_bytes: group.max_command_bytes,
        max_in_flight_operations: group.max_in_flight_operations,
    };
    let expected_value = crate::fabric_consistency::canonical::binding_value(&input, group.lifecycle);
    let expected_ref = crate::preserves_rail::canonical_hash(&expected_value)?;
    if group.value != expected_value || group.binding_ref != expected_ref {
        return Err(crate::error::Failure::invalid_harness(
            "live Raft consistency-group binding failed canonical integrity validation",
        ));
    }
    Ok(())
}
