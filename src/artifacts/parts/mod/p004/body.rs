
fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn pass_fail(is_pass: bool) -> &'static str {
    if is_pass { "pass" } else { "fail" }
}

fn release_snapshot_dependency_index_digest(
    root: &CapabilityArtifactRoot,
    artifact_refs: &[String],
) -> Result<String> {
    let mut edges = Vec::new();
    for artifact_ref in sorted_unique(artifact_refs) {
        let artifact = read_artifact_with_root(root, &artifact_ref)?;
        extend_bounded(
            &mut edges,
            dependency_edges_for_artifact(&artifact)?,
            MAX_ARTIFACT_RECORDS,
            "release snapshot dependency edges",
        )?;
    }
    dependency_index_digest(&edges)
}

fn release_snapshot_install_evidence_refs(input: &ReleaseSnapshotValueInput) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    extend_cloned_bounded(&mut refs, &input.doc_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.transcript_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(
        &mut refs,
        &input.expected_receipt_refs,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    extend_cloned_bounded(&mut refs, &input.provenance_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.source_gate_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.resource_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.compatibility_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.migration_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(
        &mut refs,
        &input.upgrade_session_refs,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    extend_cloned_bounded(&mut refs, &input.rollback_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.cutover_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(&mut refs, &input.signature_refs, MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    extend_cloned_bounded(
        &mut refs,
        &input.stale_evidence_refs,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    push_bounded(
        &mut refs,
        input.dependency_closure_digest.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    push_bounded(
        &mut refs,
        input.dependency_index_ref.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    push_bounded(
        &mut refs,
        input.signature_subject_ref.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot evidence refs",
    )?;
    if let Some(artifact_set_ref) = input.artifact_set_ref.as_ref() {
        push_bounded(&mut refs, artifact_set_ref.clone(), MAX_ARTIFACT_REF_LIST, "release snapshot evidence refs")?;
    }
    if let Some(redaction_profile_ref) = input.redaction_profile_ref.as_ref() {
        push_bounded(
            &mut refs,
            redaction_profile_ref.clone(),
            MAX_ARTIFACT_REF_LIST,
            "release snapshot evidence refs",
        )?;
    }
    Ok(sorted_unique(&refs))
}

fn release_snapshot_verify_refs(snapshot_artifact_ref: &str, snapshot: &ReleaseSnapshot) -> Result<Vec<String>> {
    validate_ref(snapshot_artifact_ref, "release snapshot artifact ref")?;
    let mut refs = Vec::new();
    push_bounded(
        &mut refs,
        snapshot_artifact_ref.to_string(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot verify refs",
    )?;
    push_bounded(
        &mut refs,
        snapshot.snapshot_ref.clone(),
        MAX_ARTIFACT_REF_LIST,
        "release snapshot verify refs",
    )?;
    extend_cloned_bounded(&mut refs, &snapshot.artifact_refs, MAX_ARTIFACT_REF_LIST, "release snapshot verify refs")?;
    extend_cloned_bounded(
        &mut refs,
        &release_snapshot_install_evidence_refs(&ReleaseSnapshotValueInput {
            namespace_scope: snapshot.namespace_scope.clone(),
            snapshot_id: snapshot.snapshot_id.clone(),
            artifact_refs: snapshot.artifact_refs.clone(),
            artifact_set_ref: snapshot.artifact_set_ref.clone(),
            dependency_closure_digest: snapshot.dependency_closure_digest.clone(),
            dependency_index_ref: snapshot.dependency_index_ref.clone(),
            doc_refs: snapshot.doc_refs.clone(),
            transcript_refs: snapshot.transcript_refs.clone(),
            expected_receipt_refs: snapshot.expected_receipt_refs.clone(),
            policy_refs: snapshot.policy_refs.clone(),
            provenance_refs: snapshot.provenance_refs.clone(),
            source_gate_refs: snapshot.source_gate_refs.clone(),
            resource_refs: snapshot.resource_refs.clone(),
            compatibility_refs: snapshot.compatibility_refs.clone(),
            migration_refs: snapshot.migration_refs.clone(),
            upgrade_session_refs: snapshot.upgrade_session_refs.clone(),
            rollback_refs: snapshot.rollback_refs.clone(),
            cutover_refs: snapshot.cutover_refs.clone(),
            caveats: snapshot.caveats.clone(),
            non_claims: snapshot.non_claims.clone(),
            redaction_profile_ref: snapshot.redaction_profile_ref.clone(),
            signature_subject_ref: snapshot.signature_subject_ref.clone(),
            signature_refs: snapshot.signature_refs.clone(),
            stale_evidence_refs: snapshot.stale_evidence_refs.clone(),
        })?,
        MAX_ARTIFACT_REF_LIST,
        "release snapshot verify refs",
    )?;
    extend_cloned_bounded(&mut refs, &snapshot.policy_refs, MAX_ARTIFACT_REF_LIST, "release snapshot verify refs")?;
    Ok(sorted_unique(&refs))
}

fn set_difference(left: &[String], right: &[String]) -> Result<Vec<String>> {
    let right_set = right.iter().collect::<std::collections::BTreeSet<_>>();
    let mut difference = Vec::new();
    for item in left {
        if !right_set.contains(item) {
            push_bounded(&mut difference, item.clone(), MAX_ARTIFACT_REF_LIST, "release snapshot set difference")?;
        }
    }
    Ok(difference)
}

fn release_snapshot_caveats_rendered(
    snapshot: &ReleaseSnapshot,
    required_caveats: &[String],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<bool> {
    let mut is_rendered = true;
    if snapshot.caveats.is_empty() {
        is_rendered = false;
        push_bounded(
            diagnostics,
            "release snapshot must render at least one caveat".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
    }
    for required in required_caveats {
        if !snapshot.caveats.iter().any(|caveat| caveat == required) {
            is_rendered = false;
            push_bounded(
                diagnostics,
                format!("required caveat not rendered: {required}"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "release snapshot diagnostics",
            )?;
        }
    }
    Ok(is_rendered)
}

fn release_snapshot_fresh_evidence(snapshot: &ReleaseSnapshot, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<bool> {
    for stale_ref in &snapshot.stale_evidence_refs {
        push_bounded(
            diagnostics,
            format!("stale evidence ref {stale_ref} prevents release snapshot pass evidence"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
    }
    Ok(snapshot.stale_evidence_refs.is_empty())
}

fn release_snapshot_redaction_bound(snapshot: &ReleaseSnapshot, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<bool> {
    if snapshot.redaction_profile_ref.is_some()
        && !snapshot.caveats.iter().any(|caveat| caveat.contains("redaction") || caveat.contains("redacted"))
    {
        push_bounded(
            diagnostics,
            "redaction profile is bound but no redaction caveat is rendered".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
        return Ok(false);
    }
    Ok(true)
}

fn release_snapshot_required_evidence_bound(
    snapshot: &ReleaseSnapshot,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<bool> {
    let mut is_bound = true;
    is_bound &= require_non_empty_refs(&snapshot.doc_refs, "release snapshot docs", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.transcript_refs, "release snapshot transcripts", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.expected_receipt_refs, "release snapshot expected receipts", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.policy_refs, "release snapshot policy evidence", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.provenance_refs, "release snapshot provenance evidence", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.source_gate_refs, "release snapshot source-gate evidence", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.resource_refs, "release snapshot resource evidence", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.compatibility_refs, "release snapshot compatibility receipts", diagnostics)?;
    is_bound &= require_non_empty_refs(&snapshot.migration_refs, "release snapshot migration receipts", diagnostics)?;
    Ok(is_bound)
}

fn require_non_empty_refs(refs: &[String], label: &str, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<bool> {
    if refs.is_empty() {
        push_bounded(
            diagnostics,
            format!("{label} must be bound"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "release snapshot diagnostics",
        )?;
        Ok(false)
    } else {
        Ok(true)
    }
}

fn release_channel_update_refs(input: &ReleaseChannelUpdateInput, pointer: &ArtifactNamePointer) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    push_bounded(&mut refs, input.snapshot_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    push_bounded(&mut refs, pointer.pointer_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    push_bounded(&mut refs, pointer.receipt_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    if let Some(previous_ref) = pointer.previous_ref.as_ref() {
        push_bounded(&mut refs, previous_ref.clone(), MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    }
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    extend_cloned_bounded(&mut refs, &input.evidence_refs, MAX_ARTIFACT_REF_LIST, "release channel update refs")?;
    Ok(sorted_unique(&refs))
}

fn release_channel_admission_refs(input: &ReleaseChannelAdmissionInput) -> Result<Vec<String>> {
    let mut refs = vec![input.channel_pointer_ref.clone()];
    extend_cloned_bounded(
        &mut refs,
        &input.release_evidence_refs,
        MAX_ARTIFACT_REF_LIST,
        "release channel admission refs",
    )?;
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.provenance_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.source_gate_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.authority_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    extend_cloned_bounded(&mut refs, &input.resource_refs, MAX_ARTIFACT_REF_LIST, "release channel admission refs")?;
    Ok(sorted_unique(&refs))
}
