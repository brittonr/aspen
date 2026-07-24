use std::collections::BTreeSet;

use super::INITIAL_COMMIT_INDEX;
use super::INITIAL_ELECTION_TIMER_SEQUENCE;
use super::INITIAL_TERM;
use super::MAX_REPLICA_EFFECTS;
use super::MAX_REPLICA_LOG_ENTRIES;
use super::MAX_REPLICA_MESSAGE_ENTRIES;
use super::ReplicaEffect;
use super::ReplicaLifecycle;
use super::ReplicaProfile;
use super::ReplicaRole;
use super::ReplicaState;
use super::STATIC_VOTER_COUNT;
use super::StaticMembership;
use super::election_timer_ref;
use crate::error::MoltenError;
use crate::error::Result;
use crate::fabric_consistency::ConsistencyGroupBinding;
use crate::fabric_consistency::ConsistencyGroupLifecycle;
use crate::fabric_durability::FABRIC_DURABILITY_PORT_VERSION;
use crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID;
use crate::fabric_durability::FABRIC_SNAPSHOT_PORT_ID;
use crate::fabric_membership::FABRIC_MEMBERSHIP_PORT_ID;
use crate::fabric_membership::FABRIC_MEMBERSHIP_PORT_VERSION;
use crate::fabric_membership::FABRIC_PLACEMENT_PORT_ID;
use crate::fabric_time::FABRIC_ENTROPY_PORT_ID;
use crate::fabric_time::FABRIC_TIME_PORT_VERSION;
use crate::fabric_time::FABRIC_TIMER_PORT_ID;
use crate::fabric_transport::FABRIC_TRANSPORT_PORT_ID;
use crate::fabric_transport::FABRIC_TRANSPORT_PORT_VERSION;

pub const LIVE_RAFT_ALGORITHM_PROFILE: &str = "raft";
pub const LIVE_RAFT_IMPLEMENTATION_PROFILE: &str = "live-raft-static-v1";

const REQUIRED_REPLICA_PORT_COUNT: usize = 7;
const MINIMUM_LIVE_REPLICA_STEP_EFFECTS: usize = 6;
const MAX_REPLICA_IDENTIFIER_BYTES: usize = 256;

