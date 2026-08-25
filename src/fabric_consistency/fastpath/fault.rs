use super::evidence::InvariantViolation;

pub const MAX_FAULT_SCENARIO_STEPS: usize = 64;
const THREE_REPLICA_COUNT: usize = 3;
const FIVE_REPLICA_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultScenarioKind {
    NonConflictingFastCommit,
    ConflictFallback,
    OriginalOnly,
    ViewStraddledAcknowledgement,
    MissingProposerPromise,
    LeaderFailureAfterFastReply,
    StaleConflictingPredecessor,
    Partition,
    QuorumLoss,
    InterruptedRecovery,
    CascadingRecovery,
    ReplicaRestart,
    DuplicateConvergence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultScenario {
    pub name: String,
    pub kind: FaultScenarioKind,
    pub node_count: usize,
    pub step_bound: usize,
    pub expected_safe: bool,
    pub expected_violation: Option<InvariantViolation>,
    pub fast_path_available: bool,
    pub original_path_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FaultScenarioIssue {
    MissingExpectedViolation,
    NonPositiveBound,
    OverStepBound,
    UnsupportedNodeCount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultExploration {
    pub visited_scenarios: Vec<String>,
    pub detected_violations: Vec<InvariantViolation>,
    pub eligible_scenarios: usize,
    pub unexplored_scenarios: usize,
}

// r[impl molten.consensus.fast_path_model.fault_corpus]
// r[impl molten.consensus.fast_path_model.evidence]
pub fn explore_fault_corpus(corpus: &[FaultScenario], scenario_bound: usize) -> FaultExploration {
    let visited: Vec<&FaultScenario> = corpus.iter().take(scenario_bound).collect();
    let visited_scenarios = visited.iter().map(|scenario| scenario.name.clone()).collect();
    let mut detected_violations: Vec<InvariantViolation> =
        visited.iter().filter_map(|scenario| scenario.expected_violation.clone()).collect();
    detected_violations.sort();
    let unexplored_scenarios = corpus.len().saturating_sub(visited.len());
    FaultExploration {
        visited_scenarios,
        detected_violations,
        eligible_scenarios: corpus.len(),
        unexplored_scenarios,
    }
}

pub fn validate_fault_scenario(scenario: &FaultScenario) -> Vec<FaultScenarioIssue> {
    let mut issues = Vec::new();
    if !matches!(scenario.node_count, THREE_REPLICA_COUNT | FIVE_REPLICA_COUNT) {
        issues.push(FaultScenarioIssue::UnsupportedNodeCount);
    }
    if scenario.step_bound == 0 {
        issues.push(FaultScenarioIssue::NonPositiveBound);
    }
    if scenario.step_bound > MAX_FAULT_SCENARIO_STEPS {
        issues.push(FaultScenarioIssue::OverStepBound);
    }
    if !scenario.expected_safe && scenario.expected_violation.is_none() {
        issues.push(FaultScenarioIssue::MissingExpectedViolation);
    }
    issues
}

// r[impl molten.consensus.fast_path_model.fault_corpus]
pub fn default_fault_corpus() -> Vec<FaultScenario> {
    let safe = |name: &str, kind, node_count| FaultScenario {
        name: name.to_owned(),
        kind,
        node_count,
        step_bound: MAX_FAULT_SCENARIO_STEPS,
        expected_safe: true,
        expected_violation: None,
        fast_path_available: true,
        original_path_available: true,
    };
    let unsafe_scenario = |name: &str, kind, violation| FaultScenario {
        name: name.to_owned(),
        kind,
        node_count: FIVE_REPLICA_COUNT,
        step_bound: MAX_FAULT_SCENARIO_STEPS,
        expected_safe: false,
        expected_violation: Some(violation),
        fast_path_available: false,
        original_path_available: true,
    };
    vec![
        safe("non-conflicting-fast-commit", FaultScenarioKind::NonConflictingFastCommit, FIVE_REPLICA_COUNT),
        safe("conflict-fallback", FaultScenarioKind::ConflictFallback, FIVE_REPLICA_COUNT),
        safe("original-only", FaultScenarioKind::OriginalOnly, THREE_REPLICA_COUNT),
        unsafe_scenario(
            "view-straddled-acknowledgement",
            FaultScenarioKind::ViewStraddledAcknowledgement,
            InvariantViolation::AcknowledgedCommandNotRecoverable("command-a".to_owned()),
        ),
        unsafe_scenario(
            "missing-proposer-promise",
            FaultScenarioKind::MissingProposerPromise,
            InvariantViolation::ConflictingPredecessor("command-b".to_owned()),
        ),
        unsafe_scenario(
            "leader-failure-after-fast-reply",
            FaultScenarioKind::LeaderFailureAfterFastReply,
            InvariantViolation::AcknowledgedCommandNotRecoverable("command-a".to_owned()),
        ),
        unsafe_scenario(
            "stale-conflicting-predecessor",
            FaultScenarioKind::StaleConflictingPredecessor,
            InvariantViolation::ConflictingPredecessor("stale-command".to_owned()),
        ),
        safe("partition-fallback", FaultScenarioKind::Partition, FIVE_REPLICA_COUNT),
        FaultScenario {
            name: "three-replica-quorum-loss".to_owned(),
            kind: FaultScenarioKind::QuorumLoss,
            node_count: THREE_REPLICA_COUNT,
            step_bound: MAX_FAULT_SCENARIO_STEPS,
            expected_safe: true,
            expected_violation: None,
            fast_path_available: false,
            original_path_available: true,
        },
        safe("interrupted-recovery", FaultScenarioKind::InterruptedRecovery, FIVE_REPLICA_COUNT),
        safe("cascading-recovery", FaultScenarioKind::CascadingRecovery, FIVE_REPLICA_COUNT),
        safe("replica-restart", FaultScenarioKind::ReplicaRestart, FIVE_REPLICA_COUNT),
        safe("duplicate-convergence", FaultScenarioKind::DuplicateConvergence, FIVE_REPLICA_COUNT),
    ]
}
