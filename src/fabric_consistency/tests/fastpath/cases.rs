use super::*;

// r[verify molten.consensus.fast_path_model.profile]
// r[verify molten.consensus.fast_path_model.nonclaims]
#[test]
fn named_profiles_derive_expected_quorums_and_deny_production() {
    let three = validate_profile(&profile(THREE_NODES));
    let five = validate_profile(&profile(FIVE_NODES));
    assert!(three.is_admitted());
    assert!(five.is_admitted());
    assert_eq!(three.quorums.map(|row| row.superquorum), Some(THREE_NODE_SUPERQUORUM));
    assert_eq!(five.quorums.map(|row| row.superquorum), Some(FIVE_NODE_SUPERQUORUM));
    assert_eq!(three.quorums.map(|row| row.fast_path_requires_every_replica), Some(true));

    let mut production = profile(THREE_NODES);
    production.selection = SelectionMode::Production;
    assert!(validate_profile(&production).issues.contains(&ProfileIssue::ProductionSelectionDenied));
}

// r[verify molten.consensus.fast_path_model.base_prerequisites]
#[test]
fn malformed_reference_bounds_and_ordering_fail_closed() {
    let mut malformed = profile(THREE_NODES);
    malformed.source.artifact_revision = "floating-main".to_owned();
    malformed.max_steps = 0;
    malformed.base_ordering.execution_order_preserved = false;
    let issues = validate_profile(&malformed).issues;
    assert!(issues.contains(&ProfileIssue::UnknownReference));
    assert!(issues.contains(&ProfileIssue::BoundExceeded("steps")));
    assert!(issues.contains(&ProfileIssue::BaseExecutionReorders));
}

// r[verify molten.consensus.fast_path_model.conflict_contract]
#[test]
fn conflict_contract_is_conservative_for_unknown_semantics() {
    let left = CommandShape::Put {
        key: "left".to_owned(),
        value_ref: "value-a".to_owned(),
    };
    let right = CommandShape::Get {
        key: "right".to_owned(),
    };
    assert_eq!(classify_conflict(&left, &right), ConflictDecision::Independent);
    assert_eq!(classify_conflict(&left, &CommandShape::Get { key: "left".to_owned() }), ConflictDecision::Conflict);
    assert_eq!(
        classify_conflict(&left, &CommandShape::Range {
            start: "a".to_owned(),
            end: "z".to_owned(),
        },),
        ConflictDecision::ConservativeFallback
    );
}

// r[verify molten.consensus.fast_path_model.stable_view]
#[test]
fn same_view_superquorum_and_all_proposer_promises_fast_commit() {
    let quorums = derive_quorums(THREE_NODES).expect("named profile quorum");
    assert_eq!(evaluate_stable_view(&attempt(THREE_NODES), quorums), StableViewDecision::FastCommitted);
}

// r[verify molten.consensus.fast_path_model.fallback_identity]
#[test]
fn mixed_views_missing_promises_and_identity_drift_fallback() {
    let quorums = derive_quorums(THREE_NODES).expect("named profile quorum");
    let mut mixed = attempt(THREE_NODES);
    mixed.acknowledgements[0].base_view = NEXT_VIEW;
    assert_eq!(evaluate_stable_view(&mixed, quorums), StableViewDecision::Fallback(FallbackReason::MixedView));

    let mut missing = attempt(THREE_NODES);
    missing.promises.clear();
    assert_eq!(
        evaluate_stable_view(&missing, quorums),
        StableViewDecision::Fallback(FallbackReason::MissingProposerPromise)
    );

    let mut drift = attempt(THREE_NODES);
    drift.original_operation.command_ref = "other-command".to_owned();
    assert_eq!(
        evaluate_stable_view(&drift, quorums),
        StableViewDecision::Fallback(FallbackReason::IdentityMismatch)
    );
}

