use super::*;

// r[verify molten.outside_in.observation]
#[test]
fn a_matching_history_agrees_and_carries_the_non_claims() {
    let result = judge(
        &declaration(FaultProfile::NoFault, &[FIRST_REQUEST]),
        agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE),
    );
    assert_eq!(result.outcome, ScenarioOutcome::Agreement);
    assert!(result.is_pass());
    assert_eq!(result.non_claims, [
        "no-production-admission",
        "no-simulation-replacement",
        "no-performance-claim"
    ]);
}

// r[verify molten.outside_in.observation]
#[test]
fn the_first_mismatch_is_named_in_a_fixed_order() {
    let declared = declaration(FaultProfile::NoFault, &[FIRST_REQUEST]);
    let mut missing = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    missing.checkpoints = vec![final_read(history(Vec::new(), Vec::new()))];
    assert_eq!(
        judge(&declared, missing).outcome,
        ScenarioOutcome::Failure(HistoryMismatch::ExpectedNotProcessed(FIRST_REQUEST.to_string()))
    );

    let mut denied = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    denied.checkpoints = vec![final_read(history(Vec::new(), vec![ObservedProcessed {
        request_ref: FIRST_REQUEST.to_string(),
        is_passed: false,
    }]))];
    assert_eq!(
        judge(&declared, denied).outcome,
        ScenarioOutcome::Failure(HistoryMismatch::ProcessingDenied(FIRST_REQUEST.to_string()))
    );

    let mut extra = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    extra.checkpoints = vec![final_read(history(Vec::new(), vec![
        processed(FIRST_REQUEST),
        processed(SECOND_REQUEST),
    ]))];
    assert_eq!(
        judge(&declared, extra).outcome,
        ScenarioOutcome::Failure(HistoryMismatch::UnexpectedProcessed(SECOND_REQUEST.to_string()))
    );

    let mut pending = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    let mut pending_history = history(Vec::new(), vec![processed(FIRST_REQUEST)]);
    pending_history.pending_requests = vec![SECOND_REQUEST.to_string()];
    pending.checkpoints = vec![final_read(pending_history)];
    assert_eq!(
        judge(&declared, pending).outcome,
        ScenarioOutcome::Failure(HistoryMismatch::PendingAfterStabilization(SECOND_REQUEST.to_string()))
    );
}

