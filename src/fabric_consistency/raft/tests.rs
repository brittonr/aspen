use super::*;
use crate::fabric_consistency::ConsistencyGroupBindingInput;
use crate::fabric_consistency::ConsistencyOperation;
use crate::fabric_consistency::ConsistencyOutcomeInput;
use crate::fabric_consistency::ConsistencyOutcomeKind;
use crate::fabric_consistency::ConsistencyPortCommandInput;
use crate::fabric_consistency::GroupOpenMode;
use crate::fabric_consistency::apply_consistency_outcome;
use crate::fabric_consistency::canonical_consistency_group_binding;
use crate::fabric_consistency::normalize_consistency_outcome;
use crate::fabric_consistency::plan_consistency_operation;

const SERVICE_GENERATION: u64 = 1;
const CONFIG_EPOCH: u64 = 1;
const FENCING_EPOCH: u64 = 1;
const HEARTBEAT_TICKS: u64 = 2;
const ELECTION_MIN_TICKS: u64 = 4;
const ELECTION_MAX_TICKS: u64 = 8;
const COMMAND_BYTE_LIMIT: u64 = 4_096;
const IN_FLIGHT_LIMIT: u32 = 8;
const EFFECT_LIMIT: usize = 16;
const EXPECTED_INITIAL_EFFECT_COUNT: usize = 2;
const NODE_A: &str = "node-a";
const NODE_B: &str = "node-b";
const NODE_C: &str = "node-c";

fn test_ref(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fabric-consistency-live-test-ref", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("test ref")
}

fn active_group() -> crate::fabric_consistency::ConsistencyGroupBinding {
    let declared = canonical_consistency_group_binding(ConsistencyGroupBindingInput {
        group_id: "group:live-raft".to_string(),
        extension_id: "extension-live-raft".to_string(),
        service_id: "service-live-raft".to_string(),
        service_generation: SERVICE_GENERATION,
        application_manifest_ref: test_ref("application-manifest"),
        engine_algorithm_profile: LIVE_RAFT_ALGORITHM_PROFILE.to_string(),
        engine_implementation_profile: LIVE_RAFT_IMPLEMENTATION_PROFILE.to_string(),
        membership_ref: test_ref("membership"),
        config_epoch: CONFIG_EPOCH,
        placement_ref: test_ref("placement"),
        fencing_ref: test_ref("fencing"),
        fencing_epoch: FENCING_EPOCH,
        resource_profile_ref: test_ref("resources"),
        policy_refs: vec![test_ref("policy")],
        non_claims: vec!["live-startup-does-not-prove-production-consensus".to_string()],
        supported_read_modes: vec![crate::fabric_consistency::ConsistencyReadMode::Linearizable],
        max_command_bytes: COMMAND_BYTE_LIMIT,
        max_in_flight_operations: IN_FLIGHT_LIMIT,
    })
    .expect("declared group");
    let plan = plan_consistency_operation(&declared, ConsistencyPortCommandInput {
        request_ref: test_ref("open-request"),
        binding_ref: declared.binding_ref.clone(),
        group_id: declared.group_id.clone(),
        extension_id: declared.extension_id.clone(),
        service_id: declared.service_id.clone(),
        service_generation: declared.service_generation,
        application_manifest_ref: declared.application_manifest_ref.clone(),
        engine_algorithm_profile: declared.engine_algorithm_profile.clone(),
        engine_implementation_profile: declared.engine_implementation_profile.clone(),
        membership_ref: declared.membership_ref.clone(),
        config_epoch: declared.config_epoch,
        placement_ref: declared.placement_ref.clone(),
        fencing_ref: declared.fencing_ref.clone(),
        fencing_epoch: declared.fencing_epoch,
        resource_profile_ref: declared.resource_profile_ref.clone(),
        policy_refs: declared.policy_refs.clone(),
        authority_refs: vec![test_ref("authority")],
        observed_in_flight_operations: 0,
        operation: ConsistencyOperation::Open {
            mode: GroupOpenMode::Create,
        },
    })
    .expect("open plan");
    let outcome = normalize_consistency_outcome(&declared, &plan, ConsistencyOutcomeInput {
        request_ref: plan.request_ref.clone(),
        binding_ref: declared.binding_ref.clone(),
        service_generation: declared.service_generation,
        config_epoch: declared.config_epoch,
        fencing_epoch: declared.fencing_epoch,
        kind: ConsistencyOutcomeKind::Opened,
        result_ref: Some(test_ref("open-result")),
        evidence_refs: vec![test_ref("open-evidence")],
        diagnostics: Vec::new(),
    })
    .expect("open outcome");
    apply_consistency_outcome(&declared, &plan, &outcome).expect("active group")
}

fn profile(group: &crate::fabric_consistency::ConsistencyGroupBinding) -> ReplicaProfile {
    ReplicaProfile {
        profile_ref: test_ref("live-raft-profile"),
        group_binding_ref: group.binding_ref.clone(),
        service_generation: group.service_generation,
        protocol_ref: test_ref("protocol"),
        durable_log_ref: test_ref("durable-log"),
        snapshot_store_ref: test_ref("snapshot-store"),
        timer_profile_ref: test_ref("timer-profile"),
        entropy_profile_ref: test_ref("entropy-profile"),
        placement_ref: group.placement_ref.clone(),
        fencing_ref: group.fencing_ref.clone(),
        fencing_epoch: group.fencing_epoch,
        supervision_ref: test_ref("supervision"),
        resource_profile_ref: group.resource_profile_ref.clone(),
        heartbeat_ticks: HEARTBEAT_TICKS,
        election_min_ticks: ELECTION_MIN_TICKS,
        election_max_ticks: ELECTION_MAX_TICKS,
        max_log_entries: MAX_REPLICA_LOG_ENTRIES,
        max_message_entries: MAX_REPLICA_MESSAGE_ENTRIES,
        max_effects_per_step: EFFECT_LIMIT,
    }
}

