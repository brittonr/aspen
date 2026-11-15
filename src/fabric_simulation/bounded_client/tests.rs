use super::*;

mod admission;

fn reference(label: &str) -> String {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("client-oracle-fixture-v1", vec![
        crate::preserves_rail::string(label),
    ]))
    .expect("fixture ref")
}

// These modeled checker fixtures never claim native or guest execution.
fn fixture() -> (Input, History) {
    let input = Input::native(4, SemanticControl::Healthy).expect("input");
    let participants = input
        .participants
        .iter()
        .map(|participant| Participant {
            participant: participant.clone(),
            receipt_ref: reference(participant),
            request_ref: input.submit_request_ref.clone(),
            is_receipt_bound: true,
            is_ready: true,
            has_membership: true,
            acknowledged: true,
            application_count: 1,
            snapshot_restored: false,
            recovered_acknowledgement: true,
            persistence: Some(molten_core::fabric_durability::PersistenceObservation::AppliedProcessVisible),
            requested_durability: molten_core::fabric_durability::DurabilityLevel::ProcessLoss,
            health_before: molten_core::fabric_durability::AdapterHealth::Serving,
            recovery: RecoverySource::Local(reference("local-reopen")),
            start_term: 1,
            end_term: Some(2),
            observed_ticks: input.recovery_ticks,
            is_fault_off: true,
        })
        .collect();
    let history = History {
        input_ref: input_ref(&input).expect("input ref"),
        executed_requests: input.steps.iter().map(|step| step.request_ref.clone()).collect(),
        fault: FaultApplication::Applied,
        participants,
        deployed_control: SemanticControl::Healthy,
        is_control_reached: false,
    };
    (input, history)
}

fn change_case(scenario: &str, history: &mut History) {
    match scenario {
        "healthy-local" => {}
        "peer-repair" => {
            history.participants[0].recovery = RecoverySource::Peer {
                participant: "node-b".to_string(),
                receipt_ref: reference("peer-repair"),
            }
        }
        "global-absence" => {
            for participant in &mut history.participants {
                participant.recovery = RecoverySource::Absent(reference("empty-inventory"));
                participant.recovered_acknowledgement = false;
            }
        }
        "no-op" => {
            history.participants.clear();
        }
        "missing-participant" => {
            history.participants.pop();
        }
        "unreached-fault" => history.fault = FaultApplication::Requested,
        "short-horizon" => history.participants[2].observed_ticks -= 1,
        "stalled-participant" => history.participants[2].end_term = Some(1),
        "missing-end" => history.participants[2].end_term = None,
        "missing-readiness" => history.participants[2].is_ready = false,
        "unavailable-recovery" => history.participants[0].recovery = RecoverySource::Unavailable,
        "mismatched-receipt" => history.participants[0].is_receipt_bound = false,
        "fabricated-receipt" => history.participants[0].receipt_ref = "not-a-receipt".to_string(),
        "unsynced-machine-loss" => {
            history.participants[0].requested_durability = molten_core::fabric_durability::DurabilityLevel::MachineLoss
        }
        "observed-sync" => {
            history.participants[0].requested_durability = molten_core::fabric_durability::DurabilityLevel::MachineLoss;
            history.participants[0].persistence =
                Some(molten_core::fabric_durability::PersistenceObservation::SyncObserved);
        }
        "unknown-storage" => {
            history.participants[0].persistence =
                Some(molten_core::fabric_durability::PersistenceObservation::OutcomeUnknown)
        }
        "corrupt-storage" => {
            history.participants[0].persistence =
                Some(molten_core::fabric_durability::PersistenceObservation::CorruptOrInconsistent)
        }
        "retry-does-not-reconcile" => {
            history.participants[0].health_before = molten_core::fabric_durability::AdapterHealth::Quarantined(
                molten_core::fabric_durability::QuarantineReason::OutcomeUnknown,
            )
        }
        "unsupported-fault" => history.fault = FaultApplication::Unsupported,
        other => panic!("unknown fixture scenario {other}"),
    }
}

