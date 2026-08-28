use molten_core::world_commit::SnapshotProfileRef;
use molten_core::world_commit::WorldCommitRef;
use molten_core::world_promotion::*;

use super::super::*;
use super::support::*;
use crate::deterministic_replay::ConsumedEffect;
use crate::deterministic_replay::EffectLogEntry;
use crate::deterministic_replay::EffectLogValidationInput;
use crate::deterministic_replay::validate_effect_log;
use crate::world_head::WorldHeadStatePort;

const PROMOTION_EFFECT_SEQUENCE: u64 = 0;
const OBSERVATION_BYTES: u64 = 64;

// r[verify molten.world_promotion.transaction]
#[test]
fn atomic_promotion_moves_head_and_commits_every_reservation() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let mut current = Current { is_admitted: true };
    let mut receipts = receipts();
    let outcome = promote_world(&request, WorldPromotionPorts {
        current: &mut current,
        transaction: &mut state.store,
        receipts: &mut receipts,
    })
    .expect("atomic promotion");
    assert!(outcome.persistence.dispatch_eligible);
    assert_eq!(outcome.committed_reservations.len(), EXPECTED_RESERVATIONS);
    assert!(
        outcome
            .committed_reservations
            .iter()
            .all(|reservation| reservation.state == WorldReleaseState::Committed)
    );
    let head = state.store.head_store_mut().read_head(&request.branch_id).expect("head read").expect("active head");
    assert_eq!(head, outcome.plan.after);
    assert_eq!(state.store.list_reservations().expect("reservations").len(), EXPECTED_RESERVATIONS);
    assert_eq!(receipts.count, 1);
    assert_eq!(receipts.events.borrow().last(), Some(&"receipt"));

    drop(state.store);
    let restarted = LocalWorldPromotionStore::open(&state.storage).expect("restarted store");
    assert_eq!(restarted.list_reservations().expect("restarted reservations").len(), EXPECTED_RESERVATIONS);
}

// r[verify molten.world_promotion.dispatch]
#[test]
fn dispatch_records_attempt_before_effect_and_denial_never_calls_adapter() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let mut current = Current { is_admitted: true };
    let mut promotion_receipts = receipts();
    let promoted = promote_world(&request, WorldPromotionPorts {
        current: &mut current,
        transaction: &mut state.store,
        receipts: &mut promotion_receipts,
    })
    .expect("promotion");
    let reservation = promoted.committed_reservations[0].clone();
    let actual_attempt = expected_initial_attempt(&reservation);
    let mut admission = Admission {
        facts: dispatch_facts(),
    };
    let mut dispatcher = Dispatcher {
        calls: 0,
        observation: WorldAttemptObservation::Succeeded(
            WorldReleaseObservationRef::new(reference("dispatch-success")).expect("observation ref"),
        ),
    };
    let mut dispatch_receipts = receipts();
    let dispatched =
        dispatch_world_reservation(&promoted.plan, &reservation.reservation_ref, &actual_attempt, WorldDispatchPorts {
            transaction: &mut state.store,
            admission: &mut admission,
            dispatcher: &mut dispatcher,
            receipts: &mut dispatch_receipts,
        })
        .expect("dispatch");
    assert_eq!(dispatched.decision, WorldDispatchDecision::Dispatched);
    assert_eq!(dispatcher.calls, 1);
    assert_eq!(dispatched.attempt.as_ref().expect("attempt").state, WorldReleaseState::Observed);
    assert!(state.store.read_attempt(&actual_attempt).expect("attempt read").is_some());

    let second_request = promotion_request_for_branch("release-two");
    let mut second_state = test_state(&second_request);
    let mut second_current = Current { is_admitted: true };
    let mut second_promotion_receipts = receipts();
    let second = promote_world(&second_request, WorldPromotionPorts {
        current: &mut second_current,
        transaction: &mut second_state.store,
        receipts: &mut second_promotion_receipts,
    })
    .expect("second promotion");
    let second_reservation = second.committed_reservations[0].clone();
    let mut denied_facts = dispatch_facts();
    denied_facts.capability_admitted = false;
    let mut denied_admission = Admission { facts: denied_facts };
    let mut denied_dispatcher = Dispatcher {
        calls: 0,
        observation: WorldAttemptObservation::Unknown,
    };
    let mut denied_receipts = receipts();
    let blocked = dispatch_world_reservation(
        &second.plan,
        &second_reservation.reservation_ref,
        &expected_initial_attempt(&second_reservation),
        WorldDispatchPorts {
            transaction: &mut second_state.store,
            admission: &mut denied_admission,
            dispatcher: &mut denied_dispatcher,
            receipts: &mut denied_receipts,
        },
    )
    .expect("blocked dispatch");
    assert_eq!(blocked.decision, WorldDispatchDecision::Blocked);
    assert_eq!(denied_dispatcher.calls, 0);
    assert_eq!(blocked.reservation.state, WorldReleaseState::Blocked);
}