#[test]
fn convergence_suppresses_duplicate_application_and_reply() {
    let mut ledger = ApplicationLedger::default();
    let operation = operation("command-a");
    let first = ledger.converge(&operation);
    let duplicate = ledger.converge(&operation);
    assert!(first.is_applied);
    assert!(first.is_replied);
    assert!(!first.is_duplicate_suppressed);
    assert!(!duplicate.is_applied);
    assert!(!duplicate.is_replied);
    assert!(duplicate.is_duplicate_suppressed);
    assert_eq!(ledger.applied_count(), 1);
}

// r[verify molten.consensus.fast_path_model.view_change_recovery]
#[test]
fn recovery_orders_agreed_set_or_empty_marker_before_resume() {
    let initial = RecoveryState {
        acceleration_view: INITIAL_VIEW,
        base_view: INITIAL_VIEW,
        last_normal_view: INITIAL_VIEW,
        phase: RecoveryPhase::Normal,
        accepted_commands: strings(&["command-a"]),
        recovery_set: BTreeSet::new(),
        marker_ref: None,
    };
    let paused = transition_recovery(&initial, &RecoveryAction::BeginViewChange {
        new_base_view: NEXT_VIEW,
    })
    .expect("pause recovery");
    let agreed = transition_recovery(&paused, &RecoveryAction::AgreeRecoverySet {
        commands: strings(&["command-a"]),
    })
    .expect("agree recovery");
    assert_eq!(
        transition_recovery(&agreed, &RecoveryAction::ResumeNormalView {
            new_acceleration_view: NEXT_VIEW,
        },),
        Err(RecoveryIssue::ResumeBeforeMarker)
    );
    let marked = transition_recovery(&agreed, &RecoveryAction::CommitRecoveryMarker {
        marker_ref: "blake3:recovery-marker".to_owned(),
    })
    .expect("commit marker");
    let resumed = transition_recovery(&marked, &RecoveryAction::ResumeNormalView {
        new_acceleration_view: NEXT_VIEW,
    })
    .expect("resume normal");
    assert_eq!(resumed.phase, RecoveryPhase::Normal);
    assert_eq!(resumed.acceleration_view, NEXT_VIEW);
}

#[test]
fn cascading_recovery_cannot_drop_previously_accepted_commands() {
    let state = RecoveryState {
        acceleration_view: INITIAL_VIEW,
        base_view: NEXT_VIEW,
        last_normal_view: INITIAL_VIEW,
        phase: RecoveryPhase::Paused,
        accepted_commands: strings(&["command-a"]),
        recovery_set: strings(&["command-a"]),
        marker_ref: None,
    };
    assert_eq!(
        transition_recovery(&state, &RecoveryAction::AgreeRecoverySet {
            commands: BTreeSet::new(),
        },),
        Err(RecoveryIssue::RecoverySetDropsAcceptedCommand)
    );
}

// r[verify molten.consensus.fast_path_model.fault_corpus]
#[test]
fn invariants_name_recoverability_predecessor_and_duplicate_hazards() {
    let input = InvariantInput {
        fast_replied: strings(&["fast-a"]),
        recoverable: BTreeSet::new(),
        conflicting_predecessors: strings(&["stale-b"]),
        applied_counts: BTreeMap::from([("fast-a".to_owned(), DUPLICATE_COUNT)]),
        reply_counts: BTreeMap::from([("session-a:7".to_owned(), DUPLICATE_COUNT)]),
        committed_order_agrees: false,
        execution_order_agrees: false,
    };
    let violations = evaluate_invariants(&input);
    assert!(violations.contains(&InvariantViolation::AcknowledgedCommandNotRecoverable("fast-a".to_owned())));
    assert!(violations.contains(&InvariantViolation::ConflictingPredecessor("stale-b".to_owned())));
    assert!(violations.contains(&InvariantViolation::DuplicateApplication("fast-a".to_owned())));
    assert!(violations.contains(&InvariantViolation::CommittedOrderMismatch));
    assert!(violations.contains(&InvariantViolation::ExecutionOrderMismatch));
}

