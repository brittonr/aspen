//! Shell boundary of the fast-path model: typed core rejections become classified failures.
//! The model's own unit and Hegel tests live with the code in `crates/molten-fastpath-core`.

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn every_core_rejection_family_maps_to_a_malformed_observation_with_its_typed_reason() {
    let cases = [
        (
            crate::fabric_consistency::fastpath::failure(
                crate::fabric_consistency::fastpath::RecoveryIssue::ViewMismatch,
            ),
            "view-change-recovery:view-mismatch",
        ),
        (
            crate::fabric_consistency::fastpath::failure(
                crate::fabric_consistency::fastpath::LedgerIssue::ConflictingSessionRetry,
            ),
            "application-ledger:conflicting-session-retry",
        ),
        (
            crate::fabric_consistency::fastpath::failure(
                crate::fabric_consistency::fastpath::ProfileIssue::ProductionSelectionDenied,
            ),
            "model-profile:production-selection-denied",
        ),
        (
            crate::fabric_consistency::fastpath::failure(
                crate::fabric_consistency::fastpath::ConflictContractIssue::UnsupportedVersion,
            ),
            "conflict-contract:unsupported-version",
        ),
        (
            crate::fabric_consistency::fastpath::failure(
                crate::fabric_consistency::fastpath::FaultScenarioIssue::OverStepBound,
            ),
            "fault-scenario:over-step-bound",
        ),
    ];
    for (failure, reason) in cases {
        assert_eq!(failure.class(), Some(crate::error::FailureClass::MalformedObservation), "{failure}");
        assert_eq!(failure.to_string(), format!("malformed observation: fast-path model rejected input: {reason}"));
    }
}

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn a_rejected_recovery_transition_surfaces_through_the_shell_as_a_classified_failure() {
    let normal = crate::fabric_consistency::fastpath::RecoveryState {
        acceleration_view: 1,
        base_view: 1,
        last_normal_view: 1,
        phase: crate::fabric_consistency::fastpath::RecoveryPhase::Normal,
        accepted_commands: std::collections::BTreeSet::new(),
        recovery_set: std::collections::BTreeSet::new(),
        marker_ref: None,
    };
    let resumed = crate::fabric_consistency::fastpath::transition_recovery(
        &normal,
        &crate::fabric_consistency::fastpath::RecoveryAction::ResumeNormalView {
            new_acceleration_view: 2,
        },
    )
    .map_err(crate::fabric_consistency::fastpath::failure);
    let failure = resumed.expect_err("resume before any marker is rejected");
    assert_eq!(failure.class(), Some(crate::error::FailureClass::MalformedObservation));
    assert!(failure.to_string().ends_with("view-change-recovery:resume-before-marker"), "{failure}");

    let paused = crate::fabric_consistency::fastpath::transition_recovery(
        &normal,
        &crate::fabric_consistency::fastpath::RecoveryAction::BeginViewChange { new_base_view: 2 },
    )
    .map_err(crate::fabric_consistency::fastpath::failure)
    .expect("a forward view change is admitted");
    assert_eq!(paused.phase, crate::fabric_consistency::fastpath::RecoveryPhase::Paused);
}
