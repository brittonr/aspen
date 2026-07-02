
fn check_bindings<S>(
    input: &RemoteClearanceRefsInput<'_>,
    clearance: &RemoteGcClearance,
    diagnostics: &mut S,
) -> Result<bool>
where
    S: VecSink<String>,
{
    let mut is_admitted = true;
    if !input.policy_refs.iter().any(|policy_ref| policy_ref == &clearance.policy_ref) {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-policy-mismatch:{}", clearance.remote_ref))?;
    }
    if !input.authority_refs.iter().any(|authority_ref| authority_ref == &clearance.authority_ref) {
        is_admitted = false;
        push_clear_note(diagnostics, format!("remote-clearance-authority-mismatch:{}", clearance.remote_ref))?;
    }
    Ok(is_admitted)
}

fn check_clear_ref<S>(
    input: &RemoteClearanceRefsInput<'_>,
    reference: &str,
    clearance: &RemoteGcClearance,
    diagnostics: &mut S,
) -> Result<Check>
where
    S: VecSink<String>,
{
    let is_state_admitted = check_state(reference, clearance, diagnostics)?;
    let scope = check_scope(input, clearance);
    let is_binding_admitted = check_bindings(input, clearance, diagnostics)?;
    Ok(Check {
        is_admitted: is_state_admitted && scope.is_admitted && is_binding_admitted,
        scope_mismatches: scope.scope_mismatches,
    })
}

fn collect_clear_refs(
    admitted_refs: &mut impl VecSink<String>,
    remote_refs: &mut impl VecSink<String>,
    peer_refs: &mut impl VecSink<String>,
    clearance: RemoteGcClearance,
) -> Result<()> {
    push_bounded(admitted_refs, clearance.clearance_ref, MAX_RETENTION_REFS, "retention remote clearance refs")?;
    push_bounded(remote_refs, clearance.remote_ref, MAX_RETENTION_REFS, "retention remote clearance remote refs")?;
    push_bounded(peer_refs, clearance.peer_ref, MAX_RETENTION_REFS, "retention remote clearance peer refs")
}

fn push_missing_clear_refs<S>(
    input: &RemoteClearanceRefsInput<'_>,
    remote_refs: &[String],
    peer_refs: &[String],
    diagnostics: &mut S,
) -> Result<()>
where
    S: VecSink<String>,
{
    for required in input.required_remote_refs {
        if !remote_refs.iter().any(|remote| remote == required) {
            push_clear_note(diagnostics, format!("remote-clearance-missing-remote:{}", required))?;
        }
    }
    for required in input.required_peer_refs {
        if !peer_refs.iter().any(|peer| peer == required) {
            push_clear_note(diagnostics, format!("remote-clearance-missing-peer:{}", required))?;
        }
    }
    Ok(())
}

fn admit_remote_clearance_refs(input: RemoteClearanceRefsInput<'_>) -> Result<RemoteClearanceRefsResult> {
    let mut diagnostics = Vec::new();
    let mut admitted_refs = Vec::new();
    let mut remote_refs = Vec::new();
    let mut peer_refs = Vec::new();
    let mut scope_mismatches = 0usize;
    for reference in input.refs {
        let clearance = match read_remote_gc_clearance(input.root, reference) {
            Ok(clearance) => clearance,
            Err(error) => {
                push_clear_note(&mut diagnostics, format!("remote-clearance-unreadable:{}:{}", reference, error))?;
                continue;
            }
        };
        let check = check_clear_ref(&input, reference, &clearance, &mut diagnostics)?;
        scope_mismatches += check.scope_mismatches;
        if check.is_admitted {
            collect_clear_refs(&mut admitted_refs, &mut remote_refs, &mut peer_refs, clearance)?;
        }
    }
    if !input.refs.is_empty() && admitted_refs.is_empty() && scope_mismatches > 0 {
        push_clear_note(&mut diagnostics, "remote-clearance-scope-mismatch".to_string())?;
    }
    push_missing_clear_refs(&input, &remote_refs, &peer_refs, &mut diagnostics)?;
    Ok(RemoteClearanceRefsResult {
        diagnostics,
        admitted_refs,
        remote_refs,
        peer_refs,
    })
}

fn read_evidence_admission(root: &Path, admission_ref: &str) -> Result<EvidenceAdmission> {
    require_ref(admission_ref, "retention evidence admission ref")?;
    let value = read_store_value(&admission_path(root, admission_ref)?)?;
    parse_evidence_admission(&value)
}

fn read_remote_gc_clearance(root: &Path, clearance_ref: &str) -> Result<RemoteGcClearance> {
    require_ref(clearance_ref, "retention remote GC clearance ref")?;
    let value = read_store_value(&remote_clearance_path(root, clearance_ref)?)?;
    parse_remote_gc_clearance(&value)
}

fn admit_refs<'a>(
    root: &'a Path,
    refs: &'a [String],
    expected_kind: &'a str,
    scope: &'a AdmissionScope<'a>,
    required_remote_refs: &'a [String],
) -> Result<AdmissionRefsResult> {
    admit_evidence_refs(AdmissionRefsInput {
        root,
        refs,
        expected_kind,
        scope,
        required_remote_refs,
    })
}

