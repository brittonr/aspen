// Admission chain for declarative resource records.
//
// Ordered admission phases for resource create, update, status, delete, and
// reconcile-apply intents. Pure core functions validate phase results and
// produce admission receipts. The shell owns persistence and side effects.
//
// Type aliases and common helpers (record, string, u64_value, canonical_hash,
// validate_content_ref, require_ref, validate_non_empty, sequence, refs_sequence,
// optional_ref_value, bool_value, symbol) are inherited from p000.

const MAX_PHASE_DIAGNOSTICS: usize = 32;
const MAX_PHASE_EVIDENCE_REFS: usize = 128;
const MAX_MUTATION_RULE_REFS: usize = 64;
const _: () = assert!(MAX_PHASE_DIAGNOSTICS > 0);
const _: () = assert!(MAX_PHASE_EVIDENCE_REFS > 0);
const _: () = assert!(MAX_MUTATION_RULE_REFS > 0);

// ---------------------------------------------------------------------------
// Resource operation intent
// ---------------------------------------------------------------------------

/// The type of resource operation being admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceOperation {
    Create,
    Update,
    Status,
    Delete,
    ReconcileApply,
}

impl ResourceOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            ResourceOperation::Create => "create",
            ResourceOperation::Update => "update",
            ResourceOperation::Status => "status",
            ResourceOperation::Delete => "delete",
            ResourceOperation::ReconcileApply => "reconcile-apply",
        }
    }
}

// ---------------------------------------------------------------------------
// Ordered admission phases
// ---------------------------------------------------------------------------

/// Ordered admission phases. Each phase result binds evidence and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmissionPhase {
    EnvelopeDecode,
    SchemaValidation,
    AuthorityPreflight,
    Defaulting,
    ReviewedMutation,
    FinalValidation,
    PolicyEvidenceGates,
    CommitPlan,
}

impl AdmissionPhase {
    pub fn all() -> &'static [AdmissionPhase] {
        &[
            AdmissionPhase::EnvelopeDecode,
            AdmissionPhase::SchemaValidation,
            AdmissionPhase::AuthorityPreflight,
            AdmissionPhase::Defaulting,
            AdmissionPhase::ReviewedMutation,
            AdmissionPhase::FinalValidation,
            AdmissionPhase::PolicyEvidenceGates,
            AdmissionPhase::CommitPlan,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AdmissionPhase::EnvelopeDecode => "envelope-decode",
            AdmissionPhase::SchemaValidation => "schema-validation",
            AdmissionPhase::AuthorityPreflight => "authority-preflight",
            AdmissionPhase::Defaulting => "defaulting",
            AdmissionPhase::ReviewedMutation => "reviewed-mutation",
            AdmissionPhase::FinalValidation => "final-validation",
            AdmissionPhase::PolicyEvidenceGates => "policy-evidence-gates",
            AdmissionPhase::CommitPlan => "commit-plan",
        }
    }

    pub fn index(self) -> u32 {
        match self {
            AdmissionPhase::EnvelopeDecode => 0,
            AdmissionPhase::SchemaValidation => 1,
            AdmissionPhase::AuthorityPreflight => 2,
            AdmissionPhase::Defaulting => 3,
            AdmissionPhase::ReviewedMutation => 4,
            AdmissionPhase::FinalValidation => 5,
            AdmissionPhase::PolicyEvidenceGates => 6,
            AdmissionPhase::CommitPlan => 7,
        }
    }
}

// ---------------------------------------------------------------------------
// Admission phase result
// ---------------------------------------------------------------------------

/// Result of a single admission phase evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseResult {
    pub phase: AdmissionPhase,
    pub decision: PhaseDecision,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhaseDecision {
    Pass,
    Deny,
    Skip,
}

impl PhaseDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            PhaseDecision::Pass => "pass",
            PhaseDecision::Deny => "deny",
            PhaseDecision::Skip => "skip",
        }
    }

    pub fn is_pass(&self) -> bool {
        matches!(self, PhaseDecision::Pass | PhaseDecision::Skip)
    }
}

// ---------------------------------------------------------------------------
// Admission chain input
// ---------------------------------------------------------------------------

/// Ordered admission chain input for resource operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionChainInput {
    pub operation: ResourceOperation,
    pub resource_ref: String,
    pub candidate_ref: String,
    pub envelope_decode_passed: Option<PhaseEvidence>,
    pub schema_validation_passed: Option<PhaseEvidence>,
    pub authority_preflight_passed: Option<PhaseEvidence>,
    pub defaulting_evidence: Option<MutationEvidence>,
    pub mutation_evidence: Option<MutationEvidence>,
    pub final_validation_passed: Option<PhaseEvidence>,
    pub policy_evidence_gates: Vec<String>,
}

