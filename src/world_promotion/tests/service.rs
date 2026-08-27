use molten_core::world_commit::WorldCommitRef;
use molten_core::world_promotion::*;

use super::super::*;
use super::support::*;
use crate::world_head::WorldHeadStatePort;

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
