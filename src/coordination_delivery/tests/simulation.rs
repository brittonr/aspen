use molten_core::coordination_delivery::*;
use molten_core::fabric_simulation::SimulationFaultKind;

use super::super::*;
use super::support::*;

const DUPLICATE_TRANSITION_COUNT: usize = 2;

fn claimed_state() -> (
    DeliveryPolicy,
    DeliveryManifest,
    molten_core::fabric_time::AdmittedTimeProfile,
    DeliveryState,
    DeliveryToken,
) {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let initial = DeliveryState::empty(QUEUE_ID, manifest.policy_ref.clone(), SERVICE_GENERATION, CONSISTENCY_EPOCH);
    let enqueue = enqueue_request(&manifest, '1');
    let enqueued = plan_delivery_transition(&DeliveryTransitionInput {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        state: &initial,
        request: &enqueue,
    });
    let claim = request(&manifest, '2', INITIAL_TICK, DeliveryOperation::Claim);
    let claimed = plan_delivery_transition(&DeliveryTransitionInput {
        manifest: &manifest,
        policy: &policy,
        time_profile: &time,
        state: &enqueued.next_state,
        request: &claim,
    });
    let token = claimed.token.clone().expect("token");
    (policy, manifest, time, claimed.next_state, token)
}

// r[verify molten.coordination_delivery.final_validation]
#[test]
fn crash_restart_preserves_claim_and_partitioned_ack_cannot_complete_it() {
    let (policy, manifest, time, state, token) = claimed_state();
    let partitioned_ack =
        request(&manifest, '3', INITIAL_TICK + 1, DeliveryOperation::Acknowledge { token: token.clone() });
    let valid_ack = request(&manifest, '4', INITIAL_TICK + 1, DeliveryOperation::Acknowledge { token });
    let trace = run_delivery_simulation(&manifest, &policy, &time, state, &[
        DeliverySimulationAction::CrashRestart,
        DeliverySimulationAction::FaultedRequest {
            fault: SimulationFaultKind::Partition,
            request: partitioned_ack,
        },
        DeliverySimulationAction::Request(valid_ack),
    ])
    .expect("simulation");
    assert_eq!(trace.transitions[0].issue, Some(DeliveryIssue::CurrentnessRequired));
    assert_eq!(trace.transitions[1].kind, DeliveryTransitionKind::Acknowledged);
    assert_eq!(trace.final_state.completed.len(), 1);
    assert!(trace.fault_classes.contains(&"crash".to_string()));
    assert!(trace.fault_classes.contains(&"restart".to_string()));
    assert!(trace.fault_classes.contains(&"partition".to_string()));
}

// r[verify molten.coordination_delivery.final_validation]
#[test]
fn duplicate_fault_reuses_the_same_core_operation_record() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let initial = DeliveryState::empty(QUEUE_ID, manifest.policy_ref.clone(), SERVICE_GENERATION, CONSISTENCY_EPOCH);
    let trace =
        run_delivery_simulation(&manifest, &policy, &time, initial, &[DeliverySimulationAction::FaultedRequest {
            fault: SimulationFaultKind::Duplicate,
            request: enqueue_request(&manifest, '1'),
        }])
        .expect("duplicate simulation");
    assert_eq!(trace.transitions.len(), DUPLICATE_TRANSITION_COUNT);
    assert_eq!(trace.transitions[1].decision, DeliveryDecisionKind::DuplicateReplay);
    assert_eq!(trace.final_state.ready.len(), 1);
}

#[test]
fn unsupported_fault_is_a_bounded_error_not_a_private_scheduler() {
    let policy = policy();
    let manifest = manifest(&policy);
    let time = time_profile(&manifest);
    let initial = DeliveryState::empty(QUEUE_ID, manifest.policy_ref.clone(), SERVICE_GENERATION, CONSISTENCY_EPOCH);
    let result =
        run_delivery_simulation(&manifest, &policy, &time, initial, &[DeliverySimulationAction::FaultedRequest {
            fault: SimulationFaultKind::Reorder,
            request: enqueue_request(&manifest, '1'),
        }]);
    assert_eq!(
        result.expect_err("unsupported fault"),
        DeliverySimulationError::UnsupportedFault(SimulationFaultKind::Reorder)
    );
}
