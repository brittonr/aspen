use super::super::super::Scenario;
use super::super::super::matrix::assert_scenario_bound;

/// The observation a negative helper produces when its tamper is removed must
/// not verify: the held denial is what makes the scenario pass.
fn assert_control_fails(scenario: Scenario, evidence: &super::super::Evidence) {
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
fn world_replay_runs_the_production_replay_service() {
    assert_scenario_bound(Scenario::WorldReplayAdmitted);
}

// r[verify molten.world_replay.execution_boundary]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_replay_denies_revoked_authority_before_transitions() {
    assert_scenario_bound(Scenario::WorldReplayAuthorityDenied);
}

// r[verify molten.world_replay.execution_boundary]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_replay_denial_scenario_fails_when_authority_is_current() {
    let control = super::replay::authority_denied(false).expect("replay with current authority");
    assert_control_fails(Scenario::WorldReplayAuthorityDenied, &control);
}

// r[verify molten.world_snapshot.restore]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_restore_activates_with_current_admission() {
    assert_scenario_bound(Scenario::WorldRestoreCurrentAdmission);
}

// r[verify molten.world_snapshot.restore]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_restore_denies_stale_admission_before_activation() {
    assert_scenario_bound(Scenario::WorldRestoreStaleAdmissionDenied);
}

// r[verify molten.world_snapshot.restore]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_restore_denial_scenario_fails_with_current_admission() {
    let control = super::restore::stale_admission_denied(false).expect("restore with current admission");
    assert_control_fails(Scenario::WorldRestoreStaleAdmissionDenied, &control);
}

// r[verify molten.world_branch_authority.activation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_activation_rechecks_current_facts() {
    assert_scenario_bound(Scenario::WorldActivationCurrentFacts);
}

// r[verify molten.world_branch_authority.activation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_activation_denies_revocation_at_recheck() {
    assert_scenario_bound(Scenario::WorldActivationRevokedDenied);
}

// r[verify molten.world_branch_authority.activation]
// r[verify molten.testing.evidence_matrix_native_producers]
#[test]
fn world_activation_denial_scenario_fails_without_revocation() {
    let control = super::activation::revoked_denied(false).expect("activation without revocation");
    assert_control_fails(Scenario::WorldActivationRevokedDenied, &control);
}