/// Summary evidence for a phase that passed at the imperative shell level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseEvidence {
    pub evidence_refs: Vec<String>,
}

/// Mutation evidence binding rule ref and pre/post candidate refs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutationEvidence {
    pub rule_ref: String,
    pub pre_mutation_ref: String,
    pub post_mutation_ref: String,
}

// ---------------------------------------------------------------------------
// Admission chain result
// ---------------------------------------------------------------------------

/// The ordered admission chain result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionChainResult {
    pub operation: ResourceOperation,
    pub pass: bool,
    pub phase_results: Vec<PhaseResult>,
    pub commit_plan_ref: Option<String>,
    pub diagnostics: Vec<String>,
}

// ---------------------------------------------------------------------------
// Pure core: evaluate ordered admission chain
// ---------------------------------------------------------------------------

/// Evaluate an ordered admission chain for a resource operation.
///
/// Returns an `AdmissionChainResult` with per-phase decisions.
/// A later phase MUST NOT claim success when an earlier phase denied.
pub fn evaluate_admission_chain(input: &AdmissionChainInput) -> AdmissionChainResult {
    let mut phase_results = Vec::with_capacity(AdmissionPhase::all().len());
    let mut is_overall_pass = true;
    let mut denial_diagnostic = None;

    for phase in AdmissionPhase::all() {
        let result = evaluate_single_phase(*phase, input);
        let is_denied = matches!(result.decision, PhaseDecision::Deny);
        if is_denied {
            is_overall_pass = false;
            denial_diagnostic = Some(format!(
                "phase {} denied: {}",
                result.phase.as_str(),
                result.diagnostics.join(", ")
            ));
        }
        phase_results.push(result);

        // A later phase must not continue after a deny
        if is_denied {
            for remaining in AdmissionPhase::all() {
                if remaining.index() > phase.index() {
                    phase_results.push(PhaseResult {
                        phase: *remaining,
                        decision: PhaseDecision::Skip,
                        evidence_refs: Vec::new(),
                        diagnostics: vec![format!(
                            "skipped because earlier phase {} denied",
                            phase.as_str()
                        )],
                    });
                }
            }
            break;
        }
    }

    let commit_plan_ref = if is_overall_pass {
        Some(generate_commit_plan_ref(input))
    } else {
        None
    };

    AdmissionChainResult {
        operation: input.operation,
        pass: is_overall_pass,
        phase_results,
        commit_plan_ref,
        diagnostics: denial_diagnostic.into_iter().collect(),
    }
}

fn evaluate_single_phase(phase: AdmissionPhase, input: &AdmissionChainInput) -> PhaseResult {
    match phase {
        AdmissionPhase::EnvelopeDecode => {
            evidence_phase(phase, input.envelope_decode_passed.as_ref(), "missing envelope decode evidence")
        }
        AdmissionPhase::SchemaValidation => {
            evidence_phase(phase, input.schema_validation_passed.as_ref(), "missing schema validation evidence")
        }
        AdmissionPhase::AuthorityPreflight => {
            evidence_phase(phase, input.authority_preflight_passed.as_ref(), "missing authority preflight evidence")
        }
        AdmissionPhase::Defaulting => {
            // Defaulting may be skipped if the resource has no defaults to apply
            if input.operation == ResourceOperation::Status || input.operation == ResourceOperation::Delete {
                unpassed_phase(phase, PhaseDecision::Skip, "defaulting skipped for status/delete operation")
            } else if let Some(ref evidence) = input.defaulting_evidence {
                passed_phase(phase, vec![evidence.rule_ref.clone()])
            } else {
                unpassed_phase(phase, PhaseDecision::Skip, "no defaulting evidence (resource may have none)")
            }
        }
        AdmissionPhase::ReviewedMutation => reviewed_mutation_result(phase, input),
        AdmissionPhase::FinalValidation => {
            evidence_phase(phase, input.final_validation_passed.as_ref(), "missing final validation evidence")
        }
        AdmissionPhase::PolicyEvidenceGates => {
            if input.policy_evidence_gates.is_empty() {
                unpassed_phase(phase, PhaseDecision::Deny, "no policy evidence gates passed")
            } else {
                passed_phase(phase, input.policy_evidence_gates.clone())
            }
        }
        AdmissionPhase::CommitPlan => {
            // Commit plan phase is always a pass if we get here — the plan ref
            // is generated by the caller using the chain result
            passed_phase(phase, Vec::new())
        }
    }
}

fn passed_phase(phase: AdmissionPhase, evidence_refs: Vec<String>) -> PhaseResult {
    PhaseResult {
        phase,
        decision: PhaseDecision::Pass,
        evidence_refs,
        diagnostics: Vec::new(),
    }
}
