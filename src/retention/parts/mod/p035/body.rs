
pub fn admit_destructive_evidence(input: DestructiveAdmissionInput<'_>) -> Result<DestructiveAdmission> {
    let root = open_capability_retention_root(input.root)?;
    admit_destructive_evidence_with_root(DestructiveAdmissionInput {
        root: &root,
        evidence: input.evidence,
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
    })
}

pub fn admit_destructive_evidence_with_root(
    input: DestructiveAdmissionInput<'_, CapabilityRetentionRoot>,
) -> Result<DestructiveAdmission> {
    ensure_store_with_root(input.root)?;
    validate_destructive_evidence(input.evidence)?;
    require_ref(input.object_ref, "retention admission object ref")?;
    validate_name(input.object_kind, "retention admission object kind")?;
    validate_class(input.retention_class)?;
    validate_action(input.action)?;
    let mut diagnostics = destructive_evidence_diagnostics(input.evidence, input.action)?;
    let mut admitted_refs = Vec::new();
    let scope = AdmissionScope {
        requester_ref: input.evidence.requester_ref.as_deref(),
        object_ref: input.object_ref,
        object_kind: input.object_kind,
        retention_class: input.retention_class,
        action: input.action,
    };
    let set = admit_set_with_root(input.root, input.evidence, &scope)?;
    let flags = admit_flags(input.evidence, &set);
    collect_admit_outputs(&mut diagnostics, &mut admitted_refs, set)?;
    let has_delete_authority = is_destructive_action(input.action)
        && flags.has_authority
        && flags.has_policy
        && flags.has_supporting
        && (!input.evidence.is_reference_index_complete || flags.has_reference_index)
        && flags.has_remote_refs;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(DestructiveAdmission {
        decision: decision.to_string(),
        diagnostics,
        admitted_refs,
        has_delete_authority,
        has_remote_gc_clearance: flags.has_remote_refs,
    })
}

pub fn destructive_requester_ref(input: &DestructiveEvidence, fallback_label: &str) -> Result<String> {
    validate_destructive_evidence(input)?;
    if let Some(requester_ref) = input.requester_ref.as_ref() {
        Ok(requester_ref.clone())
    } else {
        synthetic_ref(fallback_label)
    }
}

pub fn destructive_has_authority(input: &DestructiveEvidence) -> bool {
    input.requester_ref.is_some() && !input.authority_refs.is_empty()
}