// r[verify molten.world_promotion.reconciliation]
#[test]
fn unknown_commit_uses_readback_before_reporting_dispatch_eligibility() {
    let request = promotion_request();
    let mut current = Current { is_admitted: true };
    let mut transaction = UnknownTransaction {
        readback_calls: std::cell::Cell::new(0),
    };
    let mut receipt_port = receipts();
    let outcome = promote_world(&request, WorldPromotionPorts {
        current: &mut current,
        transaction: &mut transaction,
        receipts: &mut receipt_port,
    })
    .expect("readback reconciliation");
    assert_eq!(transaction.readback_calls.get(), 1);
    assert!(outcome.persistence.dispatch_eligible);
    assert_eq!(receipt_port.count, 1);
}

// r[verify molten.world_promotion.observation_commit]
// r[verify molten.world_promotion.verification]
#[test]
fn effect_log_parity_binds_acknowledged_observation_to_follow_up_commit() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let mut current = Current { is_admitted: true };
    let mut promotion_receipts = receipts();
    let promoted = promote_world(&request, WorldPromotionPorts {
        current: &mut current,
        transaction: &mut state.store,
        receipts: &mut promotion_receipts,
    })
    .expect("promotion");
    let reservation = promoted.committed_reservations[0].clone();
    let attempt_ref = expected_initial_attempt(&reservation);
    let observation_ref =
        WorldReleaseObservationRef::new(reference("effect-log-observation")).expect("observation ref");
    let mut admission = Admission {
        facts: dispatch_facts(),
    };
    let mut dispatcher = Dispatcher {
        calls: 0,
        observation: WorldAttemptObservation::Succeeded(observation_ref.clone()),
    };
    let mut dispatch_receipts = receipts();
    let dispatched =
        dispatch_world_reservation(&promoted.plan, &reservation.reservation_ref, &attempt_ref, WorldDispatchPorts {
            transaction: &mut state.store,
            admission: &mut admission,
            dispatcher: &mut dispatcher,
            receipts: &mut dispatch_receipts,
        })
        .expect("dispatch");
    let acknowledged =
        acknowledge_attempt(dispatched.attempt.as_ref().expect("attempt")).expect("acknowledged attempt");

    let entry = EffectLogEntry {
        sequence: PROMOTION_EFFECT_SEQUENCE,
        effect_kind: "world-release".to_string(),
        run_identity_ref: promoted.plan.plan_ref.as_str().to_string(),
        handler_profile_ref: reservation.handler_ref.as_str().to_string(),
        turn_ref: acknowledged.attempt_ref.as_str().to_string(),
        boundary_ref: reservation.reservation_ref.as_str().to_string(),
        request_ref: reservation.intent_ref.as_str().to_string(),
        response_ref: observation_ref.as_str().to_string(),
    };
    let consumed = ConsumedEffect {
        sequence: PROMOTION_EFFECT_SEQUENCE,
        effect_kind: entry.effect_kind.clone(),
        request_ref: entry.request_ref.clone(),
        response_ref: entry.response_ref.clone(),
        boundary_ref: entry.boundary_ref.clone(),
        used_live_fallback: false,
    };
    let validation = validate_effect_log(EffectLogValidationInput {
        expected_run_identity_ref: &entry.run_identity_ref,
        expected_handler_profile_ref: &entry.handler_profile_ref,
        entries: std::slice::from_ref(&entry),
        consumed: std::slice::from_ref(&consumed),
    })
    .expect("effect log validation");
    assert_eq!(validation.decision, "pass");

    let follow_up = plan_world_promotion_observation_commit(&WorldPromotionObservationCommitRequest {
        reservation,
        attempt: acknowledged,
        successor_commit: WorldCommitRef::new(reference("observation-successor")).expect("successor"),
        logical_profile_ref: SnapshotProfileRef::new(reference("logical-world-profile")).expect("profile"),
        observation_schema_ref: reference("world-release-observation-schema"),
        observation_byte_length: OBSERVATION_BYTES,
    })
    .expect("follow-up observation commit");
    assert_eq!(follow_up.trace.steps[0].input.input_ref, observation_ref.as_str());
    assert!(!follow_up.mutates_promoted_commit);
    assert!(!follow_up.grants_dispatch_authority);
}

