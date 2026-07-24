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
const EXPECTED_SINGLE_LOG_ENTRY: usize = 1;
const STALE_EPOCH_STEP: u64 = 1;
const NEXT_INDEX_AFTER_FIRST_ENTRY: u64 = 2;
pub(super) const NODE_A: &str = "node-a";
pub(super) const NODE_B: &str = "node-b";
pub(super) const NODE_C: &str = "node-c";

pub(super) fn test_ref(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("fabric-consistency-live-test-ref", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("test ref")
}

pub(super) fn active_group() -> crate::fabric_consistency::ConsistencyGroupBinding {
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

pub(super) fn started_state(group: &crate::fabric_consistency::ConsistencyGroupBinding, node_id: &str) -> ReplicaState {
    plan_live_replica_start(ReplicaStartInput {
        node_id: node_id.to_string(),
        membership: membership(group),
        profile: profile(group),
        port_bindings: port_bindings(),
        group: group.clone(),
    })
    .expect("started replica")
    .state
}

pub(super) fn sent_envelope_to(transition: &ReplicaTransition, recipient: &str) -> ReplicaMessageEnvelope {
    transition
        .effects
        .iter()
        .find_map(|effect| match effect {
            ReplicaEffect::Send { envelope } if envelope.to == recipient => Some(envelope.clone()),
            _ => None,
        })
        .expect("sent Raft envelope")
}

pub(super) fn elect_node_a() -> (ReplicaState, ReplicaState) {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let node_b = started_state(&group, NODE_B);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("node A election");
    let vote_request = sent_envelope_to(&election, NODE_B);
    let vote =
        apply_replica_event(&node_b, ReplicaEvent::Message { envelope: vote_request }).expect("node B vote response");
    let vote_response = sent_envelope_to(&vote, NODE_A);
    let leader = apply_replica_event(&election.next, ReplicaEvent::Message {
        envelope: vote_response,
    })
    .expect("node A leadership");
    assert_eq!(leader.next.role, ReplicaRole::Leader);
    (leader.next, vote.next)
}

fn committed_leader() -> ReplicaState {
    let (leader, follower) = elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: test_ref("committed-helper-request"),
        command_ref: test_ref("committed-helper-command"),
        command_schema_ref: test_ref("committed-helper-schema"),
    })
    .expect("helper proposal");
    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: append }).expect("helper follower append");
    let response = sent_envelope_to(&replicated, NODE_A);
    apply_replica_event(&proposal.next, ReplicaEvent::Message { envelope: response })
        .expect("helper majority commit")
        .next
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_majority_replication_commits_and_applies_in_effect_order() {
    let (leader, follower) = elect_node_a();
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: test_ref("proposal-request"),
        command_ref: test_ref("proposal-command"),
        command_schema_ref: test_ref("proposal-schema"),
    })
    .expect("leader proposal");
    assert_eq!(proposal.next.commit_index, INITIAL_COMMIT_INDEX);
    assert!(matches!(proposal.effects.first(), Some(ReplicaEffect::PersistEntries { .. })));
    assert!(matches!(proposal.effects.get(1), Some(ReplicaEffect::FlushLog { .. })));

    let append = sent_envelope_to(&proposal, NODE_B);
    let replicated =
        apply_replica_event(&follower, ReplicaEvent::Message { envelope: append }).expect("follower append");
    let append_response = sent_envelope_to(&replicated, NODE_A);
    let committed = apply_replica_event(&proposal.next, ReplicaEvent::Message {
        envelope: append_response,
    })
    .expect("leader majority commit");

    assert_eq!(committed.next.commit_index, INITIAL_LOG_INDEX);
    assert_eq!(committed.next.last_applied, INITIAL_LOG_INDEX);
    assert!(committed.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ApplyCommitted { .. })));
    assert!(committed.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ProposalOutcome {
        disposition: ProposalDisposition::Committed,
        committed_index: Some(INITIAL_LOG_INDEX),
        ..
    })));

    let heartbeat = apply_replica_event(&committed.next, ReplicaEvent::HeartbeatTimeout).expect("commit heartbeat");
    let commit_notice = sent_envelope_to(&heartbeat, NODE_B);
    let follower_commit = apply_replica_event(&replicated.next, ReplicaEvent::Message {
        envelope: commit_notice,
    })
    .expect("follower commit application");
    assert_eq!(follower_commit.next.commit_index, INITIAL_LOG_INDEX);
    assert!(follower_commit.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ApplyCommitted { .. })));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_minority_and_duplicate_proposals_cannot_advance_commit() {
    let (leader, _follower) = elect_node_a();
    let request_ref = test_ref("minority-request");
    let proposal = apply_replica_event(&leader, ReplicaEvent::Propose {
        request_ref: request_ref.clone(),
        command_ref: test_ref("minority-command"),
        command_schema_ref: test_ref("minority-schema"),
    })
    .expect("minority proposal");
    assert_eq!(proposal.next.commit_index, INITIAL_COMMIT_INDEX);
    assert_eq!(proposal.next.log.len(), EXPECTED_SINGLE_LOG_ENTRY);

    let duplicate = apply_replica_event(&proposal.next, ReplicaEvent::Propose {
        request_ref,
        command_ref: test_ref("minority-command"),
        command_schema_ref: test_ref("minority-schema"),
    })
    .expect("duplicate proposal outcome");
    assert_eq!(duplicate.next.commit_index, INITIAL_COMMIT_INDEX);
    assert_eq!(duplicate.next.log.len(), EXPECTED_SINGLE_LOG_ENTRY);
    assert!(duplicate.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ProposalOutcome {
        disposition: ProposalDisposition::Retryable,
        committed_index: None,
        ..
    })));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_snapshot_compacts_only_through_committed_application_state() {
    let committed = committed_leader();
    let snapshot = apply_replica_event(&committed, ReplicaEvent::CreateSnapshot {
        application_state_ref: test_ref("snapshot-application-state"),
    })
    .expect("snapshot transition");
    let stored = snapshot.next.snapshot.as_ref().expect("snapshot state");

    assert_eq!(stored.last_included_index, committed.last_applied);
    assert_eq!(stored.snapshot_ref, snapshot_ref(stored).expect("snapshot identity"));
    assert!(snapshot.next.log.iter().all(|entry| entry.index > stored.last_included_index));
    assert!(matches!(snapshot.effects.as_slice(), [ReplicaEffect::PersistSnapshot { .. }]));
    let completed_request_ref = stored.completed_requests.keys().next().expect("completed request retained").clone();
    let duplicate = apply_replica_event(&snapshot.next, ReplicaEvent::Propose {
        request_ref: completed_request_ref,
        command_ref: test_ref("snapshot-duplicate-command"),
        command_schema_ref: test_ref("snapshot-duplicate-schema"),
    })
    .expect("compacted duplicate outcome");
    assert!(matches!(duplicate.effects.as_slice(), [ReplicaEffect::ProposalOutcome {
        disposition: ProposalDisposition::Committed,
        committed_index: Some(INITIAL_LOG_INDEX),
        ..
    }]));

    let group = active_group();
    let empty = started_state(&group, NODE_A);
    let empty_error = apply_replica_event(&empty, ReplicaEvent::CreateSnapshot {
        application_state_ref: test_ref("empty-snapshot-application-state"),
    })
    .expect_err("uncommitted snapshot must deny");
    assert!(empty_error.to_string().contains("committed application boundary"));

    let mut tampered = snapshot.next;
    tampered.snapshot.as_mut().expect("snapshot").application_state_ref = test_ref("tampered-application-state");
    let tamper_error = apply_replica_event(&tampered, ReplicaEvent::Read {
        request_ref: test_ref("tampered-snapshot-read"),
        mode: crate::fabric_consistency::ConsistencyReadMode::LocalStale,
    })
    .expect_err("tampered snapshot must deny before read");
    assert!(tamper_error.to_string().contains("snapshot binding or identity mismatch"));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_denies_superseded_election_timer_before_protocol_effects() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let superseded_timer_ref = node_a.active_election_timer_ref.clone();
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: superseded_timer_ref.clone(),
    })
    .expect("current election timer");

    let error = apply_replica_event(&election.next, ReplicaEvent::ElectionTimeout {
        timer_ref: superseded_timer_ref,
    })
    .expect_err("superseded timer must deny");
    assert!(error.to_string().contains("stale Raft election timer"));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_denies_stale_epoch_messages_without_state_mutation() {
    let group = active_group();
    let node_a = started_state(&group, NODE_A);
    let node_b = started_state(&group, NODE_B);
    let election = apply_replica_event(&node_a, ReplicaEvent::ElectionTimeout {
        timer_ref: node_a.active_election_timer_ref.clone(),
    })
    .expect("election request");
    let mut stale = sent_envelope_to(&election, NODE_B);
    match &mut stale.message {
        RaftMessage::RequestVote { config_epoch, .. } => {
            *config_epoch += STALE_EPOCH_STEP;
        }
        other => panic!("expected request vote, got {other:?}"),
    }
    let before = node_b.clone();
    let error = apply_replica_event(&node_b, ReplicaEvent::Message { envelope: stale })
        .expect_err("stale config epoch must deny");
    assert!(error.to_string().contains("stale configuration or fencing epoch"));
    assert_eq!(node_b, before);
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_keeps_linearizable_reads_retryable_without_a_read_barrier() {
    let (leader, follower) = elect_node_a();
    let linearizable = apply_replica_event(&leader, ReplicaEvent::Read {
        request_ref: test_ref("linearizable-read"),
        mode: crate::fabric_consistency::ConsistencyReadMode::Linearizable,
    })
    .expect("linearizable read outcome");
    assert!(linearizable.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome {
        disposition: ReadDisposition::Retryable,
        ..
    })));

    let local = apply_replica_event(&follower, ReplicaEvent::Read {
        request_ref: test_ref("local-read"),
        mode: crate::fabric_consistency::ConsistencyReadMode::LocalStale,
    })
    .expect("local stale read outcome");
    assert!(local.effects.iter().any(|effect| matches!(effect, ReplicaEffect::ReadOutcome {
        disposition: ReadDisposition::Local,
        ..
    })));
}

