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
    let value = context_expansion_value(
        &profile_artifact.profile_ref,
        requirements,
        overrides,
        &expanded_refs,
        decision,
        &diagnostics,
    )?;
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

fn validate_profile(input: &ContextProfileInput, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_text("context profile id", &input.profile_id)?;
    validate_profile_tier(&input.profile_tier, diagnostics)?;
    validate_ref_set(&input.refs, diagnostics)?;
    ensure_scope_bound(input.allowed_operations.len(), "allowed operations")?;
    let mut seen = OrderedSet::new();
    for operation in &input.allowed_operations {
        validate_text("allowed operation", operation)?;
        if !seen.insert(operation.clone()) {
            diagnostics.push(format!("duplicate-operation-scope:{operation}"));
        }
    }
    ensure_caveat_bound(input.caveats.len(), "context caveats")?;
    for caveat in &input.caveats {
        validate_text("context caveat", caveat)?;
    }
    Ok(())
}

fn validate_profile_tier(tier: &str, diagnostics: &mut Vec<String>) -> Result<()> {
    match tier {
        "local" | "pilot" | "release" => Ok(()),
        other => {
            diagnostics.push(format!("unsupported-profile-tier:{other}"));
            Ok(())
        }
    }
}

fn validate_ref_set(refs: &ContextRefSet, diagnostics: &mut Vec<String>) -> Result<()> {
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

fn validate_overrides(overrides: &ContextOverrideInput, diagnostics: &mut Vec<String>) -> Result<()> {
    validate_ref_list_with_diagnostics("override policy", &overrides.policy_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override authority", &overrides.authority_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override resource", &overrides.resource_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override evidence", &overrides.evidence_refs, diagnostics)?;
    validate_ref_list_with_diagnostics("override retention", &overrides.retention_refs, diagnostics)
}

fn merge_refs(
    profile: &ContextProfileInput,
    overrides: &ContextOverrideInput,
    diagnostics: &mut Vec<String>,
) -> Result<ContextRefSet> {
    if !overrides.policy_refs.is_empty() && !same_ref_set(&profile.refs.policy_refs, &overrides.policy_refs) {
        diagnostics.push("conflicting-policy-override".to_string());
    }
    if !overrides.authority_refs.is_empty() && !same_ref_set(&profile.refs.authority_refs, &overrides.authority_refs) {
        diagnostics.push("conflicting-authority-override".to_string());
    }
    if !overrides.resource_refs.is_empty() && !same_ref_set(&profile.refs.resource_refs, &overrides.resource_refs) {
        diagnostics.push("conflicting-resource-override".to_string());
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

fn validate_required_refs(requirements: &OperationRequirements, refs: &ContextRefSet, diagnostics: &mut Vec<String>) {
    if requirements.require_policy && refs.policy_refs.is_empty() {
        diagnostics.push(format!("missing-required-policy:{}", requirements.operation));
    }
    if requirements.require_authority && refs.authority_refs.is_empty() {
        diagnostics.push(format!("missing-required-authority:{}", requirements.operation));
    }
    if requirements.require_resource && refs.resource_refs.is_empty() {
        diagnostics.push(format!("missing-required-resource:{}", requirements.operation));
    }
    if requirements.require_evidence && refs.evidence_refs.is_empty() {
        diagnostics.push(format!("missing-required-evidence:{}", requirements.operation));
    }
    if requirements.require_retention && refs.retention_refs.is_empty() {
        diagnostics.push(format!("missing-required-retention:{}", requirements.operation));
    }
}

fn context_profile_value(input: &ContextProfileInput, decision: &str, diagnostics: &[String]) -> Result<IoValue> {
    Ok(record("context-profile-v1", vec![
        string(CONTEXT_PROFILE_SCHEMA),
        field_string("decision", decision),
        field_string("profile-id", &input.profile_id),
        field_string("profile-tier", &input.profile_tier),
        ref_set_value("refs", &input.refs)?,
        field_sequence("allowed-operations", string_values(&input.allowed_operations)?),
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&context_caveats(&input.caveats))?),
    ]))
}

fn context_expansion_value(
    profile_ref: &str,
    requirements: &OperationRequirements,
    overrides: &ContextOverrideInput,
    expanded_refs: &ContextRefSet,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("context-profile-expansion-v1", vec![
        string(CONTEXT_EXPANSION_SCHEMA),
        field_string("decision", decision),
        field_string("profile-ref", profile_ref),
        field_string("operation", &requirements.operation),
        requirement_value(requirements),
        ref_set_value("overrides", &override_ref_set(overrides))?,
        ref_set_value("expanded-refs", expanded_refs)?,
        field_sequence("diagnostics", string_values(diagnostics)?),
        field_sequence("caveats", string_values(&[EVIDENCE_ONLY_CAVEAT.to_string()])?),
    ]))
}

fn requirement_value(requirements: &OperationRequirements) -> IoValue {
    record("requirements", vec![
        record("policy", vec![bool_value(requirements.require_policy)]),
        record("authority", vec![bool_value(requirements.require_authority)]),
        record("resource", vec![bool_value(requirements.require_resource)]),
        record("evidence", vec![bool_value(requirements.require_evidence)]),
        record("retention", vec![bool_value(requirements.require_retention)]),
    ])
}

fn override_ref_set(overrides: &ContextOverrideInput) -> ContextRefSet {
    ContextRefSet {
        policy_refs: overrides.policy_refs.clone(),
        capability_refs: Vec::new(),
        authority_refs: overrides.authority_refs.clone(),
        resource_refs: overrides.resource_refs.clone(),
        evidence_refs: overrides.evidence_refs.clone(),
        redaction_refs: Vec::new(),
        retention_refs: overrides.retention_refs.clone(),
    }
}

fn context_caveats(caveats: &[String]) -> Vec<String> {
    let mut output = caveats.to_vec();
    output.push(EVIDENCE_ONLY_CAVEAT.to_string());
    output
}

fn ref_set_value(label: &'static str, refs: &ContextRefSet) -> Result<IoValue> {
    Ok(record(label, vec![
        field_sequence("policy", string_values(&refs.policy_refs)?),
        field_sequence("capability", string_values(&refs.capability_refs)?),
        field_sequence("authority", string_values(&refs.authority_refs)?),
        field_sequence("resource", string_values(&refs.resource_refs)?),
        field_sequence("evidence", string_values(&refs.evidence_refs)?),
        field_sequence("redaction", string_values(&refs.redaction_refs)?),
        field_sequence("retention", string_values(&refs.retention_refs)?),
    ]))
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} ref {reference}: {error}")))
}

