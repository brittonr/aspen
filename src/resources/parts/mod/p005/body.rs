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

    pub fn index(self) -> usize {
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
    let mut phase_results = Vec::new();
    let mut overall_pass = true;
    let mut diagnostics = Vec::new();

    for phase in AdmissionPhase::all() {
        let result = evaluate_single_phase(*phase, input);
        let denied = matches!(result.decision, PhaseDecision::Deny);
        if denied {
            overall_pass = false;
            diagnostics.push(format!(
                "phase {} denied: {}",
                result.phase.as_str(),
                result.diagnostics.join(", ")
            ));
        }
        phase_results.push(result);

        // A later phase must not continue after a deny
        if denied {
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

    let commit_plan_ref = if overall_pass {
        Some(generate_commit_plan_ref(input))
    } else {
        None
    };

    AdmissionChainResult {
        operation: input.operation,
        pass: overall_pass,
        phase_results,
        commit_plan_ref,
        diagnostics,
    }
}

fn evaluate_single_phase(phase: AdmissionPhase, input: &AdmissionChainInput) -> PhaseResult {
    match phase {
        AdmissionPhase::EnvelopeDecode => {
            if let Some(ref evidence) = input.envelope_decode_passed {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Pass,
                    evidence_refs: evidence.evidence_refs.clone(),
                    diagnostics: Vec::new(),
                }
            } else {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Deny,
                    evidence_refs: Vec::new(),
                    diagnostics: vec!["missing envelope decode evidence".to_string()],
                }
            }
        }
        AdmissionPhase::SchemaValidation => {
            if let Some(ref evidence) = input.schema_validation_passed {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Pass,
                    evidence_refs: evidence.evidence_refs.clone(),
                    diagnostics: Vec::new(),
                }
            } else {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Deny,
                    evidence_refs: Vec::new(),
                    diagnostics: vec!["missing schema validation evidence".to_string()],
                }
            }
        }
        AdmissionPhase::AuthorityPreflight => {
            if let Some(ref evidence) = input.authority_preflight_passed {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Pass,
                    evidence_refs: evidence.evidence_refs.clone(),
                    diagnostics: Vec::new(),
                }
            } else {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Deny,
                    evidence_refs: Vec::new(),
                    diagnostics: vec!["missing authority preflight evidence".to_string()],
                }
            }
        }
        AdmissionPhase::Defaulting => {
            // Defaulting may be skipped if the resource has no defaults to apply
            if input.operation == ResourceOperation::Status || input.operation == ResourceOperation::Delete {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Skip,
                    evidence_refs: Vec::new(),
                    diagnostics: vec!["defaulting skipped for status/delete operation".to_string()],
                }
            } else if let Some(ref evidence) = input.defaulting_evidence {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Pass,
                    evidence_refs: vec![evidence.rule_ref.clone()],
                    diagnostics: Vec::new(),
                }
            } else {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Skip,
                    evidence_refs: Vec::new(),
                    diagnostics: vec!["no defaulting evidence (resource may have none)".to_string()],
                }
            }
        }
        AdmissionPhase::ReviewedMutation => {
            // Mutation is required for create/update operations
            if input.operation == ResourceOperation::Create || input.operation == ResourceOperation::Update {
                if let Some(ref evidence) = input.mutation_evidence {
                    if validate_mutation_evidence(evidence) {
                        PhaseResult {
                            phase,
                            decision: PhaseDecision::Pass,
                            evidence_refs: vec![
                                evidence.rule_ref.clone(),
                                evidence.pre_mutation_ref.clone(),
                                evidence.post_mutation_ref.clone(),
                            ],
                            diagnostics: Vec::new(),
                        }
                    } else {
                        PhaseResult {
                            phase,
                            decision: PhaseDecision::Deny,
                            evidence_refs: Vec::new(),
                            diagnostics: vec![
                                "mutation evidence has invalid refs or pre/post mismatch".to_string(),
                            ],
                        }
                    }
                } else {
                    PhaseResult {
                        phase,
                        decision: PhaseDecision::Deny,
                        evidence_refs: Vec::new(),
                        diagnostics: vec![
                            "missing mutation evidence for create/update operation".to_string(),
                        ],
                    }
                }
            } else {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Skip,
                    evidence_refs: Vec::new(),
                    diagnostics: vec![
                        format!("mutation phase skipped for {:?} operation", input.operation),
                    ],
                }
            }
        }
        AdmissionPhase::FinalValidation => {
            if let Some(ref evidence) = input.final_validation_passed {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Pass,
                    evidence_refs: evidence.evidence_refs.clone(),
                    diagnostics: Vec::new(),
                }
            } else {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Deny,
                    evidence_refs: Vec::new(),
                    diagnostics: vec!["missing final validation evidence".to_string()],
                }
            }
        }
        AdmissionPhase::PolicyEvidenceGates => {
            if input.policy_evidence_gates.is_empty() {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Deny,
                    evidence_refs: Vec::new(),
                    diagnostics: vec!["no policy evidence gates passed".to_string()],
                }
            } else {
                PhaseResult {
                    phase,
                    decision: PhaseDecision::Pass,
                    evidence_refs: input.policy_evidence_gates.clone(),
                    diagnostics: Vec::new(),
                }
            }
        }
        AdmissionPhase::CommitPlan => {
            // Commit plan phase is always a pass if we get here — the plan ref
            // is generated by the caller using the chain result
            PhaseResult {
                phase,
                decision: PhaseDecision::Pass,
                evidence_refs: Vec::new(),
                diagnostics: Vec::new(),
            }
        }
    }
}