// r[verify molten.consensus.fast_path_model.evidence]
#[test]
fn counterexample_minimization_keeps_only_causal_prefix() {
    let steps = vec![
        ModelStep {
            sequence: 1,
            kind: ModelStepKind::FastReply,
            operation_ref: Some("command-a".to_owned()),
            view: INITIAL_VIEW,
            causal: true,
        },
        ModelStep {
            sequence: 2,
            kind: ModelStepKind::Rejoin,
            operation_ref: None,
            view: INITIAL_VIEW,
            causal: false,
        },
        ModelStep {
            sequence: FIRST_DIVERGENCE,
            kind: ModelStepKind::OriginalCommit,
            operation_ref: Some("stale-b".to_owned()),
            view: NEXT_VIEW,
            causal: true,
        },
        ModelStep {
            sequence: LATER_STEP_SEQUENCE,
            kind: ModelStepKind::Apply,
            operation_ref: Some("later".to_owned()),
            view: NEXT_VIEW,
            causal: true,
        },
    ];
    let minimized = minimize_counterexample(&steps, FIRST_DIVERGENCE);
    assert_eq!(minimized.len(), 2);
    assert_eq!(minimized.last().map(|step| step.sequence), Some(FIRST_DIVERGENCE));
}

// r[verify molten.consensus.fast_path_model.reference_conformance]
#[test]
fn reference_mismatch_and_unsupported_assumption_block_conformance() {
    let conformance = compare_reference(&[
        ReferenceScenario {
            name: "recovery-priority".to_owned(),
            expected_safe: true,
            observed_safe: false,
            conflict_decision: ConflictDecision::Conflict,
            external_assumption_supported: true,
        },
        ReferenceScenario {
            name: "transport-timing".to_owned(),
            expected_safe: true,
            observed_safe: true,
            conflict_decision: ConflictDecision::ConservativeFallback,
            external_assumption_supported: false,
        },
    ]);
    assert_eq!(conformance.mismatches, vec!["recovery-priority"]);
    assert_eq!(conformance.unsupported_assumptions, vec!["transport-timing"]);
    assert!(!conformance.proof_transferred);
}

#[test]
fn named_fault_corpus_is_bounded_and_exposes_three_node_availability() {
    let corpus = default_fault_corpus();
    let partial = explore_fault_corpus(&corpus, PARTIAL_SCENARIO_BOUND);
    assert_eq!(partial.visited_scenarios.len(), PARTIAL_SCENARIO_BOUND);
    assert!(partial.unexplored_scenarios > 0);
    assert!(corpus.iter().all(|scenario| validate_fault_scenario(scenario).is_empty()));
    assert!(corpus.iter().any(|scenario| !scenario.expected_safe));
    let three_node_loss = corpus
        .iter()
        .find(|scenario| scenario.kind == FaultScenarioKind::QuorumLoss)
        .expect("three-node quorum-loss scenario");
    assert!(!three_node_loss.fast_path_available);
    assert!(three_node_loss.original_path_available);
}

// r[verify molten.consensus.fast_path_model.validation]
#[test]
fn model_evidence_is_deterministic_bounded_and_non_promotional() {
    let evidence = ModelRunEvidence {
        profile_ref: "blake3:profile".to_owned(),
        source_revision: JETPACK_ARTIFACT_REVISION.to_owned(),
        claim_profile: MODEL_ONLY_CLAIM.to_owned(),
        steps: Vec::new(),
        violations: Vec::new(),
        coverage: Coverage {
            explored_transitions: EXPLORED_TRANSITIONS,
            eligible_transitions: ELIGIBLE_TRANSITIONS,
        },
        first_divergence: None,
        non_claims: strings(required_non_claims()),
    };
    assert!(evidence_is_model_only(&evidence));
    assert_eq!(canonical_run_material(&evidence), canonical_run_material(&evidence));
    assert!(evidence.coverage.explored_transitions < evidence.coverage.eligible_transitions);
    assert!(operator_readback(&evidence).contains("production=denied"));
}
