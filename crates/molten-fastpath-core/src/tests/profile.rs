//! Profile admission, derived quorums, the conflict contract, and typed rejections.

// r[verify molten.consensus.fast_path_model.profile]
// r[verify molten.consensus.fast_path_model.nonclaims]
#[test]
fn named_profiles_derive_expected_quorums_and_deny_production() {
    let three = crate::envelope::validate_profile(&super::support::profile(super::support::THREE_NODES));
    let five = crate::envelope::validate_profile(&super::support::profile(super::support::FIVE_NODES));
    assert!(three.is_admitted());
    assert!(five.is_admitted());
    assert_eq!(three.quorums.map(|row| row.superquorum), Some(super::support::THREE_NODE_SUPERQUORUM));
    assert_eq!(five.quorums.map(|row| row.superquorum), Some(super::support::FIVE_NODE_SUPERQUORUM));
    assert_eq!(three.quorums.map(|row| row.fast_path_requires_every_replica), Some(true));
    assert_eq!(five.quorums.map(|row| row.fast_path_requires_every_replica), Some(false));

    let mut production = super::support::profile(super::support::THREE_NODES);
    production.selection = crate::envelope::SelectionMode::Production;
    assert!(
        crate::envelope::validate_profile(&production)
            .issues
            .contains(&crate::envelope::ProfileIssue::ProductionSelectionDenied)
    );
}

#[test]
fn quorums_below_three_replicas_are_impossible() {
    assert_eq!(crate::envelope::derive_quorums(2), None);
    let mut two = super::support::profile(super::support::THREE_NODES);
    two.node_count = 2;
    let admission = crate::envelope::validate_profile(&two);
    assert!(!admission.is_admitted());
    assert!(admission.issues.contains(&crate::envelope::ProfileIssue::ImpossibleQuorum));
}

// r[verify molten.consensus.fast_path_model.base_prerequisites]
#[test]
fn malformed_reference_bounds_and_ordering_fail_closed() {
    let mut malformed = super::support::profile(super::support::THREE_NODES);
    malformed.source.artifact_revision = "floating-main".to_owned();
    malformed.max_steps = 0;
    malformed.base_ordering.execution_order_preserved = false;
    let issues = crate::envelope::validate_profile(&malformed).issues;
    assert!(issues.contains(&crate::envelope::ProfileIssue::UnknownReference));
    assert!(issues.contains(&crate::envelope::ProfileIssue::BoundExceeded("steps")));
    assert!(issues.contains(&crate::envelope::ProfileIssue::BaseExecutionReorders));
}

// r[verify molten.consensus.fast_path_model.conflict_contract]
#[test]
fn conflict_contract_is_conservative_for_unknown_semantics() {
    let left = crate::commute::CommandShape::Put {
        key: "left".to_owned(),
        value_ref: "value-a".to_owned(),
    };
    let right = crate::commute::CommandShape::Get {
        key: "right".to_owned(),
    };
    assert_eq!(crate::commute::classify_conflict(&left, &right), crate::commute::ConflictDecision::Independent);
    assert_eq!(
        crate::commute::classify_conflict(&left, &crate::commute::CommandShape::Get { key: "left".to_owned() }),
        crate::commute::ConflictDecision::Conflict
    );
    assert_eq!(
        crate::commute::classify_conflict(&left, &crate::commute::CommandShape::Range {
            start: "a".to_owned(),
            end: "z".to_owned(),
        }),
        crate::commute::ConflictDecision::ConservativeFallback
    );
}

// r[verify molten.consensus.fast_path_model.conflict_contract]
#[test]
fn conflict_contract_rejects_empty_refs_and_unknown_versions() {
    let contract = crate::commute::ConflictContract {
        contract_ref: "blake3:contract".to_owned(),
        version: "v1".to_owned(),
        command_schema_ref: "blake3:command".to_owned(),
        state_schema_ref: "blake3:state".to_owned(),
    };
    assert!(crate::commute::validate_conflict_contract(&contract).is_empty());
    let mut drifted = contract;
    drifted.version = "v2".to_owned();
    drifted.state_schema_ref = " ".to_owned();
    assert_eq!(crate::commute::validate_conflict_contract(&drifted), vec![
        crate::commute::ConflictContractIssue::EmptyField("state schema ref"),
        crate::commute::ConflictContractIssue::UnsupportedVersion,
    ]);
}

// r[verify molten.consensus.no_std_raft_and_reliability_kernels]
#[test]
fn typed_rejections_name_their_family_and_keep_the_issue_as_source() {
    let rejection = crate::Rejection::from(crate::succession::RecoveryIssue::ResumeBeforeMarker);
    assert_eq!(rejection.to_string(), "view-change-recovery:resume-before-marker");
    let source = core::error::Error::source(&rejection).expect("typed issue source");
    assert_eq!(source.to_string(), "resume-before-marker");
    assert_eq!(
        crate::Rejection::from(crate::envelope::ProfileIssue::BoundExceeded("steps")).to_string(),
        "model-profile:bound-exceeded:steps"
    );
    assert_eq!(
        crate::Rejection::from(crate::quorate::LedgerIssue::ConflictingSessionRetry).to_string(),
        "application-ledger:conflicting-session-retry"
    );
}

/// `3 * node_count` exceeds `u32::MAX` from 1,431,655,766 replicas on; the superquorum is still
/// derived because the arithmetic runs in `u64`.
// r[verify molten.consensus.fast_path_model.stable_view]
#[test]
fn quorums_derive_past_the_u32_superquorum_product_boundary() {
    let last_in_range = crate::envelope::derive_quorums(1_431_655_765).expect("3n == u32::MAX");
    assert_eq!(last_in_range.superquorum, 1_073_741_824);
    let first_past = crate::envelope::derive_quorums(1_431_655_766).expect("3n > u32::MAX");
    assert_eq!(first_past.majority, 715_827_884);
    assert_eq!(first_past.superquorum, 1_073_741_825);
    assert_eq!(first_past.tolerated_failures, 715_827_882);
    assert!(!first_past.fast_path_requires_every_replica);
    let widest = crate::envelope::derive_quorums(u32::MAX).expect("largest node count");
    assert_eq!(widest.superquorum, 3_221_225_472);
    assert_eq!(widest.majority, 2_147_483_648);
}