// r[verify molten.outside_in.observation]
#[test]
fn two_accepted_envelopes_for_one_request_fail_even_when_processing_looks_single() {
    let mut doubled = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    doubled.checkpoints = vec![final_read(history(
        vec![
            delivered(FIRST_ENVELOPE, FIRST_REQUEST),
            delivered(SECOND_ENVELOPE, FIRST_REQUEST),
        ],
        vec![processed(FIRST_REQUEST)],
    ))];
    assert_eq!(
        judge(&declaration(FaultProfile::NoFault, &[FIRST_REQUEST]), doubled).outcome,
        ScenarioOutcome::Failure(HistoryMismatch::DuplicateAcceptedDelivery(FIRST_REQUEST.to_string()))
    );
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn kill_and_restart_needs_a_post_restart_read_and_rejects_a_reset() {
    let declared = declaration(FaultProfile::KillRestart, &[FIRST_REQUEST, SECOND_REQUEST]);
    let committed = history(vec![delivered(FIRST_ENVELOPE, FIRST_REQUEST)], vec![processed(FIRST_REQUEST)]);
    let both = history(
        vec![
            delivered(FIRST_ENVELOPE, FIRST_REQUEST),
            delivered(SECOND_ENVELOPE, SECOND_REQUEST),
        ],
        vec![processed(FIRST_REQUEST), processed(SECOND_REQUEST)],
    );
    let restarted = ObservedHistory {
        attempts: vec![acknowledged(FIRST_REQUEST), acknowledged(SECOND_REQUEST)],
        checkpoints: vec![
            HistoryCheckpoint {
                kind: CheckpointKind::AfterFault,
                receiver: Some(committed),
            },
            final_read(both.clone()),
        ],
        listener_endpoints: vec![ENDPOINT.to_string(), ENDPOINT.to_string(), ENDPOINT.to_string()],
    };
    assert_eq!(judge(&declared, restarted.clone()).outcome, ScenarioOutcome::Agreement);

    let mut reset = restarted.clone();
    reset.checkpoints[0].receiver = Some(history(Vec::new(), vec![processed(FIRST_REQUEST)]));
    reset.checkpoints[1] =
        final_read(history(vec![delivered(SECOND_ENVELOPE, SECOND_REQUEST)], vec![processed(SECOND_REQUEST)]));
    assert_eq!(
        judge(&declared, reset).outcome,
        ScenarioOutcome::Failure(HistoryMismatch::StateReset(FIRST_REQUEST.to_string()))
    );

    let mut renamed = restarted.clone();
    renamed.listener_endpoints[2] = OTHER_ENDPOINT.to_string();
    assert_eq!(
        judge(&declared, renamed).outcome,
        ScenarioOutcome::Failure(HistoryMismatch::ListenerIdentityChanged)
    );

    let mut unread = restarted.clone();
    unread.checkpoints[0].receiver = None;
    assert_eq!(judge(&declared, unread).outcome, ScenarioOutcome::Incomplete(MissingObservation::AfterFaultHistory));

    let mut never_restarted = restarted;
    never_restarted.listener_endpoints.truncate(1);
    assert_eq!(
        judge(&declared, never_restarted).outcome,
        ScenarioOutcome::Incomplete(MissingObservation::ListenerRuns)
    );
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn a_lost_response_stays_unresolved_and_an_invented_outcome_fails() {
    let declared = declaration(FaultProfile::LostResponse, &[FIRST_REQUEST]);
    let mut lost = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    lost.attempts = vec![
        attempt(FIRST_REQUEST, ResponseFate::Lost, ClientOutcome::Unresolved),
        acknowledged(FIRST_REQUEST),
    ];
    assert_eq!(
        judge(&declared, lost.clone()).outcome,
        ScenarioOutcome::PermittedDivergence(vec![
            DivergenceReason::LostResponseApplied,
            DivergenceReason::DuplicateSuppressed
        ])
    );

    for invented in [ClientOutcome::Acknowledged, ClientOutcome::Failed] {
        let mut reported = lost.clone();
        reported.attempts[0].outcome = invented;
        assert_eq!(
            judge(&declared, reported).outcome,
            ScenarioOutcome::Failure(HistoryMismatch::InventedClientOutcome(FIRST_REQUEST.to_string()))
        );
    }
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn a_failed_attempt_during_the_fault_and_a_later_success_is_a_retry_not_a_duplicate() {
    let mut retried = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    retried.attempts = vec![
        attempt(FIRST_REQUEST, ResponseFate::Observed, ClientOutcome::Failed),
        acknowledged(FIRST_REQUEST),
    ];
    assert_eq!(
        judge(&declaration(FaultProfile::PartitionHeal, &[FIRST_REQUEST]), retried).outcome,
        ScenarioOutcome::PermittedDivergence(vec![DivergenceReason::RetriedAfterFault])
    );
}

// r[verify molten.outside_in.validation]
#[test]
fn incomplete_and_unexecuted_scenarios_are_typed_and_never_pass() {
    let declared = declaration(FaultProfile::NoFault, &[FIRST_REQUEST]);
    let mut unread = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    unread.checkpoints[0].receiver = None;
    assert_eq!(judge(&declared, unread).outcome, ScenarioOutcome::Incomplete(MissingObservation::FinalHistory));
    let mut silent = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    silent.attempts.clear();
    assert_eq!(judge(&declared, silent).outcome, ScenarioOutcome::Incomplete(MissingObservation::ClientAttempts));
    let mut flooded = agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE);
    flooded.attempts = vec![acknowledged(FIRST_REQUEST); 5];
    assert_eq!(judge(&declared, flooded).outcome, ScenarioOutcome::BudgetExhausted(BudgetKind::ClientOperations));

    for (execution, expected) in [
        (
            ScenarioExecution::StartupFailed {
                process: "listener".to_string(),
            },
            ScenarioOutcome::StartupFailed("listener".to_string()),
        ),
        (
            ScenarioExecution::BudgetExhausted(BudgetKind::WallClock),
            ScenarioOutcome::BudgetExhausted(BudgetKind::WallClock),
        ),
        (
            ScenarioExecution::Unsupported {
                behavior: "pause-signal".to_string(),
            },
            ScenarioOutcome::Unsupported("pause-signal".to_string()),
        ),
    ] {
        let result = judge_scenario(&declared, &execution, TeardownStatus::Complete);
        assert_eq!(result.outcome, expected);
        assert!(!result.is_pass());
    }
}

// r[verify molten.outside_in.real_stack]
#[test]
fn an_agreeing_history_with_a_retained_scratch_path_does_not_pass() {
    let result = judge_scenario(
        &declaration(FaultProfile::NoFault, &[FIRST_REQUEST]),
        &ScenarioExecution::Completed(agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE)),
        TeardownStatus::ScratchPathRetained {
            path_label: "receiver-state".to_string(),
        },
    );
    assert_eq!(result.outcome, ScenarioOutcome::Agreement);
    assert!(!result.is_pass());
}
