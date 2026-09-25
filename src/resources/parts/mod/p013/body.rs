
fn unpassed_phase(phase: AdmissionPhase, decision: PhaseDecision, diagnostic: &str) -> PhaseResult {
    PhaseResult {
        phase,
        decision,
        evidence_refs: Vec::new(),
        diagnostics: vec![diagnostic.to_string()],
    }
}

/// A pass carrying the phase's evidence refs, or a denial naming the missing evidence.
fn evidence_phase(phase: AdmissionPhase, evidence: Option<&PhaseEvidence>, missing: &str) -> PhaseResult {
    match evidence {
        Some(evidence) => passed_phase(phase, evidence.evidence_refs.clone()),
        None => unpassed_phase(phase, PhaseDecision::Deny, missing),
    }
}

/// Mutation evidence is required for create and update operations, and the
/// phase is skipped for every other operation.
fn reviewed_mutation_result(phase: AdmissionPhase, input: &AdmissionChainInput) -> PhaseResult {
    let is_mutating = input.operation == ResourceOperation::Create || input.operation == ResourceOperation::Update;
    if !is_mutating {
        return PhaseResult {
            phase,
            decision: PhaseDecision::Skip,
            evidence_refs: Vec::new(),
            diagnostics: vec![format!("mutation phase skipped for {:?} operation", input.operation)],
        };
    }
    let Some(evidence) = input.mutation_evidence.as_ref() else {
        return PhaseResult {
            phase,
            decision: PhaseDecision::Deny,
            evidence_refs: Vec::new(),
            diagnostics: vec!["missing mutation evidence for create/update operation".to_string()],
        };
    };
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
            diagnostics: vec!["mutation evidence has invalid refs or pre/post mismatch".to_string()],
        }
    }
}

fn validate_mutation_evidence(evidence: &MutationEvidence) -> bool {
    // All refs must be valid content refs
    let is_rule_ok = validate_content_ref(&evidence.rule_ref).is_ok();
    let is_pre_ok = validate_content_ref(&evidence.pre_mutation_ref).is_ok();
    let is_post_ok = validate_content_ref(&evidence.post_mutation_ref).is_ok();
    is_rule_ok && is_pre_ok && is_post_ok
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
    let mut should_pass = true;

    if input.changes_desired_ref {
        should_pass = false;
        diagnostics.push("status operation cannot change desired-state ref".to_string());
    }
    if input.changes_desired_generation && input.proposed_generation != input.current_generation {
        should_pass = false;
        diagnostics.push("status operation cannot advance desired generation".to_string());
    }
    if input.changes_finalizers {
        should_pass = false;
        diagnostics.push("status operation cannot alter finalizers".to_string());
    }
    if input.changes_authority_metadata {
        should_pass = false;
        diagnostics.push("status operation cannot alter authority-bearing metadata".to_string());
    }
    if !input.has_status_condition_evidence {
        should_pass = false;
        diagnostics.push("status operation must have condition evidence".to_string());
    }

    StatusOperationDecision { pass: should_pass, diagnostics }
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