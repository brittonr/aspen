use std::collections::BTreeMap;
use std::collections::BTreeSet;

use super::conflict::ConflictDecision;
use super::profile::JETPACK_ARTIFACT_REVISION;
use super::profile::JETPACK_ARTIFACT_SOURCE;
use super::profile::MODEL_ONLY_CLAIM;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStepKind {
    OriginalPropose,
    FastAcknowledge,
    FastReply,
    BaseViewChange,
    RecoveryAgree,
    RecoveryMarker,
    OriginalCommit,
    Apply,
    Reply,
    Partition,
    Rejoin,
    Restart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStep {
    pub sequence: usize,
    pub kind: ModelStepKind,
    pub operation_ref: Option<String>,
    pub view: u64,
    pub causal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvariantViolation {
    AcknowledgedCommandNotRecoverable(String),
    ConflictingPredecessor(String),
    DuplicateApplication(String),
    DuplicateReply(String),
    ExecutionOrderMismatch,
    CommittedOrderMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantInput {
    pub fast_replied: BTreeSet<String>,
    pub recoverable: BTreeSet<String>,
    pub conflicting_predecessors: BTreeSet<String>,
    pub applied_counts: BTreeMap<String, usize>,
    pub reply_counts: BTreeMap<String, usize>,
    pub committed_order_agrees: bool,
    pub execution_order_agrees: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Coverage {
    pub explored_transitions: usize,
    pub eligible_transitions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRunEvidence {
    pub profile_ref: String,
    pub source_revision: String,
    pub claim_profile: String,
    pub steps: Vec<ModelStep>,
    pub violations: Vec<InvariantViolation>,
    pub coverage: Coverage,
    pub first_divergence: Option<usize>,
    pub non_claims: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEvidence {
    pub last_normal_view: u64,
    pub recovered_commands: BTreeSet<String>,
    pub marker_ref: String,
    pub resumed_view: u64,
    pub new_work_admitted_after_marker: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceScenario {
    pub name: String,
    pub expected_safe: bool,
    pub observed_safe: bool,
    pub conflict_decision: ConflictDecision,
    pub external_assumption_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceConformance {
    pub source: String,
    pub revision: String,
    pub mismatches: Vec<String>,
    pub unsupported_assumptions: Vec<String>,
    pub proof_transferred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelReproBundle {
    pub profile_ref: String,
    pub source_revision: String,
    pub claim_profile: String,
    pub minimized_steps: Vec<ModelStep>,
    pub expected_violation: InvariantViolation,
    pub live_engine_claim: bool,
    pub measured_performance_claim: bool,
}

// r[impl molten.consensus.fast_path_model.fault_corpus]
pub fn evaluate_invariants(input: &InvariantInput) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();
    for command in input.fast_replied.difference(&input.recoverable) {
        violations.push(InvariantViolation::AcknowledgedCommandNotRecoverable(command.clone()));
    }
    for command in &input.conflicting_predecessors {
        violations.push(InvariantViolation::ConflictingPredecessor(command.clone()));
    }
    for (command, count) in &input.applied_counts {
        if *count > 1 {
            violations.push(InvariantViolation::DuplicateApplication(command.clone()));
        }
    }
    for (operation, count) in &input.reply_counts {
        if *count > 1 {
            violations.push(InvariantViolation::DuplicateReply(operation.clone()));
        }
    }
    if !input.committed_order_agrees {
        violations.push(InvariantViolation::CommittedOrderMismatch);
    }
    if !input.execution_order_agrees {
        violations.push(InvariantViolation::ExecutionOrderMismatch);
    }
    violations.sort();
    violations
}

// r[impl molten.consensus.fast_path_model.evidence]
pub fn minimize_counterexample(steps: &[ModelStep], first_divergence: usize) -> Vec<ModelStep> {
    steps
        .iter()
        .filter(|step| step.sequence <= first_divergence && (step.causal || step.sequence == first_divergence))
        .cloned()
        .collect()
}

// r[impl molten.consensus.fast_path_model.evidence]
// r[impl molten.consensus.fast_path_model.nonclaims]
pub fn export_repro_bundle(
    evidence: &ModelRunEvidence,
    expected_violation: InvariantViolation,
) -> Option<ModelReproBundle> {
    let first_divergence = evidence.first_divergence?;
    if !evidence.violations.contains(&expected_violation) || !evidence_is_model_only(evidence) {
        return None;
    }
    Some(ModelReproBundle {
        profile_ref: evidence.profile_ref.clone(),
        source_revision: evidence.source_revision.clone(),
        claim_profile: evidence.claim_profile.clone(),
        minimized_steps: minimize_counterexample(&evidence.steps, first_divergence),
        expected_violation,
        live_engine_claim: false,
        measured_performance_claim: false,
    })
}

pub fn operator_readback(evidence: &ModelRunEvidence) -> String {
    format!(
        "profile={} claim={} source={} explored={}/{} violations={} live=denied production=denied performance=unmeasured",
        evidence.profile_ref,
        evidence.claim_profile,
        evidence.source_revision,
        evidence.coverage.explored_transitions,
        evidence.coverage.eligible_transitions,
        evidence.violations.len(),
    )
}

// r[impl molten.consensus.fast_path_model.reference_conformance]
pub fn compare_reference(scenarios: &[ReferenceScenario]) -> ReferenceConformance {
    let mut mismatches = Vec::new();
    let mut unsupported_assumptions = Vec::new();
    for scenario in scenarios {
        if scenario.expected_safe != scenario.observed_safe {
            mismatches.push(scenario.name.clone());
        }
        if !scenario.external_assumption_supported {
            unsupported_assumptions.push(scenario.name.clone());
        }
    }
    mismatches.sort();
    unsupported_assumptions.sort();
    ReferenceConformance {
        source: JETPACK_ARTIFACT_SOURCE.to_owned(),
        revision: JETPACK_ARTIFACT_REVISION.to_owned(),
        mismatches,
        unsupported_assumptions,
        proof_transferred: false,
    }
}

// r[impl molten.consensus.fast_path_model.evidence]
pub fn canonical_run_material(evidence: &ModelRunEvidence) -> String {
    let step_rows = evidence
        .steps
        .iter()
        .map(|step| format!("{}:{:?}:{}", step.sequence, step.kind, step.view))
        .collect::<Vec<_>>()
        .join("|");
    let violation_rows =
        evidence.violations.iter().map(|violation| format!("{violation:?}")).collect::<Vec<_>>().join("|");
    format!(
        "profile={}\nsource={}\nclaim={}\nsteps={}\nviolations={}\ncoverage={}/{}\nfirst-divergence={:?}\nnon-claims={}\n",
        evidence.profile_ref,
        evidence.source_revision,
        evidence.claim_profile,
        step_rows,
        violation_rows,
        evidence.coverage.explored_transitions,
        evidence.coverage.eligible_transitions,
        evidence.first_divergence,
        evidence.non_claims.iter().cloned().collect::<Vec<_>>().join("|"),
    )
}

// r[impl molten.consensus.fast_path_model.nonclaims]
pub fn evidence_is_model_only(evidence: &ModelRunEvidence) -> bool {
    evidence.claim_profile == MODEL_ONLY_CLAIM
        && evidence.source_revision == JETPACK_ARTIFACT_REVISION
        && !evidence.non_claims.is_empty()
}