fn validate_ref_list(label: &str, refs: &[String]) -> Result<()> {
    ensure_ref_bound(refs.len(), label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref_list_with_diagnostics(label: &str, refs: &[String], diagnostics: &mut Vec<String>) -> Result<()> {
    ensure_ref_bound(refs.len(), label)?;
    for reference in refs {
        if let Err(error) = validate_ref(reference, label) {
            diagnostics.push(format!("stale-ref:{label}:{reference}:{error}"));
        }
    }
    Ok(())
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_ref_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_REFS, label)
}

fn ensure_scope_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_SCOPES, label)
}

fn ensure_caveat_bound(count: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_CAVEATS, label)
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "context diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn refs() -> ContextRefSet {
        ContextRefSet {
            policy_refs: vec![local_ref("policy")],
            capability_refs: vec![local_ref("capability")],
            authority_refs: vec![local_ref("authority")],
            resource_refs: vec![local_ref("resource")],
            evidence_refs: vec![local_ref("evidence")],
            redaction_refs: vec![local_ref("redaction")],
            retention_refs: vec![local_ref("retention")],
        }
    }

    fn profile() -> ContextProfileInput {
        ContextProfileInput {
            profile_id: "operator:node-control".to_string(),
            profile_tier: "pilot".to_string(),
            refs: refs(),
            allowed_operations: vec!["node.status".to_string(), "node.install".to_string()],
            caveats: vec!["pilot only".to_string()],
        }
    }

    fn requirements(operation: &str) -> OperationRequirements {
        OperationRequirements {
            operation: operation.to_string(),
            require_policy: true,
            require_authority: true,
            require_resource: true,
            require_evidence: true,
            require_retention: false,
        }
    }

    fn empty_overrides() -> ContextOverrideInput {
        ContextOverrideInput {
            policy_refs: Vec::new(),
            authority_refs: Vec::new(),
            resource_refs: Vec::new(),
            evidence_refs: Vec::new(),
            retention_refs: Vec::new(),
        }
    }

    // r[verify molten.operator_workflow.context_profile.artifact]
    // r[verify molten.operator_workflow.context_profile.expansion]
    // r[verify molten.operator_workflow.context_profile.overrides]
    // r[verify molten.operator_workflow.context_profile.evidence_only]
    #[test]
    fn context_profile_artifact_and_expansion_pass_for_valid_refs() {
        let artifact = build_context_profile_artifact(&profile()).expect("profile artifact");
        assert_eq!(artifact.decision, DECISION_PASS);
        let expansion = expand_context_profile(&profile(), &requirements("node.status"), &empty_overrides())
            .expect("context expansion");
        assert_eq!(expansion.decision, DECISION_PASS);
        assert_eq!(expansion.expanded_refs.policy_refs, refs().policy_refs);
    }

    #[test]
    fn context_profile_denies_malformed_refs_and_unsupported_scope() {
        let mut profile = profile();
        profile.refs.authority_refs = vec!["not-a-ref".to_string()];
        let expansion = expand_context_profile(&profile, &requirements("retention.delete"), &empty_overrides())
            .expect("context expansion");
        assert_eq!(expansion.decision, DECISION_DENY);
        assert!(
            expansion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("stale-ref:authority:not-a-ref"))
        );
        assert!(
            expansion
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "unsupported-operation-scope:retention.delete")
        );
    }

    #[test]
    fn context_profile_allows_additive_evidence_but_denies_conflicting_authority() {
        let mut overrides = empty_overrides();
        overrides.evidence_refs = vec![local_ref("extra-evidence")];
        let expansion =
            expand_context_profile(&profile(), &requirements("node.install"), &overrides).expect("context expansion");
        assert_eq!(expansion.decision, DECISION_PASS);
        assert!(expansion.expanded_refs.evidence_refs.contains(&local_ref("extra-evidence")));

        let mut conflicting = empty_overrides();
        conflicting.authority_refs = vec![local_ref("other-authority")];
        let denied = expand_context_profile(&profile(), &requirements("node.install"), &conflicting)
            .expect("denied context expansion");
        assert_eq!(denied.decision, DECISION_DENY);
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic == "conflicting-authority-override"));
    }

    #[test]
    fn context_profile_presence_cannot_authorize_mutation_by_itself() {
        let artifact = build_context_profile_artifact(&profile()).expect("profile artifact");
        let decision = evaluate_context_profile_authorization_use(&artifact.profile_ref, "node.install", &[])
            .expect("authorization use");
        assert_eq!(decision.decision, DECISION_DENY);
        assert!(decision.diagnostics.iter().any(|diagnostic| diagnostic == "context-profile-is-not-authority"));
        assert!(
            decision
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "missing-expanded-authority:node.install")
        );
    }
}
