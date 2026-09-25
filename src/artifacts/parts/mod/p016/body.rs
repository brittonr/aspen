
fn active_resolution_candidates(input: &ArtifactNameResolutionInput) -> Result<Vec<ArtifactNameView>> {
    let stale = input.stale_view_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let mut candidates = Vec::new();
    for view in &input.candidate_views {
        if view.view_kind != input.view_kind || view.name != input.name || view.tombstone_ref.is_some() {
            continue;
        }
        if input.scope.as_ref().is_some_and(|scope| &view.scope != scope) {
            continue;
        }
        if stale.contains(&view.view_ref) {
            continue;
        }
        push_bounded(&mut candidates, view.clone(), MAX_ARTIFACT_RECORDS, "artifact name resolution candidates")?;
    }
    candidates.sort_by(|left, right| left.view_ref.cmp(&right.view_ref));
    Ok(candidates)
}

fn name_resolution_diagnostics(
    input: &ArtifactNameResolutionInput,
    candidates: &[ArtifactNameView],
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    for stale_ref in &input.stale_view_refs {
        if input.candidate_views.iter().any(|view| &view.view_ref == stale_ref) {
            push_bounded(
                &mut diagnostics,
                format!("deny stale name view {stale_ref}"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact name resolution diagnostics",
            )?;
        }
    }
    match candidates.len() {
        0 => push_bounded(
            &mut diagnostics,
            "deny name resolution has no active exact-ref candidate".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name resolution diagnostics",
        )?,
        1 => {
            let scope = candidates[0].scope.clone();
            push_bounded(
                &mut diagnostics,
                format!("resolved exact artifact ref in scope {scope}; name views are non-authority"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact name resolution diagnostics",
            )?;
        }
        _ => {
            let refs = candidates.iter().map(|view| view.target_ref.clone()).collect::<Vec<_>>().join(",");
            push_bounded(
                &mut diagnostics,
                format!("deny ambiguous name resolution candidates: {refs}"),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact name resolution diagnostics",
            )?;
        }
    }
    if input.normative_use {
        push_bounded(
            &mut diagnostics,
            "normative use must pin the resolved exact artifact ref".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name resolution diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn name_use_diagnostics(input: &ArtifactNameUseInput) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();
    if input.exact_artifact_ref.is_none() {
        push_bounded(
            &mut diagnostics,
            "name-only use denies until exact artifact ref is pinned".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name use diagnostics",
        )?;
    }
    if input.resolution_receipt_ref.is_none() && input.name.is_some() {
        push_bounded(
            &mut diagnostics,
            "name use must bind an admitted resolution receipt".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name use diagnostics",
        )?;
    }
    if input.policy_refs.is_empty() || input.provenance_refs.is_empty() || input.capability_refs.is_empty() {
        push_bounded(
            &mut diagnostics,
            "name views do not grant policy, provenance, or capability authority".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact name use diagnostics",
        )?;
    }
    Ok(diagnostics)
}

fn name_use_refs(input: &ArtifactNameUseInput) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    if let Some(exact_artifact_ref) = input.exact_artifact_ref.as_ref() {
        push_bounded(&mut refs, exact_artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    }
    if let Some(resolution_receipt_ref) = input.resolution_receipt_ref.as_ref() {
        push_bounded(
            &mut refs,
            resolution_receipt_ref.clone(),
            MAX_ARTIFACT_REF_LIST,
            "artifact name use refs",
        )?;
    }
    extend_cloned_bounded(&mut refs, &input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    extend_cloned_bounded(&mut refs, &input.provenance_refs, MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    extend_cloned_bounded(&mut refs, &input.capability_refs, MAX_ARTIFACT_REF_LIST, "artifact name use refs")?;
    if refs.is_empty() {
        push_bounded(
            &mut refs,
            canonical_hash(&record("artifact-name-use-denial", vec![string(&input.operation)]))?,
            MAX_ARTIFACT_REF_LIST,
            "artifact name use refs",
        )?;
    }
    Ok(sorted_unique(&refs))
}

fn validate_release_snapshot_draft(input: &ReleaseSnapshotDraftInput) -> Result<()> {
    validate_non_empty(&input.namespace_scope, "release snapshot namespace")?;
    validate_non_empty(&input.snapshot_id, "release snapshot id")?;
    validate_release_snapshot_refs(
        &ReleaseSnapshotValueInput {
            namespace_scope: input.namespace_scope.clone(),
            snapshot_id: input.snapshot_id.clone(),
            artifact_refs: input.artifact_refs.clone(),
            artifact_set_ref: input.artifact_set_ref.clone(),
            dependency_closure_digest: testable_placeholder_ref("release-snapshot-closure")?,
            dependency_index_ref: testable_placeholder_ref("release-snapshot-index")?,
            doc_refs: input.doc_refs.clone(),
            transcript_refs: input.transcript_refs.clone(),
            expected_receipt_refs: input.expected_receipt_refs.clone(),
            policy_refs: input.policy_refs.clone(),
            provenance_refs: input.provenance_refs.clone(),
            source_gate_refs: input.source_gate_refs.clone(),
            resource_refs: input.resource_refs.clone(),
            compatibility_refs: input.compatibility_refs.clone(),
            migration_refs: input.migration_refs.clone(),
            upgrade_session_refs: input.upgrade_session_refs.clone(),
            rollback_refs: input.rollback_refs.clone(),
            cutover_refs: input.cutover_refs.clone(),
            caveats: input.caveats.clone(),
            non_claims: input.non_claims.clone(),
            redaction_profile_ref: input.redaction_profile_ref.clone(),
            signature_subject_ref: testable_placeholder_ref("release-snapshot-subject")?,
            signature_refs: input.signature_refs.clone(),
            stale_evidence_refs: input.stale_evidence_refs.clone(),
        },
        "release snapshot draft",
    )
}

fn validate_release_snapshot_value_input(input: &ReleaseSnapshotValueInput) -> Result<()> {
    validate_non_empty(&input.namespace_scope, "release snapshot namespace")?;
    validate_non_empty(&input.snapshot_id, "release snapshot id")?;
    validate_release_snapshot_refs(input, "release snapshot")
}

fn validate_release_snapshot_subject_input(input: &ReleaseSnapshotSubjectInput<'_>) -> Result<()> {
    validate_non_empty(input.namespace_scope, "release snapshot subject namespace")?;
    validate_non_empty(input.snapshot_id, "release snapshot subject id")?;
    validate_refs(input.artifact_refs, "release snapshot subject artifact ref")?;
    ensure_non_empty(input.artifact_refs.len(), "release snapshot subject artifacts")?;
    if let Some(artifact_set_ref) = input.artifact_set_ref {
        validate_ref(artifact_set_ref, "release snapshot subject artifact set ref")?;
    }
    validate_ref(input.dependency_closure_digest, "release snapshot subject closure digest")?;
    validate_ref(input.dependency_index_ref, "release snapshot subject dependency index ref")?;
    validate_refs(input.doc_refs, "release snapshot subject doc ref")?;
    validate_refs(input.transcript_refs, "release snapshot subject transcript ref")?;
    validate_refs(input.expected_receipt_refs, "release snapshot subject expected receipt ref")?;
    validate_refs(input.policy_refs, "release snapshot subject policy ref")?;
    validate_refs(input.provenance_refs, "release snapshot subject provenance ref")?;
    validate_refs(input.source_gate_refs, "release snapshot subject source gate ref")?;
    validate_refs(input.resource_refs, "release snapshot subject resource ref")?;
    validate_refs(input.compatibility_refs, "release snapshot subject compatibility ref")?;
    validate_refs(input.migration_refs, "release snapshot subject migration ref")?;
    validate_refs(input.upgrade_session_refs, "release snapshot subject upgrade ref")?;
    validate_refs(input.rollback_refs, "release snapshot subject rollback ref")?;
    validate_refs(input.cutover_refs, "release snapshot subject cutover ref")?;
    validate_strings(input.caveats, "release snapshot subject caveat")?;
    validate_strings(input.non_claims, "release snapshot subject non-claim")?;
    if let Some(redaction_profile_ref) = input.redaction_profile_ref {
        validate_ref(redaction_profile_ref, "release snapshot subject redaction profile ref")?;
    }
    validate_refs(input.stale_evidence_refs, "release snapshot subject stale evidence ref")
}

fn validate_release_snapshot_refs(input: &ReleaseSnapshotValueInput, label: &str) -> Result<()> {
    validate_refs(&input.artifact_refs, "release snapshot artifact ref")?;
    ensure_non_empty(input.artifact_refs.len(), "release snapshot artifacts")?;
    if let Some(artifact_set_ref) = input.artifact_set_ref.as_ref() {
        validate_ref(artifact_set_ref, "release snapshot artifact set ref")?;
    }
    validate_ref(&input.dependency_closure_digest, "release snapshot closure digest")?;
    validate_ref(&input.dependency_index_ref, "release snapshot dependency index ref")?;
    validate_refs(&input.doc_refs, "release snapshot doc ref")?;
    validate_refs(&input.transcript_refs, "release snapshot transcript ref")?;
    validate_refs(&input.expected_receipt_refs, "release snapshot expected receipt ref")?;
    validate_refs(&input.policy_refs, "release snapshot policy ref")?;
    validate_refs(&input.provenance_refs, "release snapshot provenance ref")?;
    validate_refs(&input.source_gate_refs, "release snapshot source gate ref")?;
    validate_refs(&input.resource_refs, "release snapshot resource ref")?;
    validate_refs(&input.compatibility_refs, "release snapshot compatibility ref")?;
    validate_refs(&input.migration_refs, "release snapshot migration ref")?;
    validate_refs(&input.upgrade_session_refs, "release snapshot upgrade session ref")?;
    validate_refs(&input.rollback_refs, "release snapshot rollback ref")?;
    validate_refs(&input.cutover_refs, "release snapshot cutover ref")?;
    validate_strings(&input.caveats, "release snapshot caveat")?;
    validate_strings(&input.non_claims, "release snapshot non-claim")?;
    if let Some(redaction_profile_ref) = input.redaction_profile_ref.as_ref() {
        validate_ref(redaction_profile_ref, "release snapshot redaction profile ref")?;
    }
    validate_ref(&input.signature_subject_ref, "release snapshot signature subject ref")?;
    validate_refs(&input.signature_refs, "release snapshot signature ref")?;
    ensure_non_empty(input.signature_refs.len(), "release snapshot signatures")?;
    validate_refs(&input.stale_evidence_refs, "release snapshot stale evidence ref")?;
    ensure_count_at_most(input.caveats.len(), MAX_ARTIFACT_DIAGNOSTICS, label)?;
    ensure_count_at_most(input.non_claims.len(), MAX_ARTIFACT_DIAGNOSTICS, label)
}

fn validate_release_channel_update_input(input: &ReleaseChannelUpdateInput) -> Result<()> {
    validate_non_empty(&input.channel, "release channel name")?;
    validate_ref(&input.snapshot_ref, "release channel snapshot ref")?;
    validate_refs(&input.policy_refs, "release channel policy ref")?;
    validate_refs(&input.capability_refs, "release channel capability ref")?;
    validate_refs(&input.evidence_refs, "release channel evidence ref")?;
    ensure_non_empty(input.policy_refs.len(), "release channel policy refs")?;
    ensure_non_empty(input.capability_refs.len(), "release channel capability refs")
}

fn validate_release_channel_admission_input(input: &ReleaseChannelAdmissionInput) -> Result<()> {
    validate_ref(&input.channel_pointer_ref, "release channel pointer ref")?;
    validate_refs(&input.release_evidence_refs, "release channel release evidence ref")?;
    validate_refs(&input.policy_refs, "release channel policy ref")?;
    validate_refs(&input.provenance_refs, "release channel provenance ref")?;
    validate_refs(&input.source_gate_refs, "release channel source gate ref")?;
    validate_refs(&input.authority_refs, "release channel authority ref")?;
    validate_refs(&input.resource_refs, "release channel resource ref")
}

fn validate_strings(values: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_ARTIFACT_REF_LIST, field)?;
    for value in values {
        validate_non_empty(value, field)?;
    }
    Ok(())
}

fn ensure_non_empty(count: usize, label: &str) -> Result<()> {
    if count == 0 {
        Err(MoltenError::invalid_harness(format!("{label} cannot be empty")))
    } else {
        Ok(())
    }
}

fn testable_placeholder_ref(label: &'static str) -> Result<String> {
    canonical_hash(&record("artifact-placeholder-ref", vec![string(label)]))
}

fn validate_install_input(input: &ArtifactInstallInput) -> Result<()> {
    validate_kind(&input.kind)?;
    validate_refs(&input.schema_refs, "artifact schema ref")?;
    validate_refs(&input.dependency_refs, "artifact dependency ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref.as_ref() {
        validate_ref(effect_manifest_ref, "artifact effect manifest ref")?;
    }
    validate_refs(&input.policy_refs, "artifact policy ref")?;
    validate_refs(&input.evidence_refs, "artifact evidence ref")?;
    validate_ref(&input.installer_ref, "artifact installer ref")?;
    if input.capability_refs.is_empty() {
        return Err(MoltenError::invalid_harness("artifact install requires at least one capability ref"));
    }
    validate_refs(&input.capability_refs, "artifact capability ref")
}
