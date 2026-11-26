/// The observation a negative helper produces when its tamper is removed must
/// not verify: the held denial is what makes the scenario pass.
fn assert_control_fails(scenario: super::super::super::Scenario, evidence: &super::super::Evidence) {
    let observation = super::super::observation(scenario, evidence).expect("control observation");
    assert!(
        super::super::verify_observation(scenario, &observation).is_err(),
        "{} passed without its denial",
        scenario.id()
    );
}

// Each scenario runs once and must reproduce the runtime-artifact ref checked
// into `tests/evidence-matrix.ncl` by earlier independent executions.

// r[verify molten.world_replay.execution_boundary]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn replay_runs_the_production_replay_service() {
    super::super::super::matrix::assert_scenario_bound(super::super::super::Scenario::WorldReplayAdmitted);
}

// r[verify molten.world_replay.execution_boundary]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn replay_denies_revoked_authority_before_transitions() {
    super::super::super::matrix::assert_scenario_bound(super::super::super::Scenario::WorldReplayAuthorityDenied);
}

// r[verify molten.world_replay.execution_boundary]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn replay_denial_scenario_fails_when_authority_is_current() {
    let control = super::replay::authority_denied(false).expect("replay with current authority");
    assert_control_fails(super::super::super::Scenario::WorldReplayAuthorityDenied, &control);
}

// r[verify molten.world_snapshot.restore]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn restore_activates_with_current_admission() {
    super::super::super::matrix::assert_scenario_bound(super::super::super::Scenario::WorldRestoreCurrentAdmission);
}

// r[verify molten.world_snapshot.restore]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn restore_denies_stale_admission_before_activation() {
    super::super::super::matrix::assert_scenario_bound(super::super::super::Scenario::WorldRestoreStaleAdmissionDenied);
}

// r[verify molten.world_snapshot.restore]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn restore_denial_scenario_fails_with_current_admission() {
    let control = super::restore::stale_admission_denied(false).expect("restore with current admission");
    assert_control_fails(super::super::super::Scenario::WorldRestoreStaleAdmissionDenied, &control);
}

// r[verify molten.world_branch_authority.activation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn activation_rechecks_current_facts() {
    super::super::super::matrix::assert_scenario_bound(super::super::super::Scenario::WorldActivationCurrentFacts);
}

// r[verify molten.world_branch_authority.activation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn activation_denies_revocation_at_recheck() {
    super::super::super::matrix::assert_scenario_bound(super::super::super::Scenario::WorldActivationRevokedDenied);
}

// r[verify molten.world_branch_authority.activation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn activation_denial_scenario_fails_without_revocation() {
    let control = super::activation::revoked_denied(false).expect("activation without revocation");
    assert_control_fails(super::super::super::Scenario::WorldActivationRevokedDenied, &control);
}
