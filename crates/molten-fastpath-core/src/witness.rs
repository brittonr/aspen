//! Model-only run evidence, invariant evaluation, counterexample minimization, and reference
//! conformance.

use alloc::borrow::ToOwned;
use alloc::string::ToString;

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
    pub sequence: u64,
    pub kind: ModelStepKind,
    pub operation_ref: Option<alloc::string::String>,
    pub view: u64,
    pub causal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum InvariantViolation {
    AcknowledgedCommandNotRecoverable(alloc::string::String),
    ConflictingPredecessor(alloc::string::String),
    DuplicateApplication(alloc::string::String),
    DuplicateReply(alloc::string::String),
    ExecutionOrderMismatch,
    CommittedOrderMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvariantInput {
    pub fast_replied: alloc::collections::BTreeSet<alloc::string::String>,
    pub recoverable: alloc::collections::BTreeSet<alloc::string::String>,
    pub conflicting_predecessors: alloc::collections::BTreeSet<alloc::string::String>,
    pub applied_counts: alloc::collections::BTreeMap<alloc::string::String, usize>,
    pub reply_counts: alloc::collections::BTreeMap<alloc::string::String, usize>,
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
    pub profile_ref: alloc::string::String,
    pub source_revision: alloc::string::String,
    pub claim_profile: alloc::string::String,
    pub steps: alloc::vec::Vec<ModelStep>,
    pub violations: alloc::vec::Vec<InvariantViolation>,
    pub coverage: Coverage,
    pub first_divergence: Option<u64>,
    pub non_claims: alloc::collections::BTreeSet<alloc::string::String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEvidence {
    pub last_normal_view: u64,
    pub recovered_commands: alloc::collections::BTreeSet<alloc::string::String>,
    pub marker_ref: alloc::string::String,
    pub resumed_view: u64,
    pub new_work_admitted_after_marker: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceScenario {
    pub name: alloc::string::String,
    pub expected_safe: bool,
    pub observed_safe: bool,
    pub conflict_decision: crate::commute::ConflictDecision,
    pub external_assumption_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceConformance {
    pub source: alloc::string::String,
    pub revision: alloc::string::String,
    pub mismatches: alloc::vec::Vec<alloc::string::String>,
    pub unsupported_assumptions: alloc::vec::Vec<alloc::string::String>,
    pub proof_transferred: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelReproBundle {
    pub profile_ref: alloc::string::String,
    pub source_revision: alloc::string::String,
    pub claim_profile: alloc::string::String,
    pub minimized_steps: alloc::vec::Vec<ModelStep>,
    pub expected_violation: InvariantViolation,
    pub live_engine_claim: bool,
    pub measured_performance_claim: bool,
}

// r[impl molten.consensus.fast_path_model.fault_corpus]
pub fn evaluate_invariants(input: &InvariantInput) -> alloc::vec::Vec<InvariantViolation> {
    let unrecoverable = input
        .fast_replied
        .difference(&input.recoverable)
        .map(|command| InvariantViolation::AcknowledgedCommandNotRecoverable(command.clone()));
    let predecessors = input
        .conflicting_predecessors
        .iter()
        .map(|command| InvariantViolation::ConflictingPredecessor(command.clone()));
    let applications = input
        .applied_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(command, _)| InvariantViolation::DuplicateApplication(command.clone()));
    let replies = input
        .reply_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(operation, _)| InvariantViolation::DuplicateReply(operation.clone()));
    let committed = (!input.committed_order_agrees).then_some(InvariantViolation::CommittedOrderMismatch);
    let executed = (!input.execution_order_agrees).then_some(InvariantViolation::ExecutionOrderMismatch);
    let mut violations: alloc::vec::Vec<InvariantViolation> = unrecoverable
        .chain(predecessors)
        .chain(applications)
        .chain(replies)
        .chain(committed)
        .chain(executed)
        .collect();
    violations.sort();
    violations
}

// r[impl molten.consensus.fast_path_model.evidence]
pub fn minimize_counterexample(steps: &[ModelStep], first_divergence: u64) -> alloc::vec::Vec<ModelStep> {
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

pub fn operator_readback(evidence: &ModelRunEvidence) -> alloc::string::String {
    alloc::format!(
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
    let mut mismatches: alloc::vec::Vec<alloc::string::String> = scenarios
        .iter()
        .filter(|scenario| scenario.expected_safe != scenario.observed_safe)
        .map(|scenario| scenario.name.clone())
        .collect();
    let mut unsupported_assumptions: alloc::vec::Vec<alloc::string::String> = scenarios
        .iter()
        .filter(|scenario| !scenario.external_assumption_supported)
        .map(|scenario| scenario.name.clone())
        .collect();
    mismatches.sort();
    unsupported_assumptions.sort();
    ReferenceConformance {
        source: crate::envelope::JETPACK_ARTIFACT_SOURCE.to_owned(),
        revision: crate::envelope::JETPACK_ARTIFACT_REVISION.to_owned(),
        mismatches,
        unsupported_assumptions,
        proof_transferred: false,
    }
}

// r[impl molten.consensus.fast_path_model.evidence]
pub fn canonical_run_material(evidence: &ModelRunEvidence) -> alloc::string::String {
    let step_rows = evidence
        .steps
        .iter()
        .map(|step| alloc::format!("{}:{:?}:{}", step.sequence, step.kind, step.view))
        .collect::<alloc::vec::Vec<_>>()
        .join("|");
    let violation_rows = evidence
        .violations
        .iter()
        .map(|violation| alloc::format!("{violation:?}"))
        .collect::<alloc::vec::Vec<_>>()
        .join("|");
    alloc::format!(
        "profile={}\nsource={}\nclaim={}\nsteps={}\nviolations={}\ncoverage={}/{}\nfirst-divergence={:?}\nnon-claims={}\n",
        evidence.profile_ref,
        evidence.source_revision,
        evidence.claim_profile,
        step_rows,
        violation_rows,
        evidence.coverage.explored_transitions,
        evidence.coverage.eligible_transitions,
        evidence.first_divergence,
        evidence.non_claims.iter().map(ToString::to_string).collect::<alloc::vec::Vec<_>>().join("|"),
    )
}

// r[impl molten.consensus.fast_path_model.nonclaims]
pub fn evidence_is_model_only(evidence: &ModelRunEvidence) -> bool {
    evidence.claim_profile == crate::envelope::MODEL_ONLY_CLAIM
        && evidence.source_revision == crate::envelope::JETPACK_ARTIFACT_REVISION
        && !evidence.non_claims.is_empty()
}
