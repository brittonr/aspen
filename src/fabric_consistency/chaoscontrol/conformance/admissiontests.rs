include!("admission/tests/policy.rs");
include!("admission/tests/observations.rs");
include!("admission/tests/history.rs");
include!("admission/tests/fixtures.rs");
include!("admission/tests/retention.rs");
include!("admission/tests/negative.rs");

#[test]
fn instrumentation_import_has_canonical_packed_identity_not_execution_authority() {
    let (e, b) = fixture();
    let policy = super::expected_json(&serde_json::to_vec(&e).unwrap()).unwrap();
    let result = super::import_json(&policy, &serde_json::to_vec(&b).unwrap()).unwrap();
    assert_eq!(result.status, "bounded-instrumentation-valid");
    assert_eq!(result.evidence_class, "instrumentation");
    assert_eq!(result.receipt_ref, crate::preserves_rail::canonical_hash(&result.value).unwrap());
}

#[test]
fn candidate_cannot_change_independent_policy_or_guest_roles() {
    let (e, b) = fixture();
    for mutate in [
        |b: &mut super::Bundle| b.molten.dependency_closure_ref = id("other-closure"),
        |b: &mut super::Bundle| b.molten.observer_ref = id("other-observer"),
        |b: &mut super::Bundle| b.sources[0].executable_ref = id("other-build"),
        |b: &mut super::Bundle| b.producer_revision = "wrong-revision".into(),
    ] {
        let mut candidate = b.clone();
        mutate(&mut candidate);
        assert!(super::import(&e, &candidate).is_err());
    }
    let mut guest = b.clone();
    guest.evidence_class = chaoscontrol_smr::observation::guest::EvidenceClass::GuestExecution;
    assert_eq!(super::import(&e, &guest).unwrap_err(), super::ImportIssue::Claim);
    let mut policy = e.clone();
    policy.evidence_class = chaoscontrol_smr::observation::guest::EvidenceClass::GuestExecution;
    assert_eq!(super::expected_json(&serde_json::to_vec(&policy).unwrap()).unwrap_err(), super::ImportIssue::Claim);
    let mut snapshot = b.clone();
    snapshot.claim = "snapshot-backed-replay".into();
    assert_eq!(super::import(&e, &snapshot).unwrap_err(), super::ImportIssue::Claim);
}

#[test]
fn forged_pass_receipt_and_real_oracle_identity_do_not_establish_pass() {
    let (e, mut b) = fixture();
    // Every replica agrees on the same wrong state: producer history alone passes.
    for observation in &mut b.observations {
        if observation.apply.chain.command_index == 2 {
            observation.apply.chain.application_state_ref = id("coherently-wrong-state");
            let bytes = serde_json::to_vec(&observation.apply.chain).unwrap();
            observation.envelope.projection_ref = chaoscontrol_smr::observation::projection_ref(&bytes);
            observation.envelope.projection = Some(bytes);
        }
    }
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Pass, &[]);
    assert_eq!(b.history.verdict, chaoscontrol_smr::SafetyVerdict::Pass);
    assert_eq!(super::import(&e, &b).unwrap_err(), super::ImportIssue::Oracle);
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Fail, &["application-state-drift"]);
    assert_eq!(super::import(&e, &b).unwrap().status, "external-failure");
}

#[test]
fn observation_gap_and_incomplete_drains_are_not_consensus_safety_failures() {
    let (e, mut b) = fixture();
    b.drains[1].final_drain = false;
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Incomplete, &["cohort is not complete"]);
    assert_eq!(super::import(&e, &b).unwrap().status, "observer-incomplete");
    b.drains[1].final_drain = true;
    b.observations.remove(2); // omit the other participant's first apply, retain sequence 2
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Incomplete, &["cohort is not complete"]);
    assert!(b.history.violations.iter().all(|v| v.class == chaoscontrol_smr::SafetyViolationClass::LosslessGap));
    assert_eq!(super::import(&e, &b).unwrap().status, "observer-incomplete");
}

#[test]
fn real_safety_failure_and_not_evaluated_liveness_survive_import() {
    let (e, mut b) = fixture();
    b.observations[3].apply.chain.application_state_ref = id("divergent-state");
    let bytes = serde_json::to_vec(&b.observations[3].apply.chain).unwrap();
    b.observations[3].envelope.projection_ref = chaoscontrol_smr::observation::projection_ref(&bytes);
    b.observations[3].envelope.projection = Some(bytes);
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::Fail, &[
        "application-state-drift",
        "producer-safety-failure",
    ]);
    assert_eq!(super::import(&e, &b).unwrap().status, "external-failure");
    let (e, mut b) = fixture();
    b.stabilization = None;
    retain(&e, &mut b, chaoscontrol_smr::observation::OracleVerdict::NotEvaluated, &[
        "conditional liveness was not evaluated",
    ]);
    assert_eq!(super::import(&e, &b).unwrap().status, "not-evaluated");
}

#[test]
fn replay_requires_real_contained_runs_and_preserves_a_real_mismatch() {
    let (e, mut b) = fixture();
    attach_replay(&e, &mut b, false);
    assert_eq!(super::import(&e, &b).unwrap().evidence_class, "instrumentation");
    b.replay.as_mut().unwrap().repeat.observations[0].application_state_ref = id("unbound-repeat");
    assert_eq!(super::import(&e, &b).unwrap_err(), super::ImportIssue::Replay);
    let (e, mut b) = fixture();
    attach_replay(&e, &mut b, true);
    assert_eq!(super::import(&e, &b).unwrap().status, "semantic-replay-mismatch");
    b.replay.as_mut().unwrap().comparison = chaoscontrol_smr::ReplayComparison {
        accepted: true,
        first_mismatch: None,
    };
    assert_eq!(super::import(&e, &b).unwrap_err(), super::ImportIssue::Replay);
}

#[test]
fn bound_sidecars_reject_foreign_group_generation_and_missing_inline() {
    let (e, b) = fixture();
    for mutate in [
        |b: &mut super::Bundle| b.observations[0].apply.group_ref = id("other-group"),
        |b: &mut super::Bundle| b.observations[0].apply.lifecycle_generation += 1,
        |b: &mut super::Bundle| b.observations[0].envelope.projection = None,
        |b: &mut super::Bundle| b.observations[0].apply.command_bytes.push(0),
    ] {
        let mut candidate = b.clone();
        mutate(&mut candidate);
        assert_eq!(super::import(&e, &candidate).unwrap_err(), super::ImportIssue::Observation);
    }
    let mut swapped = b.clone();
    swapped.receipt.oracle_result_ref = id("arbitrary-shaped-oracle-result");
    assert_eq!(super::import(&e, &swapped).unwrap_err(), super::ImportIssue::Receipt);
}
