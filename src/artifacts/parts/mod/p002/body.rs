
pub fn reference_diagnostics(root: &Path, target_ref: &str) -> Result<Vec<String>> {
    let root = open_capability_artifact_root(root)?;
    reference_diagnostics_with_root(&root, target_ref)
}

pub fn reference_diagnostics_with_root(
    root: &CapabilityArtifactRoot,
    target_ref: &str,
) -> Result<Vec<String>> {
    validate_ref(target_ref, "artifact reference diagnostic ref")?;
    let mut diagnostics = Vec::new();
    if let Ok(impact) = impact_refs_with_root(root, &[target_ref.to_string()])
        && impact.iter().any(|reference| reference != target_ref)
    {
        push_bounded(
            &mut diagnostics,
            format!("registry reverse dependencies retain {target_ref}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact reference diagnostics",
        )?;
    }
    for pointer in all_name_pointers(root)? {
        if pointer.artifact_ref == target_ref || pointer.previous_ref.as_deref() == Some(target_ref) {
            push_bounded(
                &mut diagnostics,
                format!("registry pointer {}:{} retains {target_ref}", pointer.pointer_kind, pointer.name),
                MAX_ARTIFACT_DIAGNOSTICS,
                "artifact reference diagnostics",
            )?;
        }
    }
    if registry_contains_structural_ref(root, target_ref)? {
        push_bounded(
            &mut diagnostics,
            format!("registry receipts or metadata retain {target_ref}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact reference diagnostics",
        )?;
    }
    Ok(diagnostics)
}

pub fn rebuild_index(root: &Path) -> Result<ArtifactIndexRebuild> {
    let root = open_capability_artifact_root(root)?;
    rebuild_index_with_root(&root)
}

pub fn rebuild_index_with_root(root: &CapabilityArtifactRoot) -> Result<ArtifactIndexRebuild> {
    ensure_dirs(root)?;
    let artifacts = list_artifacts_with_root(root, None)?;
    let names = all_name_pointers(root)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    clear_derived_index_tables_in_tx(&write_txn)?;
    for artifact in &artifacts {
        store_derived_indexes_in_tx(&write_txn, artifact)?;
    }
    for pointer in &names {
        let mut table = write_txn.open_table(INDEX_NAMES).map_err(index_error)?;
        table
            .insert(
                name_key(&pointer.pointer_kind, &pointer.name)?.as_str(),
                canonical_bytes(&pointer.value)?.as_slice(),
            )
            .map_err(index_error)?;
    }
    let mut refs = Vec::new();
    for artifact in &artifacts {
        push_bounded(&mut refs, artifact.artifact_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact index rebuild refs")?;
    }
    let rebuild_ref = local_ref("artifact-index-rebuild", &refs)?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "index-rebuild",
        decision: "pass",
        subject_ref: &rebuild_ref,
        name: None,
        refs: &refs,
        diagnostics: &[],
        checks: &[
            ("redb-index-artifacts", "pass"),
            ("redb-index-dependencies", "pass"),
            ("redb-index-reverse-dependencies", "pass"),
            ("redb-index-semantic", "pass"),
        ],
    })?;
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    Ok(ArtifactIndexRebuild {
        artifacts: artifacts.len(),
        names: names.len(),
        receipt_value,
    })
}

// r[impl molten.artifacts.dependency_edge_records]
pub fn dependency_edges_for_artifact(artifact: &ArtifactRecord) -> Result<Vec<ArtifactDependencyEdge>> {
    let mut edges = Vec::new();
    for dependency_ref in &artifact.dependency_refs {
        push_dependency_edge(&mut edges, artifact, DependencyTargetInput { target_ref: dependency_ref, target_kind: "artifact", relation: "imports", required: true, evidence_refs: artifact.evidence_refs.as_slice() })?;
    }
    for schema_ref in &artifact.schema_refs {
        push_dependency_edge(&mut edges, artifact, DependencyTargetInput { target_ref: schema_ref, target_kind: "schema", relation: "validates-with", required: true, evidence_refs: artifact.evidence_refs.as_slice() })?;
    }
    if let Some(effect_manifest_ref) = artifact.effect_manifest_ref.as_ref() {
        push_dependency_edge(&mut edges, artifact, DependencyTargetInput { target_ref: effect_manifest_ref, target_kind: "effect", relation: "invokes", required: true, evidence_refs: artifact.evidence_refs.as_slice() })?;
    }
    for policy_ref in &artifact.policy_refs {
        push_dependency_edge(&mut edges, artifact, DependencyTargetInput { target_ref: policy_ref, target_kind: "policy", relation: "validates-with", required: true, evidence_refs: artifact.evidence_refs.as_slice() })?;
    }
    for evidence_ref in &artifact.evidence_refs {
        push_dependency_edge(&mut edges, artifact, DependencyTargetInput { target_ref: evidence_ref, target_kind: "evidence", relation: "documents", required: false, evidence_refs: &[] })?;
    }
    Ok(edges)
}

pub fn list_dependency_edges(root: &Path) -> Result<Vec<ArtifactDependencyEdge>> {
    let root = open_capability_artifact_root(root)?;
    list_dependency_edges_with_root(&root)
}

pub fn list_dependency_edges_with_root(root: &CapabilityArtifactRoot) -> Result<Vec<ArtifactDependencyEdge>> {
    let mut edges = Vec::new();
    for artifact in list_artifacts_with_root(root, None)? {
        extend_bounded(
            &mut edges,
            dependency_edges_for_artifact(&artifact)?,
            MAX_ARTIFACT_RECORDS,
            "artifact dependency edges",
        )?;
    }
    let normalized = normalize_dependency_edges(&edges)?.edges;
    Ok(normalized)
}

// r[impl molten.artifacts.reverse_dependency_index]
// r[impl molten.artifacts.index_rebuild_determinism]
pub fn dependency_index_digest(edges: &[ArtifactDependencyEdge]) -> Result<String> {
    let normalized = normalize_dependency_edges(edges)?;
    ensure_count_at_most(
        normalized.duplicate_refs.len(),
        MAX_ARTIFACT_RECORDS,
        "artifact dependency duplicate refs",
    )?;
    let edge_refs = normalized.edges.iter().map(|edge| edge.edge_ref.clone()).collect::<Vec<_>>();
    local_ref("artifact-dependency-index", &edge_refs)
}

// r[impl molten.artifacts.impact_query_receipts]
pub fn impact_query(root: &Path, input: &ArtifactImpactQueryInput) -> Result<ArtifactImpactQueryReceipt> {
    let root = open_capability_artifact_root(root)?;
    impact_query_with_root(&root, input)
}

pub fn impact_query_with_root(
    root: &CapabilityArtifactRoot,
    input: &ArtifactImpactQueryInput,
) -> Result<ArtifactImpactQueryReceipt> {
    validate_ref(&input.subject_ref, "artifact impact query subject ref")?;
    validate_relation_filters(&input.relation_filters)?;
    validate_refs(&input.hidden_refs, "artifact impact query hidden ref")?;
    let edges = list_dependency_edges_with_root(root)?;
    let index_ref = dependency_index_digest(&edges)?;
    let hidden = input.hidden_refs.iter().cloned().collect::<std::collections::BTreeSet<_>>();
    let direct_all = dependents_from_edges(&edges, std::slice::from_ref(&input.subject_ref), &input.relation_filters)?;
    let direct = redact_refs(&direct_all, &hidden)?;
    let redacted_direct = redacted_refs(&direct_all, &hidden)?;
    let transitive_all = if input.include_transitive {
        transitive_dependents_from_edges(&edges, &input.subject_ref, &input.relation_filters)?
    } else {
        Vec::new()
    };
    let transitive = redact_refs(&transitive_all, &hidden)?;
    let redacted_transitive = redacted_refs(&transitive_all, &hidden)?;
    let redacted = sorted_unique(&[redacted_direct, redacted_transitive].concat());
    let mut diagnostics = Vec::new();
    if !redacted.is_empty() {
        push_bounded(
            &mut diagnostics,
            "impact query redacted hidden dependency refs".to_string(),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact impact query diagnostics",
        )?;
    }
    let query_ref = impact_query_ref(input, &index_ref)?;
    let mut refs = vec![input.subject_ref.clone(), index_ref.clone(), query_ref.clone()];
    extend_cloned_bounded(&mut refs, &direct, MAX_ARTIFACT_REF_LIST, "artifact impact query refs")?;
    extend_cloned_bounded(&mut refs, &transitive, MAX_ARTIFACT_REF_LIST, "artifact impact query refs")?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "impact-query",
        decision: "pass",
        subject_ref: &query_ref,
        name: None,
        refs: &sorted_unique(&refs),
        diagnostics: &diagnostics,
        checks: &[
            ("canonical-dependency-edges", "pass"),
            ("reverse-index-digest", "pass"),
            ("redaction-bound", "pass"),
            ("planning-evidence-only", "pass"),
        ],
    })?;
    Ok(ArtifactImpactQueryReceipt {
        query_ref,
        decision: "pass".to_string(),
        direct_dependents: direct,
        transitive_dependents: transitive,
        redacted_refs: redacted,
        diagnostics,
        receipt_value,
    })
}