fn membership(group: &crate::fabric_consistency::ConsistencyGroupBinding) -> StaticMembership {
    StaticMembership {
        membership_ref: group.membership_ref.clone(),
        config_epoch: group.config_epoch,
        voters: vec![NODE_C.to_string(), NODE_A.to_string(), NODE_B.to_string()],
    }
}

fn port_bindings() -> Vec<ReplicaPortBinding> {
    REQUIRED_REPLICA_PORTS
        .iter()
        .map(|(port_id, version)| ReplicaPortBinding {
            port_id: (*port_id).to_string(),
            version: (*version).to_string(),
            implementation_profile: "live-test-profile-v1".to_string(),
            binding_ref: test_ref(port_id),
        })
        .collect()
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn live_replica_start_is_deterministic_and_production_denied() {
    let group = active_group();
    let input = ReplicaStartInput {
        node_id: NODE_A.to_string(),
        membership: membership(&group),
        profile: profile(&group),
        port_bindings: port_bindings(),
        group,
    };
    let first = plan_live_replica_start(input.clone()).expect("first live replica start plan");
    let second = plan_live_replica_start(input).expect("second live replica start plan");

    assert_eq!(first, second);
    assert_eq!(first.state.role, ReplicaRole::Follower);
    assert_eq!(first.state.lifecycle, ReplicaLifecycle::Running);
    assert_eq!(first.state.membership.voters, vec![NODE_A, NODE_B, NODE_C]);
    assert_eq!(first.initial_effects.len(), EXPECTED_INITIAL_EFFECT_COUNT);
    assert_eq!(first.port_binding_refs.len(), REQUIRED_REPLICA_PORTS.len());
    assert!(!first.production_admitted);
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn live_replica_start_denies_missing_durable_log_and_stale_generation() {
    let group = active_group();
    let mut missing_log_bindings = port_bindings();
    missing_log_bindings.retain(|binding| binding.port_id != crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID);
    let missing_log = plan_live_replica_start(ReplicaStartInput {
        node_id: NODE_A.to_string(),
        membership: membership(&group),
        profile: profile(&group),
        port_bindings: missing_log_bindings,
        group: group.clone(),
    })
    .expect_err("missing durable log binding must deny");
    assert!(missing_log.to_string().contains("exactly"));

    let mut stale_profile = profile(&group);
    stale_profile.service_generation = group.service_generation + 1;
    let stale = plan_live_replica_start(ReplicaStartInput {
        node_id: NODE_A.to_string(),
        membership: membership(&group),
        profile: stale_profile,
        port_bindings: port_bindings(),
        group,
    })
    .expect_err("stale generation must deny");
    assert!(stale.to_string().contains("stale service generation"));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn live_replica_start_denies_noncanonical_group_and_port_substitution() {
    let mut noncanonical_group = active_group();
    noncanonical_group.service_id = "service-substituted".to_string();
    let noncanonical = plan_live_replica_start(ReplicaStartInput {
        node_id: NODE_A.to_string(),
        membership: membership(&noncanonical_group),
        profile: profile(&noncanonical_group),
        port_bindings: port_bindings(),
        group: noncanonical_group,
    })
    .expect_err("noncanonical group must deny");
    assert!(noncanonical.to_string().contains("canonical integrity"));

    let group = active_group();
    let mut substituted_bindings = port_bindings();
    let durable = substituted_bindings
        .iter_mut()
        .find(|binding| binding.port_id == crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID)
        .expect("durable binding");
    durable.port_id = "molten.fabric.durability.substituted".to_string();
    let substituted = plan_live_replica_start(ReplicaStartInput {
        node_id: NODE_A.to_string(),
        membership: membership(&group),
        profile: profile(&group),
        port_bindings: substituted_bindings,
        group,
    })
    .expect_err("substituted durable port must deny");
    assert!(substituted.to_string().contains(crate::fabric_durability::FABRIC_DURABLE_LOG_PORT_ID));
}

// r[verify molten.fabric_consistency.live_service_ports]
#[test]
fn live_replica_start_denies_duplicate_membership_and_unsafe_timer_bounds() {
    let group = active_group();
    let mut duplicate_membership = membership(&group);
    duplicate_membership.voters = vec![NODE_A.to_string(), NODE_A.to_string(), NODE_C.to_string()];
    let duplicate = plan_live_replica_start(ReplicaStartInput {
        node_id: NODE_A.to_string(),
        membership: duplicate_membership,
        profile: profile(&group),
        port_bindings: port_bindings(),
        group: group.clone(),
    })
    .expect_err("duplicate voter must deny");
    assert!(duplicate.to_string().contains("duplicate voter"));

    let mut unsafe_timer_profile = profile(&group);
    unsafe_timer_profile.election_min_ticks = unsafe_timer_profile.heartbeat_ticks;
    let unsafe_timer = plan_live_replica_start(ReplicaStartInput {
        node_id: NODE_A.to_string(),
        membership: membership(&group),
        profile: unsafe_timer_profile,
        port_bindings: port_bindings(),
        group,
    })
    .expect_err("unsafe election bounds must deny");
    assert!(unsafe_timer.to_string().contains("heartbeat < election minimum"));
}