// r[verify molten.world_promotion.verification]
#[test]
fn promotion_effect_log_rejects_missing_mismatched_and_live_fallback_outcomes() {
    let run_ref = reference("promotion-run");
    let handler_ref = reference("promotion-handler");
    let request_ref = reference("promotion-request");
    let response_ref = reference("promotion-response");
    let boundary_ref = reference("promotion-boundary");
    let entry = EffectLogEntry {
        sequence: PROMOTION_EFFECT_SEQUENCE,
        effect_kind: "world-release".to_string(),
        run_identity_ref: run_ref.clone(),
        handler_profile_ref: handler_ref.clone(),
        turn_ref: reference("promotion-turn"),
        boundary_ref: boundary_ref.clone(),
        request_ref: request_ref.clone(),
        response_ref: response_ref.clone(),
    };
    let consumed = ConsumedEffect {
        sequence: PROMOTION_EFFECT_SEQUENCE,
        effect_kind: entry.effect_kind.clone(),
        request_ref,
        response_ref: response_ref.clone(),
        boundary_ref,
        used_live_fallback: false,
    };

    let missing = validate_effect_log(EffectLogValidationInput {
        expected_run_identity_ref: &run_ref,
        expected_handler_profile_ref: &handler_ref,
        entries: &[],
        consumed: std::slice::from_ref(&consumed),
    })
    .expect("missing validation");
    assert_eq!(missing.decision, "deny");

    let mut mismatched = consumed.clone();
    mismatched.response_ref = reference("other-response");
    let mismatch = validate_effect_log(EffectLogValidationInput {
        expected_run_identity_ref: &run_ref,
        expected_handler_profile_ref: &handler_ref,
        entries: std::slice::from_ref(&entry),
        consumed: std::slice::from_ref(&mismatched),
    })
    .expect("mismatch validation");
    assert_eq!(mismatch.decision, "deny");

    let mut live = consumed;
    live.used_live_fallback = true;
    let fallback = validate_effect_log(EffectLogValidationInput {
        expected_run_identity_ref: &run_ref,
        expected_handler_profile_ref: &handler_ref,
        entries: std::slice::from_ref(&entry),
        consumed: std::slice::from_ref(&live),
    })
    .expect("fallback validation");
    assert_eq!(fallback.decision, "deny");
}

#[test]
fn stale_current_authority_leaves_head_and_outbox_unchanged() {
    let request = promotion_request();
    let mut state = test_state(&request);
    let before = state.store.head_store_mut().read_head(&request.branch_id).expect("head read");
    let mut current = Current { is_admitted: false };
    let mut receipt_port = receipts();
    assert!(
        promote_world(&request, WorldPromotionPorts {
            current: &mut current,
            transaction: &mut state.store,
            receipts: &mut receipt_port,
        })
        .is_err()
    );
    let after = state.store.head_store_mut().read_head(&request.branch_id).expect("head read");
    assert_eq!(before, after);
    assert!(state.store.list_reservations().expect("reservations").is_empty());
    assert_eq!(receipt_port.count, 0);
}

fn promotion_request_for_branch(branch: &str) -> WorldPromotionRequest {
    let mut request = promotion_request();
    request.branch_id = molten_core::world_head::WorldBranchId::new(branch).expect("branch");
    request.operation_ref =
        WorldPromotionOperationRef::new(reference(&format!("operation:{branch}"))).expect("operation ref");
    request.expected_head = WorldCommitRef::new(reference(&format!("active:{branch}"))).expect("head");
    request.candidate_head = WorldCommitRef::new(reference(&format!("candidate:{branch}"))).expect("candidate");
    request
}

fn expected_initial_attempt(reservation: &WorldReleaseReservation) -> WorldReleaseAttemptRef {
    let mut hasher = blake3::Hasher::new_derive_key("onixresearch.molten.world-promotion.transaction-attempt.v1");
    for field in [reservation.reservation_ref.as_str(), reservation.operation_ref.as_str()] {
        let length = u64::try_from(field.len()).expect("field length");
        hasher.update(&length.to_be_bytes());
        hasher.update(field.as_bytes());
    }
    WorldReleaseAttemptRef::new(format!("blake3:{}", hasher.finalize().to_hex())).expect("attempt ref")
}
