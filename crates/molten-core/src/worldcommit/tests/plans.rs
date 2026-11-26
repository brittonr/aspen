use super::*;

#[test]
fn complete_logical_closure_produces_deterministic_admission_last_restore() {
    // r[verify molten.world_commit.restore]
    let capture = plan_capture(&logical_request()).expect("capture plan");
    let commit_ref = identify_world_commit(b"logical-commit").expect("commit ref");
    let closure_request = complete_closure(commit_ref.clone(), capture.core.clone());
    let closure = validate_closure(&closure_request);
    let plan = plan_restore(&commit_ref, &capture.core, &closure).expect("restore plan");

    assert!(closure.complete);
    assert_eq!(plan.steps.first().map(|step| step.kind), Some(RestoreStepKind::VerifySchema));
    assert_eq!(plan.steps.last().map(|step| step.kind), Some(RestoreStepKind::ActivateRuntime));
    assert!(plan.current_admission_required);
    assert!(
        plan.replay.iter().any(|item| {
            item.root_kind == RootKind::AuthorityObservation && item.class == RootReplayClass::HistoricalEvidenceOnly
        }) == capture.core.roots.iter().any(|root| root.kind() == RootKind::AuthorityObservation)
    );
}

#[test]
fn opaque_profile_requires_exact_cohort_and_never_claims_logical_replay() {
    // r[verify molten.world_commit.verification]
    let request = CaptureRequest {
        version: WorldCommitVersion::V1,
        profile: opaque_profile(SnapshotProfileKind::Opaque),
        parents: Vec::new(),
        observations: observations(SnapshotProfileKind::Opaque),
        bounds: WorldCommitBounds::default(),
    };
    let capture = plan_capture(&request).expect("opaque capture plan");
    let opaque = capture
        .core
        .roots
        .iter()
        .find(|root| root.kind() == RootKind::OpaqueMachineSnapshot)
        .expect("opaque root");

    assert_eq!(replay_class(opaque.kind()), RootReplayClass::RestoreOpaqueState);
    assert!(!capture.core.roots.iter().any(|root| root.kind() == RootKind::DurableState));

    let mut missing_cohort = request;
    missing_cohort.profile.cohort_ref = None;
    assert!(
        plan_capture(&missing_cohort)
            .expect_err("missing cohort must fail")
            .contains(&CaptureIssue::OpaqueProfileMissingCohort)
    );
}

#[test]
fn bounds_fail_before_unbounded_capture_or_closure_work() {
    // r[verify molten.world_commit.verification]
    let mut request = logical_request();
    request.bounds.max_roots = TINY_BOUND;
    let issues = plan_capture(&request).expect_err("over-bound capture must fail");
    assert!(issues.iter().any(|issue| matches!(issue, CaptureIssue::BoundExceeded {
        field: "root-observations",
        ..
    })));

    request.bounds.max_roots = MAX_WORLD_COMMIT_ROOTS;
    request.bounds.max_closure_objects = 0;
    let issues = plan_capture(&request).expect_err("zero closure bound must fail");
    assert!(issues.contains(&CaptureIssue::ZeroBound("max-closure-objects")));
}

#[test]
fn evidence_and_authority_observations_do_not_enter_current_admission() {
    // r[verify molten.world_commit.detached_evidence]
    let mut request = logical_request();
    request.observations.push(RootObservation {
        root: root(RootKind::AuthorityObservation, "historical-authority"),
        source_kind: RootKind::AuthorityObservation,
        schema_validated: true,
        stability: ObservationStability::Immutable,
        durable: true,
        inventory_complete: true,
    });
    let capture = plan_capture(&request).expect("capture with authority observation");
    let commit_ref = identify_world_commit(b"authority-observation").expect("commit ref");
    let closure_request = complete_closure(commit_ref.clone(), capture.core.clone());
    let closure = validate_closure(&closure_request);
    let restore = plan_restore(&commit_ref, &capture.core, &closure).expect("restore plan");

    assert!(restore.current_admission_required);
    assert!(restore.replay.contains(&RootReplayClassification {
        root_kind: RootKind::AuthorityObservation,
        class: RootReplayClass::HistoricalEvidenceOnly,
    }));
}
