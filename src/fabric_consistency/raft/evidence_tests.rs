use super::tests::NODE_A;
use super::tests::active_group;
use super::tests::started_state;
use super::tests::test_ref;
use super::*;

const STARTUP_EVIDENCE_COUNT: usize = 2;
const RECOVERY_EVIDENCE_COUNT: usize = 3;
const FIRST_OBSERVATION_SEQUENCE: u32 = 0;

// r[verify molten.fabric_consistency.evidence_granularity]
#[test]
fn selected_evidence_records_milestones_and_suppresses_heartbeat_receipts() {
    let (plan, state) = start_plan();
    let mut ledger = ReplicaEvidenceLedger::new(&plan).expect("evidence ledger");
    assert_eq!(ledger.records().len(), STARTUP_EVIDENCE_COUNT);
    assert_eq!(ledger.records()[0].kind, ReplicaEvidenceKind::GroupAdmission);
    assert_eq!(ledger.records()[1].kind, ReplicaEvidenceKind::Configuration);

    ledger
        .observe(
            &state,
            &ReplicaEvent::HeartbeatTimeout,
            &ReplicaExecutionOutcome::Applied(ExecutedReplicaTransition {
                next: state.clone(),
                observations: Vec::new(),
            }),
        )
        .expect("suppressed heartbeat");
    assert_eq!(ledger.records().len(), STARTUP_EVIDENCE_COUNT);
    assert_eq!(ledger.suppressed_heartbeat_count(), 1);

    let mut committed = state.clone();
    committed.commit_index = INITIAL_LOG_INDEX;
    committed.last_applied = INITIAL_LOG_INDEX;
    ledger
        .observe(
            &state,
            &ReplicaEvent::Message {
                envelope: vote_envelope(&state),
            },
            &ReplicaExecutionOutcome::Applied(ExecutedReplicaTransition {
                next: committed.clone(),
                observations: vec![observation(ReplicaEffectKind::PersistCommit, "selected-commit")],
            }),
        )
        .expect("selected commit evidence");
    ledger
        .observe(
            &committed,
            &ReplicaEvent::Message {
                envelope: vote_envelope(&committed),
            },
            &ReplicaExecutionOutcome::Applied(ExecutedReplicaTransition {
                next: committed.clone(),
                observations: vec![observation(ReplicaEffectKind::ReadOutcome, "selected-read")],
            }),
        )
        .expect("selected read evidence");
    assert!(ledger.records().iter().any(|record| record.kind == ReplicaEvidenceKind::Commit));
    assert!(ledger.records().iter().any(|record| record.kind == ReplicaEvidenceKind::ReadCurrentness));

    let health = ledger.aggregate_health(&committed, false).expect("aggregate health");
    assert_eq!(health.status, "healthy");
    assert!(!health.production_admitted);
    crate::preserves_rail::validate_content_ref(&health.evidence_ref).expect("health evidence ref");
}

// r[verify molten.fabric_consistency.evidence_granularity]
#[test]
fn evidence_capacity_saturates_without_growth_and_failure_is_selected_when_space_exists() {
    let (plan, state) = start_plan();
    let mut bounded = ReplicaEvidenceLedger::with_capacity(&plan, STARTUP_EVIDENCE_COUNT).expect("bounded ledger");
    bounded
        .observe(&state, &ReplicaEvent::Stop, &ReplicaExecutionOutcome::Denied {
            retained: state.clone(),
            diagnostic: "injected denial".to_string(),
        })
        .expect("bounded failure observation");
    assert!(bounded.saturated());
    assert_eq!(bounded.records().len(), STARTUP_EVIDENCE_COUNT);

    let mut selected = ReplicaEvidenceLedger::new(&plan).expect("selected failure ledger");
    selected
        .observe(&state, &ReplicaEvent::Stop, &ReplicaExecutionOutcome::Denied {
            retained: state.clone(),
            diagnostic: "selected denial".to_string(),
        })
        .expect("selected failure");
    assert_eq!(selected.records().last().expect("failure record").kind, ReplicaEvidenceKind::Failure);
}

// r[verify molten.fabric_consistency.evidence_granularity]
#[test]
fn recovery_startup_selects_one_recovery_record() {
    let (mut plan, mut state) = start_plan();
    let mut snapshot = ReplicaSnapshot {
        snapshot_ref: String::new(),
        group_binding_ref: state.profile.group_binding_ref.clone(),
        membership_ref: state.membership.membership_ref.clone(),
        config_epoch: state.membership.config_epoch,
        fencing_epoch: state.profile.fencing_epoch,
        last_included_index: INITIAL_LOG_INDEX,
        last_included_term: INITIAL_LOG_INDEX,
        application_state_ref: test_ref("evidence-recovery-application"),
        completed_requests: Default::default(),
    };
    snapshot.snapshot_ref = snapshot_ref(&snapshot).expect("evidence snapshot identity");
    state.snapshot = Some(snapshot.clone());
    state.commit_index = INITIAL_LOG_INDEX;
    state.last_applied = INITIAL_LOG_INDEX;
    plan.state = state;
    plan.initial_effects = vec![ReplicaEffect::RestoreApplicationSnapshot { snapshot }];
    let ledger = ReplicaEvidenceLedger::new(&plan).expect("recovery evidence ledger");
    assert_eq!(ledger.records().len(), RECOVERY_EVIDENCE_COUNT);
    assert_eq!(ledger.records().last().expect("recovery record").kind, ReplicaEvidenceKind::Recovery);
}

fn start_plan() -> (ReplicaStartPlan, ReplicaState) {
    let group = active_group();
    let state = started_state(&group, NODE_A);
    (
        ReplicaStartPlan {
            state: state.clone(),
            service_id: group.service_id,
            application_manifest_ref: group.application_manifest_ref,
            initial_effects: Vec::new(),
            port_binding_refs: Vec::new(),
            production_admitted: false,
        },
        state,
    )
}

fn observation(kind: ReplicaEffectKind, label: &str) -> ReplicaEffectObservation {
    ReplicaEffectObservation {
        sequence: FIRST_OBSERVATION_SEQUENCE,
        kind,
        evidence_ref: test_ref(label),
    }
}

fn vote_envelope(state: &ReplicaState) -> ReplicaMessageEnvelope {
    ReplicaMessageEnvelope {
        group_binding_ref: state.profile.group_binding_ref.clone(),
        service_generation: state.profile.service_generation,
        from: "node-b".to_string(),
        to: NODE_A.to_string(),
        message: RaftMessage::VoteResponse {
            term: INITIAL_LOG_INDEX,
            voter_id: "node-b".to_string(),
            granted: true,
            config_epoch: state.membership.config_epoch,
            fencing_epoch: state.profile.fencing_epoch,
        },
    }
}