// r[impl molten.release_snapshots.namespace_snapshot_artifacts]
pub fn release_snapshot_value_input(root: &Path, draft: &ReleaseSnapshotDraftInput) -> Result<ReleaseSnapshotValueInput> {
    let root = open_capability_artifact_root(root)?;
    release_snapshot_value_input_with_root(&root, draft)
}

pub fn release_snapshot_value_input_with_root(
    root: &CapabilityArtifactRoot,
    draft: &ReleaseSnapshotDraftInput,
) -> Result<ReleaseSnapshotValueInput> {
    validate_release_snapshot_draft(draft)?;
    let artifact_refs = sorted_unique(&draft.artifact_refs);
    let closure_refs = require_exact_closure_refs(root, &artifact_refs)?;
    let no_missing_refs = Vec::new();
    let dependency_closure_digest = canonical_hash(&closure_value(&artifact_refs, &closure_refs, &no_missing_refs)?)?;
    let dependency_index_ref = release_snapshot_dependency_index_digest(root, &artifact_refs)?;
    let subject = ReleaseSnapshotSubjectInput {
        namespace_scope: &draft.namespace_scope,
        snapshot_id: &draft.snapshot_id,
        artifact_refs: &artifact_refs,
        artifact_set_ref: draft.artifact_set_ref.as_deref(),
        dependency_closure_digest: &dependency_closure_digest,
        dependency_index_ref: &dependency_index_ref,
        doc_refs: &draft.doc_refs,
        transcript_refs: &draft.transcript_refs,
        expected_receipt_refs: &draft.expected_receipt_refs,
        policy_refs: &draft.policy_refs,
        provenance_refs: &draft.provenance_refs,
        source_gate_refs: &draft.source_gate_refs,
        resource_refs: &draft.resource_refs,
        compatibility_refs: &draft.compatibility_refs,
        migration_refs: &draft.migration_refs,
        upgrade_session_refs: &draft.upgrade_session_refs,
        rollback_refs: &draft.rollback_refs,
        cutover_refs: &draft.cutover_refs,
        caveats: &draft.caveats,
        non_claims: &draft.non_claims,
        redaction_profile_ref: draft.redaction_profile_ref.as_deref(),
        stale_evidence_refs: &draft.stale_evidence_refs,
    };
    let signature_subject_ref = release_snapshot_subject_ref(&subject)?;
    Ok(ReleaseSnapshotValueInput {
        namespace_scope: draft.namespace_scope.clone(),
        snapshot_id: draft.snapshot_id.clone(),
        artifact_refs,
        artifact_set_ref: draft.artifact_set_ref.clone(),
        dependency_closure_digest,
        dependency_index_ref,
        doc_refs: sorted_unique(&draft.doc_refs),
        transcript_refs: sorted_unique(&draft.transcript_refs),
        expected_receipt_refs: sorted_unique(&draft.expected_receipt_refs),
        policy_refs: sorted_unique(&draft.policy_refs),
        provenance_refs: sorted_unique(&draft.provenance_refs),
        source_gate_refs: sorted_unique(&draft.source_gate_refs),
        resource_refs: sorted_unique(&draft.resource_refs),
        compatibility_refs: sorted_unique(&draft.compatibility_refs),
        migration_refs: sorted_unique(&draft.migration_refs),
        upgrade_session_refs: sorted_unique(&draft.upgrade_session_refs),
        rollback_refs: sorted_unique(&draft.rollback_refs),
        cutover_refs: sorted_unique(&draft.cutover_refs),
        caveats: sorted_unique(&draft.caveats),
        non_claims: sorted_unique(&draft.non_claims),
        redaction_profile_ref: draft.redaction_profile_ref.clone(),
        signature_subject_ref,
        signature_refs: sorted_unique(&draft.signature_refs),
        stale_evidence_refs: sorted_unique(&draft.stale_evidence_refs),
    })
}