fn admit_clear_refs<'a>(
    root: &'a Path,
    evidence: &'a DestructiveEvidence,
    scope: &'a AdmissionScope<'a>,
) -> Result<RemoteClearanceRefsResult> {
    admit_remote_clearance_refs(RemoteClearanceRefsInput {
        root,
        refs: &evidence.remote_clearance_refs,
        scope,
        required_remote_refs: &evidence.remote_refs,
        required_peer_refs: &evidence.remote_peer_refs,
        policy_refs: &evidence.policy_refs,
        authority_refs: &evidence.authority_refs,
    })
}

struct AdmitSet {
    policy: AdmissionRefsResult,
    authority: AdmissionRefsResult,
    supporting: AdmissionRefsResult,
    reference_index: AdmissionRefsResult,
    remote_gc: AdmissionRefsResult,
    remote_clearance: RemoteClearanceRefsResult,
}

struct AdmitFlags {
    has_policy: bool,
    has_authority: bool,
    has_supporting: bool,
    has_reference_index: bool,
    has_remote_refs: bool,
}

fn admit_set<'a>(root: &'a Path, evidence: &'a DestructiveEvidence, scope: &'a AdmissionScope<'a>) -> Result<AdmitSet> {
    Ok(AdmitSet {
        policy: admit_refs(root, &evidence.policy_refs, ADMISSION_KIND_POLICY, scope, &[])?,
        authority: admit_refs(root, &evidence.authority_refs, ADMISSION_KIND_AUTHORITY, scope, &[])?,
        supporting: admit_refs(root, &evidence.evidence_refs, ADMISSION_KIND_SUPPORTING_EVIDENCE, scope, &[])?,
        reference_index: admit_refs(root, &evidence.reference_index_refs, ADMISSION_KIND_REFERENCE_INDEX, scope, &[])?,
        remote_gc: admit_refs(root, &evidence.remote_gc_refs, ADMISSION_KIND_REMOTE_GC, scope, &evidence.remote_refs)?,
        remote_clearance: admit_clear_refs(root, evidence, scope)?,
    })
}

fn has_required_remote_refs(
    evidence: &DestructiveEvidence,
    remote_gc: &AdmissionRefsResult,
    remote_clearance: &RemoteClearanceRefsResult,
) -> bool {
    let has_local_remote_gc_plan = evidence.remote_refs.is_empty()
        || evidence
            .remote_refs
            .iter()
            .all(|reference| remote_gc.remote_refs.iter().any(|remote| remote == reference));
    let has_remote_ref_clearance = evidence.remote_refs.is_empty()
        || evidence
            .remote_refs
            .iter()
            .all(|reference| remote_clearance.remote_refs.iter().any(|remote| remote == reference));
    let has_remote_peer_clearance = evidence.remote_peer_refs.is_empty()
        || evidence
            .remote_peer_refs
            .iter()
            .all(|peer| remote_clearance.peer_refs.iter().any(|cleared_peer| cleared_peer == peer));
    has_local_remote_gc_plan && has_remote_ref_clearance && has_remote_peer_clearance
}

fn admit_flags(evidence: &DestructiveEvidence, set: &AdmitSet) -> AdmitFlags {
    AdmitFlags {
        has_policy: !set.policy.admitted_refs.is_empty(),
        has_authority: !set.authority.admitted_refs.is_empty(),
        has_supporting: !set.supporting.admitted_refs.is_empty(),
        has_reference_index: !set.reference_index.admitted_refs.is_empty(),
        has_remote_refs: has_required_remote_refs(evidence, &set.remote_gc, &set.remote_clearance),
    }
}

fn push_admit_notes<S>(diagnostics: &mut S, notes: Vec<String>) -> Result<()>
where S: VecSink<String> {
    for diagnostic in notes {
        push_bounded(diagnostics, diagnostic, MAX_RETENTION_DIAGNOSTICS, "retention admission diagnostics")?;
    }
    Ok(())
}

fn push_admit_refs<S>(admitted_refs: &mut S, refs: Vec<String>) -> Result<()>
where S: VecSink<String> {
    for reference in refs {
        push_bounded(admitted_refs, reference, MAX_RETENTION_REFS, "retention admitted refs")?;
    }
    Ok(())
}

fn collect_admit_outputs<D, R>(diagnostics: &mut D, admitted_refs: &mut R, set: AdmitSet) -> Result<()>
where
    D: VecSink<String>,
    R: VecSink<String>,
{
    push_admit_notes(diagnostics, set.policy.diagnostics)?;
    push_admit_notes(diagnostics, set.authority.diagnostics)?;
    push_admit_notes(diagnostics, set.supporting.diagnostics)?;
    push_admit_notes(diagnostics, set.reference_index.diagnostics)?;
    push_admit_notes(diagnostics, set.remote_gc.diagnostics)?;
    push_admit_notes(diagnostics, set.remote_clearance.diagnostics)?;
    push_admit_refs(admitted_refs, set.policy.admitted_refs)?;
    push_admit_refs(admitted_refs, set.authority.admitted_refs)?;
    push_admit_refs(admitted_refs, set.supporting.admitted_refs)?;
    push_admit_refs(admitted_refs, set.reference_index.admitted_refs)?;
    push_admit_refs(admitted_refs, set.remote_gc.admitted_refs)?;
    push_admit_refs(admitted_refs, set.remote_clearance.admitted_refs)
}

pub fn admit_destructive_evidence(input: DestructiveAdmissionInput<'_>) -> Result<DestructiveAdmission> {
    ensure_store(input.root)?;
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
    let set = admit_set(input.root, input.evidence, &scope)?;
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