// r[verify molten.chaoscontrol_client.observation]
// r[verify molten.chaoscontrol_client.controls]
#[test]
fn executable_outcome_bank_preserves_negative_and_incomplete_classifications() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/chaoscontrol-client/oracle-controls.json"))
            .expect("control bank");
    for case in cases.as_array().expect("case list") {
        let (input, mut history) = fixture();
        change_case(case["scenario"].as_str().expect("scenario"), &mut history);
        let result = check_history(&input, &history, true);
        let observed = match result.verdict {
            Verdict::Pass => "pass",
            Verdict::Fail { .. } => "fail",
            Verdict::NotEvaluated { .. } => "not-evaluated",
            Verdict::Incomplete { .. } => "incomplete",
            Verdict::Invalid { .. } => "invalid",
            Verdict::Unsupported => "unsupported",
        };
        assert_eq!(observed, case["expected"].as_str().expect("expected classification"), "{}", case["scenario"]);
    }
}

// r[verify molten.chaoscontrol_client.controls]
#[test]
fn control_attribution_requires_reached_divergence_and_intended_property() {
    let (mut input, mut history) = fixture();
    input.control = SemanticControl::DuplicateApplication;
    history.input_ref = input_ref(&input).expect("binding");
    history.deployed_control = input.control;
    let check = check_history(&input, &history, true);
    assert_eq!(classify_semantic_control(&history, &check), SemanticControlVerdict::NotReached);
    history.is_control_reached = true;
    assert_eq!(classify_semantic_control(&history, &check), SemanticControlVerdict::ReachedSurvived);
    history.participants[0].application_count = 2;
    let detected = check_history(&input, &history, true);
    assert_eq!(classify_semantic_control(&history, &detected), SemanticControlVerdict::ReachedDetected {
        property: APPLY_ONCE_PROPERTY.to_string()
    });
    let suppressed = check_history(&input, &history, false);
    assert_eq!(classify_semantic_control(&history, &suppressed), SemanticControlVerdict::Invalid);
    history.participants[0].application_count = 1;
    history.participants[2].end_term = Some(1);
    let unrelated = check_history(&input, &history, true);
    assert_eq!(classify_semantic_control(&history, &unrelated), SemanticControlVerdict::Incomplete);
}

// r[verify molten.chaoscontrol_client.real_execution]
#[test]
fn noop_workload_executor_cannot_substitute_for_observed_execution() {
    struct Unobserved;
    impl crate::fabric_simulation::WorkloadExecutor for Unobserved {
        fn execute_step(
            &mut self,
            _: &crate::fabric_simulation::LiveWorkloadStep,
        ) -> Option<crate::fabric_simulation::OutcomeClass> {
            Some(crate::fabric_simulation::OutcomeClass::Delivered)
        }
    }
    let (input, mut history) = fixture();
    let outcomes =
        crate::fabric_simulation::execute_workload(&mut Unobserved, &input.differential_workload().expect("workload"));
    assert!(outcomes.iter().all(|outcome| *outcome == Some(crate::fabric_simulation::OutcomeClass::Delivered)));
    history.participants.clear();
    assert!(matches!(check_history(&input, &history, true).verdict, Verdict::Incomplete { .. }));
}

// r[verify molten.chaoscontrol_client.observation]
// r[verify molten.chaoscontrol_client.controls]
#[test]
fn receipt_validation_and_apply_once_keep_observation_order() {
    let (input, mut history) = fixture();
    history.participants[0].is_receipt_bound = false;
    history.participants[0].application_count = 2;
    assert!(matches!(check_history(&input, &history, true).verdict, Verdict::Invalid { .. }));

    let (input, mut history) = fixture();
    history.participants[0].application_count = 2;
    history.participants[1].is_receipt_bound = false;
    assert!(matches!(check_history(&input, &history, true).verdict,
        Verdict::Fail { property, participant }
        if property == APPLY_ONCE_PROPERTY && participant == input.participants[0]));
}

// r[verify molten.chaoscontrol_client.bounded_input]
// r[verify molten.chaoscontrol_client.controls]
#[test]
fn prefix_divergence_is_detected_without_qualifying_missing_recovery() {
    let (input, mut history) = fixture();
    let prefix = input.valid_prefix(3).expect("valid prefix");
    history.input_ref = input_ref(&prefix).expect("prefix binding");
    history.executed_requests.truncate(3);
    history.fault = FaultApplication::Requested;
    history.participants[0].application_count = 2;
    assert!(matches!(check_history(&prefix, &history, true).verdict,
        Verdict::Fail { property, .. } if property == APPLY_ONCE_PROPERTY));
    assert!(matches!(check_history(&prefix, &history, false).verdict, Verdict::Invalid { .. }));
    history.participants[0].application_count = 1;
    assert!(matches!(check_history(&prefix, &history, true).verdict, Verdict::Incomplete { .. }));
}
