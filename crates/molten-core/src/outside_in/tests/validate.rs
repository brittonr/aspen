use super::*;

// r[verify molten.outside_in.real_stack]
#[test]
fn every_profile_admits_its_declared_boundary_and_bounded_budgets() {
    for profile in [
        FaultProfile::NoFault,
        FaultProfile::KillRestart,
        FaultProfile::PauseResume,
        FaultProfile::PartitionHeal,
        FaultProfile::LostResponse,
    ] {
        assert_eq!(validate_scenario(&declaration(profile, &[FIRST_REQUEST])), Vec::new(), "{}", profile.as_str());
    }
}

// r[verify molten.outside_in.validation]
#[test]
fn the_consensus_path_is_excluded_before_any_process_starts() {
    let mut consensus = declaration(FaultProfile::KillRestart, &[FIRST_REQUEST]);
    consensus.path = ScenarioPath::Consensus;
    assert_eq!(validate_scenario(&consensus), vec![ScenarioIssue::ConsensusPathExcluded]);
    let result = judge(&consensus, agreeing_history(FIRST_REQUEST, FIRST_ENVELOPE));
    assert_eq!(result.outcome, ScenarioOutcome::DeclarationRejected(vec![ScenarioIssue::ConsensusPathExcluded]));
    assert!(!result.is_pass(), "a rejected declaration never passes, whatever was observed");
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn a_fault_must_apply_at_its_profile_boundary() {
    let mut mismatched = declaration(FaultProfile::KillRestart, &[FIRST_REQUEST]);
    mismatched.boundary = FaultBoundary::ClientResponse;
    assert_eq!(validate_scenario(&mismatched), vec![ScenarioIssue::ProfileBoundaryMismatch]);
}

// r[verify molten.outside_in.real_stack]
#[test]
fn budgets_must_be_nonzero_and_under_their_caps() {
    let mut zero = declaration(FaultProfile::NoFault, &[FIRST_REQUEST]);
    zero.budgets.max_processes = 0;
    zero.budgets.max_retained_output_bytes = MAX_RETAINED_OUTPUT_BYTES + 1;
    assert_eq!(validate_scenario(&zero), vec![
        ScenarioIssue::BudgetOutOfBounds(BudgetKind::Processes),
        ScenarioIssue::BudgetOutOfBounds(BudgetKind::RetainedOutput),
    ]);

    let mut at_caps = declaration(FaultProfile::NoFault, &[FIRST_REQUEST]);
    at_caps.budgets = ScenarioBudgets {
        max_processes: MAX_SCENARIO_PROCESSES,
        wall_clock_ms: MAX_SCENARIO_WALL_CLOCK_MS,
        max_retained_output_bytes: MAX_RETAINED_OUTPUT_BYTES,
        max_client_operations: MAX_CLIENT_OPERATIONS,
    };
    assert_eq!(validate_scenario(&at_caps), Vec::new(), "the caps themselves are admitted");
}

// r[verify molten.outside_in.fault_profiles]
#[test]
fn held_faults_need_a_bounded_hold_and_unheld_faults_hold_nothing() {
    let mut unbounded = declaration(FaultProfile::PartitionHeal, &[FIRST_REQUEST]);
    unbounded.fault_hold_ms = WALL_CLOCK_MS;
    assert!(validate_scenario(&unbounded).contains(&ScenarioIssue::FaultHoldOutOfBounds));

    let mut unheld = declaration(FaultProfile::PauseResume, &[FIRST_REQUEST]);
    unheld.fault_hold_ms = 0;
    assert_eq!(validate_scenario(&unheld), vec![ScenarioIssue::FaultHoldOutOfBounds]);

    let mut held_kill = declaration(FaultProfile::KillRestart, &[FIRST_REQUEST]);
    held_kill.fault_hold_ms = HOLD_MS;
    assert_eq!(validate_scenario(&held_kill), vec![ScenarioIssue::FaultHoldOutOfBounds]);

    let mut unsettled = declaration(FaultProfile::NoFault, &[FIRST_REQUEST]);
    unsettled.stabilization_ms = WALL_CLOCK_MS;
    assert_eq!(validate_scenario(&unsettled), vec![ScenarioIssue::StabilizationOutOfBounds]);
}

// r[verify molten.outside_in.validation]
#[test]
fn the_expected_history_is_nonempty_unique_and_within_the_operation_budget() {
    assert_eq!(validate_scenario(&declaration(FaultProfile::NoFault, &[])), vec![
        ScenarioIssue::ExpectedHistoryEmpty
    ]);
    assert_eq!(validate_scenario(&declaration(FaultProfile::NoFault, &[FIRST_REQUEST, FIRST_REQUEST])), vec![
        ScenarioIssue::DuplicateExpectedRequest(FIRST_REQUEST.to_string())
    ]);
    let mut over = declaration(FaultProfile::NoFault, &[FIRST_REQUEST, SECOND_REQUEST]);
    over.budgets.max_client_operations = 1;
    assert_eq!(validate_scenario(&over), vec![ScenarioIssue::ExpectedHistoryOverBudget]);
    let mut blank = declaration(FaultProfile::NoFault, &[FIRST_REQUEST]);
    blank.scenario_id = " ".to_string();
    assert_eq!(validate_scenario(&blank), vec![ScenarioIssue::EmptyScenarioId]);
}