pub(crate) const REQUIRED_REPLICA_PORTS: [(&str, &str); REQUIRED_REPLICA_PORT_COUNT] = [
    (FABRIC_TRANSPORT_PORT_ID, FABRIC_TRANSPORT_PORT_VERSION),
    (FABRIC_DURABLE_LOG_PORT_ID, FABRIC_DURABILITY_PORT_VERSION),
    (FABRIC_SNAPSHOT_PORT_ID, FABRIC_DURABILITY_PORT_VERSION),
    (FABRIC_TIMER_PORT_ID, FABRIC_TIME_PORT_VERSION),
    (FABRIC_ENTROPY_PORT_ID, FABRIC_TIME_PORT_VERSION),
    (FABRIC_MEMBERSHIP_PORT_ID, FABRIC_MEMBERSHIP_PORT_VERSION),
    (FABRIC_PLACEMENT_PORT_ID, FABRIC_MEMBERSHIP_PORT_VERSION),
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
    pub group: ConsistencyGroupBinding,
    pub node_id: String,
    pub membership: StaticMembership,
    pub profile: ReplicaProfile,
    pub port_bindings: Vec<ReplicaPortBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaStartPlan {
    pub state: ReplicaState,
    pub service_id: String,
    pub application_manifest_ref: String,
    pub initial_effects: Vec<ReplicaEffect>,
    pub port_binding_refs: Vec<String>,
    pub production_admitted: bool,
}

// r[impl molten.fabric_consistency.live_service_ports]
pub(crate) fn plan_live_replica_start(input: ReplicaStartInput) -> Result<ReplicaStartPlan> {
    validate_group(&input.group, &input.profile)?;
    validate_profile(&input.profile)?;
    let membership = validate_membership(&input.group, &input.node_id, input.membership)?;
    let port_bindings = validate_port_bindings(input.port_bindings)?;
    let port_binding_refs = port_bindings.into_iter().map(|binding| binding.binding_ref).collect();
    let service_id = input.group.service_id.clone();
    let application_manifest_ref = input.group.application_manifest_ref.clone();
    let state = initial_state(input.group, input.node_id, membership, input.profile)?;
    let initial_effects = initial_effects(&state)?;
    Ok(ReplicaStartPlan {
        state,
        service_id,
        application_manifest_ref,
        initial_effects,
        port_binding_refs,
        production_admitted: false,
    })
}

fn validate_group(group: &ConsistencyGroupBinding, profile: &ReplicaProfile) -> Result<()> {
    validate_group_integrity(group)?;
    if group.lifecycle != ConsistencyGroupLifecycle::Active {
        return Err(MoltenError::invalid_harness("live Raft startup requires an active consistency group"));
    }
    if group.engine_algorithm_profile != LIVE_RAFT_ALGORITHM_PROFILE
        || group.engine_implementation_profile != LIVE_RAFT_IMPLEMENTATION_PROFILE
    {
        return Err(MoltenError::invalid_harness(
            "live Raft startup requires the exact admitted algorithm and implementation profile",
        ));
    }
    if profile.group_binding_ref != group.binding_ref {
        return Err(MoltenError::invalid_harness("live Raft profile uses a substituted consistency-group binding"));
    }
    if profile.service_generation != group.service_generation {
        return Err(MoltenError::invalid_harness("live Raft profile uses a stale service generation"));
    }
    if profile.placement_ref != group.placement_ref
        || profile.fencing_ref != group.fencing_ref
        || profile.fencing_epoch != group.fencing_epoch
        || profile.resource_profile_ref != group.resource_profile_ref
    {
        return Err(MoltenError::invalid_harness(
            "live Raft profile placement, fencing, or resource binding does not match the group",
        ));
    }
    Ok(())
}

fn validate_group_integrity(group: &ConsistencyGroupBinding) -> Result<()> {
    let input = crate::fabric_consistency::ConsistencyGroupBindingInput {
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
        return Err(MoltenError::invalid_harness(
            "live Raft consistency-group binding failed canonical integrity validation",
        ));
    }
    Ok(())
}

fn validate_profile(profile: &ReplicaProfile) -> Result<()> {
    for (reference, label) in [
        (&profile.profile_ref, "live Raft profile ref"),
        (&profile.group_binding_ref, "live Raft group binding ref"),
        (&profile.protocol_ref, "live Raft protocol ref"),
        (&profile.durable_log_ref, "live Raft durable log ref"),
        (&profile.snapshot_store_ref, "live Raft snapshot store ref"),
        (&profile.timer_profile_ref, "live Raft timer profile ref"),
        (&profile.entropy_profile_ref, "live Raft entropy profile ref"),
        (&profile.placement_ref, "live Raft placement ref"),
        (&profile.fencing_ref, "live Raft fencing ref"),
        (&profile.supervision_ref, "live Raft supervision ref"),
        (&profile.resource_profile_ref, "live Raft resource profile ref"),
    ] {
        validate_content_ref(reference, label)?;
    }
    if profile.service_generation == 0 || profile.fencing_epoch == 0 {
        return Err(MoltenError::invalid_harness("live Raft generation and fencing epoch must be positive"));
    }
    if profile.heartbeat_ticks == 0
        || profile.election_min_ticks <= profile.heartbeat_ticks
        || profile.election_max_ticks < profile.election_min_ticks
    {
        return Err(MoltenError::invalid_harness(
            "live Raft timer bounds require heartbeat < election minimum <= election maximum",
        ));
    }
    if profile.max_log_entries == 0 || profile.max_log_entries > MAX_REPLICA_LOG_ENTRIES {
        return Err(MoltenError::invalid_harness("live Raft log-entry bound is outside the admitted range"));
    }
    if profile.max_message_entries == 0 || profile.max_message_entries > MAX_REPLICA_MESSAGE_ENTRIES {
        return Err(MoltenError::invalid_harness("live Raft message-entry bound is outside the admitted range"));
    }
    if profile.max_effects_per_step < MINIMUM_LIVE_REPLICA_STEP_EFFECTS
        || profile.max_effects_per_step > MAX_REPLICA_EFFECTS
    {
        return Err(MoltenError::invalid_harness("live Raft effect bound cannot admit a complete static-replica step"));
    }
    Ok(())
}

fn validate_membership(
    group: &ConsistencyGroupBinding,
    node_id: &str,
    mut membership: StaticMembership,
) -> Result<StaticMembership> {
    validate_identifier(node_id, "live Raft node id")?;
    validate_content_ref(&membership.membership_ref, "live Raft membership ref")?;
    if membership.membership_ref != group.membership_ref || membership.config_epoch != group.config_epoch {
        return Err(MoltenError::invalid_harness("live Raft membership ref or configuration epoch is stale"));
    }
    if membership.voters.len() != STATIC_VOTER_COUNT {
        return Err(MoltenError::invalid_harness(format!(
            "initial live Raft profile requires exactly {STATIC_VOTER_COUNT} voters"
        )));
    }
    let mut unique = BTreeSet::new();
    for voter in &membership.voters {
        validate_identifier(voter, "live Raft voter id")?;
        if !unique.insert(voter.as_str()) {
            return Err(MoltenError::invalid_harness("live Raft membership contains a duplicate voter"));
        }
    }
    if !unique.contains(node_id) {
        return Err(MoltenError::invalid_harness("live Raft node is absent from the admitted static membership"));
    }
    membership.voters.sort();
    Ok(membership)
}

fn validate_port_bindings(mut bindings: Vec<ReplicaPortBinding>) -> Result<Vec<ReplicaPortBinding>> {
    if bindings.len() != REQUIRED_REPLICA_PORT_COUNT {
        return Err(MoltenError::invalid_harness(format!(
            "live Raft startup requires exactly {REQUIRED_REPLICA_PORT_COUNT} admitted fabric port bindings"
        )));
    }
    let mut unique = BTreeSet::new();
    for binding in &bindings {
        validate_identifier(&binding.implementation_profile, "live Raft port implementation profile")?;
        validate_content_ref(&binding.binding_ref, "live Raft port binding ref")?;
        if !unique.insert((binding.port_id.as_str(), binding.version.as_str())) {
            return Err(MoltenError::invalid_harness("live Raft startup contains a duplicate fabric port binding"));
        }
    }
    for (port_id, version) in REQUIRED_REPLICA_PORTS {
        if !unique.contains(&(port_id, version)) {
            return Err(MoltenError::invalid_harness(format!(
                "live Raft startup is missing required fabric port {port_id}@{version}"
            )));
        }
    }
    bindings.sort_by(|left, right| (&left.port_id, &left.version).cmp(&(&right.port_id, &right.version)));
    Ok(bindings)
}

fn initial_effects(state: &ReplicaState) -> Result<Vec<ReplicaEffect>> {
    let effects = vec![
        ReplicaEffect::PersistHardState {
            term: INITIAL_TERM,
            voted_for: None,
        },
        ReplicaEffect::ArmElectionTimer {
            timer_ref: state.active_election_timer_ref.clone(),
        },
    ];
    if effects.len() > state.profile.max_effects_per_step {
        return Err(MoltenError::invalid_harness("live Raft startup exceeds its admitted effect bound"));
    }
    Ok(effects)
}

fn initial_state(
    group: ConsistencyGroupBinding,
    node_id: String,
    membership: StaticMembership,
    profile: ReplicaProfile,
) -> Result<ReplicaState> {
    debug_assert_eq!(profile.group_binding_ref, group.binding_ref);
    let active_election_timer_ref = election_timer_ref(
        &profile.group_binding_ref,
        &node_id,
        profile.service_generation,
        INITIAL_TERM,
        INITIAL_ELECTION_TIMER_SEQUENCE,
    )?;
    Ok(ReplicaState {
        profile,
        node_id,
        membership,
        role: ReplicaRole::Follower,
        lifecycle: ReplicaLifecycle::Running,
        current_term: INITIAL_TERM,
        election_timer_sequence: INITIAL_ELECTION_TIMER_SEQUENCE,
        active_election_timer_ref,
        voted_for: None,
        leader_id: None,
        log: Vec::new(),
        commit_index: INITIAL_COMMIT_INDEX,
        last_applied: INITIAL_COMMIT_INDEX,
        snapshot: None,
        votes_received: BTreeSet::new(),
        next_index: Default::default(),
        match_index: Default::default(),
        quorum_confirmed_term: None,
    })
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty() || value.len() > MAX_REPLICA_IDENTIFIER_BYTES {
        return Err(MoltenError::invalid_harness(format!(
            "{label} must be non-empty and at most {MAX_REPLICA_IDENTIFIER_BYTES} bytes"
        )));
    }
    if !value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':')) {
        return Err(MoltenError::invalid_harness(format!("{label} contains unsupported characters")));
    }
    Ok(())
}

fn validate_content_ref(value: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label}: {error}")))
}