fn validate_mutation_evidence(evidence: &MutationEvidence) -> bool {
    // All refs must be valid content refs
    let rule_ok = validate_content_ref(&evidence.rule_ref).is_ok();
    let pre_ok = validate_content_ref(&evidence.pre_mutation_ref).is_ok();
    let post_ok = validate_content_ref(&evidence.post_mutation_ref).is_ok();
    rule_ok && pre_ok && post_ok
}

fn generate_commit_plan_ref(_input: &AdmissionChainInput) -> String {
    // In a real implementation, this would hash the chain result.
    // For now, return a placeholder indicative value — the pure core
    // generates a deterministic plan ref from the admission input.
    let plan_value = record("resource-commit-plan-v1", vec![
        string("admission-pass"),
        string(&_input.resource_ref),
        string(&_input.candidate_ref),
    ]);
    canonical_hash(&plan_value).unwrap_or_else(|_| "blake3:commit-plan-hash-error".to_string())
}

// ---------------------------------------------------------------------------
// Status isolation
// ---------------------------------------------------------------------------

/// Validate that a status operation only changes permitted fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusOperationInput {
    pub current_generation: u64,
    pub proposed_generation: u64,
    pub changes_desired_ref: bool,
    pub changes_desired_generation: bool,
    pub changes_finalizers: bool,
    pub changes_authority_metadata: bool,
    pub has_status_condition_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusOperationDecision {
    pub pass: bool,
    pub diagnostics: Vec<String>,
}

/// Validate that a status operation respects status subresource isolation.
///
/// A status operation may update observed-state refs and status conditions
/// for an observed generation, but MUST NOT advance desired generation,
/// change desired-state refs, alter finalizers, or alter authority-bearing
/// metadata.
pub fn validate_status_operation(input: &StatusOperationInput) -> StatusOperationDecision {
    let mut diagnostics = Vec::new();
    let mut pass = true;

    if input.changes_desired_ref {
        pass = false;
        diagnostics.push("status operation cannot change desired-state ref".to_string());
    }
    if input.changes_desired_generation && input.proposed_generation != input.current_generation {
        pass = false;
        diagnostics.push("status operation cannot advance desired generation".to_string());
    }
    if input.changes_finalizers {
        pass = false;
        diagnostics.push("status operation cannot alter finalizers".to_string());
    }
    if input.changes_authority_metadata {
        pass = false;
        diagnostics.push("status operation cannot alter authority-bearing metadata".to_string());
    }
    if !input.has_status_condition_evidence {
        pass = false;
        diagnostics.push("status operation must have condition evidence".to_string());
    }

    StatusOperationDecision { pass, diagnostics }
}

// ---------------------------------------------------------------------------
// Preserves encoding helpers
// ---------------------------------------------------------------------------

pub fn admission_chain_result_to_value(result: &AdmissionChainResult) -> IoValue {
    let phase_values: Vec<IoValue> = result
        .phase_results
        .iter()
        .map(|phase| {
            record("admission-phase", vec![
                symbol(phase.phase.as_str()),
                symbol(phase.decision.as_str()),
                refs_sequence(&phase.evidence_refs),
                diagnostics_sequence(&phase.diagnostics),
            ])
        })
        .collect();

    record("resource-admission-receipt-v1", vec![
        symbol(result.operation.as_str()),
        bool_value(result.pass),
        record("phases", vec![sequence(phase_values)]),
        optional_ref_value(result.commit_plan_ref.as_deref()),
        diagnostics_sequence(&result.diagnostics),
    ])
}

fn diagnostics_sequence(diagnostics: &[String]) -> IoValue {
    let values: Vec<IoValue> = diagnostics.iter().map(string).collect();
    record("diagnostics", vec![sequence(values)])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// Tests moved to p003/body.rs to avoid duplicate `mod tests`