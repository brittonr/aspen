//! Invariant evaluation, counterexample minimization, reference conformance, and the fault corpus.

const FIRST_DIVERGENCE: u64 = 3;
const LATER_STEP_SEQUENCE: u64 = 4;
const PARTIAL_SCENARIO_BOUND: u32 = 2;
const DUPLICATE_COUNT: usize = 2;
const EXPLORED_TRANSITIONS: usize = 3;
const ELIGIBLE_TRANSITIONS: usize = 5;

fn step(
    sequence: u64,
    kind: crate::witness::ModelStepKind,
    operation_ref: Option<&str>,
    causal: bool,
) -> crate::witness::ModelStep {
    crate::witness::ModelStep {
        sequence,
        kind,
        operation_ref: operation_ref.map(str::to_owned),
        view: super::support::INITIAL_VIEW,
        causal,
    }
}

fn model_run(
    steps: Vec<crate::witness::ModelStep>,
    violations: Vec<crate::witness::InvariantViolation>,
) -> crate::witness::ModelRunEvidence {
    crate::witness::ModelRunEvidence {
        profile_ref: "blake3:profile".to_owned(),
        source_revision: crate::envelope::JETPACK_ARTIFACT_REVISION.to_owned(),
        claim_profile: crate::envelope::MODEL_ONLY_CLAIM.to_owned(),
        steps,
        violations,
        coverage: crate::witness::Coverage {
            explored_transitions: EXPLORED_TRANSITIONS,
            eligible_transitions: ELIGIBLE_TRANSITIONS,
        },
        first_divergence: None,
        non_claims: super::support::strings(crate::envelope::required_non_claims()),
    }
}

// r[verify molten.consensus.fast_path_model.fault_corpus]
#[test]
fn invariants_name_recoverability_predecessor_and_duplicate_hazards() {
    let input = crate::witness::InvariantInput {
        fast_replied: super::support::strings(&["fast-a"]),
        recoverable: std::collections::BTreeSet::new(),
        conflicting_predecessors: super::support::strings(&["stale-b"]),
        applied_counts: std::collections::BTreeMap::from([("fast-a".to_owned(), DUPLICATE_COUNT)]),
        reply_counts: std::collections::BTreeMap::from([("session-a:7".to_owned(), DUPLICATE_COUNT)]),
        committed_order_agrees: false,
        execution_order_agrees: false,
    };
    assert_eq!(crate::witness::evaluate_invariants(&input), vec![
        crate::witness::InvariantViolation::AcknowledgedCommandNotRecoverable("fast-a".to_owned()),
        crate::witness::InvariantViolation::ConflictingPredecessor("stale-b".to_owned()),
        crate::witness::InvariantViolation::DuplicateApplication("fast-a".to_owned()),
        crate::witness::InvariantViolation::DuplicateReply("session-a:7".to_owned()),
        crate::witness::InvariantViolation::ExecutionOrderMismatch,
        crate::witness::InvariantViolation::CommittedOrderMismatch,
    ]);
    let healthy = crate::witness::InvariantInput {
        fast_replied: super::support::strings(&["fast-a"]),
        recoverable: super::support::strings(&["fast-a"]),
        conflicting_predecessors: std::collections::BTreeSet::new(),
        applied_counts: std::collections::BTreeMap::from([("fast-a".to_owned(), 1)]),
        reply_counts: std::collections::BTreeMap::from([("session-a:7".to_owned(), 1)]),
        committed_order_agrees: true,
        execution_order_agrees: true,
    };
    assert!(crate::witness::evaluate_invariants(&healthy).is_empty());
}

// r[verify molten.consensus.fast_path_model.evidence]
#[test]
fn counterexample_minimization_keeps_only_causal_prefix() {
    let steps = vec![
        step(1, crate::witness::ModelStepKind::FastReply, Some("command-a"), true),
        step(2, crate::witness::ModelStepKind::Rejoin, None, false),
        step(FIRST_DIVERGENCE, crate::witness::ModelStepKind::OriginalCommit, Some("stale-b"), true),
        step(LATER_STEP_SEQUENCE, crate::witness::ModelStepKind::Apply, Some("later"), true),
    ];
    let minimized = crate::witness::minimize_counterexample(&steps, FIRST_DIVERGENCE);
    assert_eq!(minimized.iter().map(|row| row.sequence).collect::<Vec<_>>(), vec![1, FIRST_DIVERGENCE]);
}

