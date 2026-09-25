type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;
type OrderedSet<T> = std::collections::BTreeSet<T>;

const CONTEXT_PROFILE_SCHEMA: &str = "molten.operator.context-profile.v1";
const CONTEXT_EXPANSION_SCHEMA: &str = "molten.operator.context-profile-expansion.v1";
const CONTEXT_AUTHORIZATION_SCHEMA: &str = "molten.operator.context-profile-authorization-use.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const MAX_REFS: usize = 256;
const MAX_SCOPES: usize = 128;
const MAX_CAVEATS: usize = 128;
const MAX_DIAGNOSTICS: usize = 4096;
const EVIDENCE_ONLY_CAVEAT: &str = "operator context profiles are convenience and review evidence only; expanded refs must still pass subsystem authority, policy, resource, provenance, retention, source-gate, transport, mutation, and release gates";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextRefSet {
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub redaction_refs: Vec<String>,
    pub retention_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextProfileInput {
    pub profile_id: String,
    pub profile_tier: String,
    pub refs: ContextRefSet,
    pub allowed_operations: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRequirements {
    pub operation: String,
    pub require_policy: bool,
    pub require_authority: bool,
    pub require_resource: bool,
    pub require_evidence: bool,
    pub require_retention: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextOverrideInput {
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub retention_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextProfileArtifact {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub profile_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextExpansion {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub profile_ref: String,
    pub expanded_refs: ContextRefSet,
    pub expansion_ref: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextAuthorizationUse {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

// r[impl molten.operator_workflow.context_profile.artifact]
// r[impl molten.operator_workflow.context_profile.expansion]
// r[impl molten.operator_workflow.context_profile.overrides]
// r[impl molten.operator_workflow.context_profile.evidence_only]
pub fn build_context_profile_artifact(input: &ContextProfileInput) -> Result<ContextProfileArtifact> {
    let mut diagnostics = Vec::new();
    validate_profile(input, &mut diagnostics)?;
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = context_profile_value(input, decision, &diagnostics)?;
    let profile_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ContextProfileArtifact {
        decision: decision.to_string(),
        diagnostics,
        profile_ref,
        value,
    })
}

pub fn expand_context_profile(
    profile: &ContextProfileInput,
    requirements: &OperationRequirements,
    overrides: &ContextOverrideInput,
) -> Result<ContextExpansion> {
    let profile_artifact = build_context_profile_artifact(profile)?;
    let mut diagnostics = profile_artifact.diagnostics.clone();
    validate_requirements(requirements)?;
    validate_overrides(overrides, &mut diagnostics)?;
    if !profile.allowed_operations.iter().any(|operation| operation == &requirements.operation) {
        diagnostics.push(format!("unsupported-operation-scope:{}", requirements.operation));
    }
    let expanded_refs = merge_refs(profile, overrides, &mut diagnostics)?;
    validate_required_refs(requirements, &expanded_refs, &mut diagnostics);
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let decision = if diagnostics.is_empty() {
        DECISION_PASS
    } else {
        DECISION_DENY
    };
    let value = context_expansion_value(ExpansionValueInput {
        profile_ref: &profile_artifact.profile_ref,
        requirements,
        overrides,
        expanded_refs: &expanded_refs,
        decision,
        diagnostics: &diagnostics,
    })?;
    let expansion_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(ContextExpansion {
        decision: decision.to_string(),
        diagnostics,
        profile_ref: profile_artifact.profile_ref,
        expanded_refs,
        expansion_ref,
        value,
    })
}

pub fn evaluate_context_profile_authorization_use(
    profile_ref: &str,
    requested_operation: &str,
    expanded_authority_refs: &[String],
) -> Result<ContextAuthorizationUse> {
    validate_ref(profile_ref, "context profile")?;
    validate_text("requested operation", requested_operation)?;
    validate_ref_list("expanded authority", expanded_authority_refs)?;
    let mut diagnostics = vec!["context-profile-is-not-authority".to_string()];
    if expanded_authority_refs.is_empty() {
        diagnostics.push(format!("missing-expanded-authority:{requested_operation}"));
    }
    let value = record("context-profile-authorization-use-v1", vec![
        string(CONTEXT_AUTHORIZATION_SCHEMA),
        field_string("decision", DECISION_DENY),
        field_string("profile-ref", profile_ref),
        field_string("requested-operation", requested_operation),
        field_sequence("expanded-authority-refs", string_values(expanded_authority_refs)?),
        field_sequence("diagnostics", string_values(&diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]);
    Ok(ContextAuthorizationUse {
        decision: DECISION_DENY.to_string(),
        diagnostics,
        value,
    })
}

fn validate_profile(input: &ContextProfileInput, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    validate_text("context profile id", &input.profile_id)?;
    validate_profile_tier(&input.profile_tier, diagnostics)?;
    validate_ref_set(&input.refs, diagnostics)?;
    ensure_scope_bound(input.allowed_operations.len(), "allowed operations")?;
    let mut seen = OrderedSet::new();
    for operation in &input.allowed_operations {
        validate_text("allowed operation", operation)?;
        if !seen.insert(operation.clone()) {
            diagnostics.push_item(format!("duplicate-operation-scope:{operation}"));
        }
    }
    ensure_caveat_bound(input.caveats.len(), "context caveats")?;
    for caveat in &input.caveats {
        validate_text("context caveat", caveat)?;
    }
    Ok(())
}

fn validate_profile_tier(tier: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    match tier {
        "local" | "pilot" | "release" => Ok(()),
        other => {
            diagnostics.push_item(format!("unsupported-profile-tier:{other}"));
            Ok(())
        }
    }
}

fn validate_ref_set(refs: &ContextRefSet, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<()> {
    validate_ref_list_with_diagnostics("policy", &refs.policy_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("capability", &refs.capability_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("authority", &refs.authority_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("resource", &refs.resource_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("evidence", &refs.evidence_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("redaction", &refs.redaction_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("retention", &refs.retention_refs, diagnostics)
}

fn validate_requirements(requirements: &OperationRequirements) -> Result<()> {
    validate_text("operation", &requirements.operation)
}

fn validate_overrides(
    overrides: &ContextOverrideInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<()> {
    validate_ref_list_with_diagnostics("override policy", &overrides.policy_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override authority", &overrides.authority_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override resource", &overrides.resource_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override evidence", &overrides.evidence_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override retention", &overrides.retention_refs, diagnostics)
}

fn merge_refs(
    profile: &ContextProfileInput,
    overrides: &ContextOverrideInput,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<ContextRefSet> {
    if !overrides.policy_refs.is_empty() && !same_ref_set(&profile.refs.policy_refs, &overrides.policy_refs) {
        diagnostics.push_item("conflicting-policy-override".to_string());
    }
    if !overrides.authority_refs.is_empty() && !same_ref_set(&profile.refs.authority_refs, &overrides.authority_refs) {
        diagnostics.push_item("conflicting-authority-override".to_string());
    }
    if !overrides.resource_refs.is_empty() && !same_ref_set(&profile.refs.resource_refs, &overrides.resource_refs) {
        diagnostics.push_item("conflicting-resource-override".to_string());
    }
    Ok(ContextRefSet {
        policy_refs: merge_same_or_profile(&profile.refs.policy_refs, &overrides.policy_refs),
        capability_refs: profile.refs.capability_refs.clone(),
        authority_refs: merge_same_or_profile(&profile.refs.authority_refs, &overrides.authority_refs),
        resource_refs: merge_same_or_profile(&profile.refs.resource_refs, &overrides.resource_refs),
        evidence_refs: merge_additive(&profile.refs.evidence_refs, &overrides.evidence_refs),
        redaction_refs: profile.refs.redaction_refs.clone(),
        retention_refs: merge_same_or_profile(&profile.refs.retention_refs, &overrides.retention_refs),
    })
}

fn merge_same_or_profile(profile_refs: &[String], override_refs: &[String]) -> Vec<String> {
    if override_refs.is_empty() || same_ref_set(profile_refs, override_refs) {
        return profile_refs.to_vec();
    }
    override_refs.to_vec()
}

fn merge_additive(profile_refs: &[String], override_refs: &[String]) -> Vec<String> {
    let mut refs = OrderedSet::new();
    refs.extend(profile_refs.iter().cloned());
    refs.extend(override_refs.iter().cloned());
    refs.into_iter().collect()
}

fn same_ref_set(left: &[String], right: &[String]) -> bool {
    left.iter().collect::<OrderedSet<_>>() == right.iter().collect::<OrderedSet<_>>()
}