// r[verify molten.fabric_consistency.live_raft]
#[test]
fn live_raft_ignores_out_of_order_append_failure_after_success() {
    let leader = committed_leader();
    assert_eq!(leader.next_index.get(NODE_B), Some(&NEXT_INDEX_AFTER_FIRST_ENTRY));
    let stale_failure = ReplicaMessageEnvelope {
        group_binding_ref: leader.profile.group_binding_ref.clone(),
        service_generation: leader.profile.service_generation,
        from: NODE_B.to_string(),
        to: NODE_A.to_string(),
        message: RaftMessage::AppendResponse {
            term: leader.current_term,
            follower_id: NODE_B.to_string(),
            success: false,
            request_prev_log_index: INITIAL_COMMIT_INDEX,
            match_index: INITIAL_COMMIT_INDEX,
            conflict_index: INITIAL_LOG_INDEX,
            config_epoch: leader.membership.config_epoch,
            fencing_epoch: leader.profile.fencing_epoch,
        },
    };
    let after = apply_replica_event(&leader, ReplicaEvent::Message {
        envelope: stale_failure,
    })
    .expect("stale append failure ignored");
    assert_eq!(after.next.next_index.get(NODE_B), Some(&NEXT_INDEX_AFTER_FIRST_ENTRY));
    assert!(after.effects.is_empty());
}