#[test]
fn repro_bundle_preserves_model_only_claim_and_expected_failure() {
    let expected = crate::witness::InvariantViolation::ConflictingPredecessor("stale-command".to_owned());
    let mut run = model_run(
        vec![step(
            FIRST_DIVERGENCE,
            crate::witness::ModelStepKind::OriginalCommit,
            Some("stale-command"),
            true,
        )],
        vec![expected.clone()],
    );
    assert_eq!(crate::witness::export_repro_bundle(&run, expected.clone()), None, "no divergence, no bundle");
    run.first_divergence = Some(FIRST_DIVERGENCE);
    let bundle = crate::witness::export_repro_bundle(&run, expected.clone()).expect("model repro bundle");
    assert_eq!(bundle.expected_violation, expected);
    assert_eq!(bundle.claim_profile, crate::envelope::MODEL_ONLY_CLAIM);
    assert!(!bundle.live_engine_claim);
    assert!(!bundle.measured_performance_claim);
    run.claim_profile = "production".to_owned();
    assert_eq!(crate::witness::export_repro_bundle(&run, expected), None, "a promoted claim exports nothing");
}

// r[verify molten.consensus.fast_path_model.reference_conformance]
#[test]
fn reference_mismatch_and_unsupported_assumption_block_conformance() {
    let conformance = crate::witness::compare_reference(&[
        crate::witness::ReferenceScenario {
            name: "recovery-priority".to_owned(),
            expected_safe: true,
            observed_safe: false,
            conflict_decision: crate::commute::ConflictDecision::Conflict,
            external_assumption_supported: true,
        },
        crate::witness::ReferenceScenario {
            name: "transport-timing".to_owned(),
            expected_safe: true,
            observed_safe: true,
            conflict_decision: crate::commute::ConflictDecision::ConservativeFallback,
            external_assumption_supported: false,
        },
    ]);
    assert_eq!(conformance.mismatches, vec!["recovery-priority"]);
    assert_eq!(conformance.unsupported_assumptions, vec!["transport-timing"]);
    assert!(!conformance.proof_transferred);
}

// r[verify molten.consensus.fast_path_model.fault_corpus]
#[test]
fn named_fault_corpus_is_bounded_and_exposes_three_node_availability() {
    let corpus = crate::hazard::default_fault_corpus();
    let partial = crate::hazard::explore_fault_corpus(&corpus, PARTIAL_SCENARIO_BOUND);
    assert_eq!(partial.visited_scenarios.len(), 2);
    assert_eq!(partial.unexplored_scenarios, corpus.len() - 2);
    let whole = crate::hazard::explore_fault_corpus(&corpus, u32::MAX);
    assert_eq!(whole.visited_scenarios.len(), corpus.len());
    assert_eq!(whole.unexplored_scenarios, 0);
    assert!(corpus.iter().all(|scenario| crate::hazard::validate_fault_scenario(scenario).is_empty()));
    assert!(corpus.iter().any(|scenario| !scenario.expected_safe));
    let three_node_loss = corpus
        .iter()
        .find(|scenario| scenario.kind == crate::hazard::FaultScenarioKind::QuorumLoss)
        .expect("three-node quorum-loss scenario");
    assert!(!three_node_loss.fast_path_available);
    assert!(three_node_loss.original_path_available);

    let mut unbounded = corpus[0].clone();
    unbounded.node_count = 4;
    unbounded.step_bound = crate::hazard::MAX_FAULT_SCENARIO_STEPS + 1;
    unbounded.expected_safe = false;
    assert_eq!(crate::hazard::validate_fault_scenario(&unbounded), vec![
        crate::hazard::FaultScenarioIssue::UnsupportedNodeCount,
        crate::hazard::FaultScenarioIssue::OverStepBound,
        crate::hazard::FaultScenarioIssue::MissingExpectedViolation,
    ]);
}

// r[verify molten.consensus.fast_path_model.validation]
#[test]
fn model_evidence_is_deterministic_bounded_and_non_promotional() {
    let run = model_run(Vec::new(), Vec::new());
    assert!(crate::witness::evidence_is_model_only(&run));
    assert_eq!(crate::witness::canonical_run_material(&run), crate::witness::canonical_run_material(&run));
    assert!(run.coverage.explored_transitions < run.coverage.eligible_transitions);
    assert!(crate::witness::operator_readback(&run).contains("production=denied"));
}
