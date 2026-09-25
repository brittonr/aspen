
pub fn parse_name_view_value(value: &IoValue) -> Result<ArtifactNameView> {
    let fields = value
        .collect_simple_record("artifact-name-view-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-name-view-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_NAME_VIEW_SCHEMA, "artifact name view")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "name-view-is-not-authority", "artifact name view")?;
    let target_value = value_to_iovalue(&fields[4]);
    let target = simple_record(&target_value, "target", 2)?;
    Ok(ArtifactNameView {
        view_ref: canonical_hash(value)?,
        view_kind: record_string(&fields[1], "kind")?,
        name: record_string(&fields[2], "name")?,
        scope: record_string(&fields[3], "scope")?,
        target_kind: required_string(&target[0], "target kind")?,
        target_ref: required_ref(&target[1], "target ref")?,
        issuer_ref: record_ref(&fields[5], "issuer")?,
        previous_view_ref: record_optional_ref(&fields[6], "previous")?,
        tombstone_ref: record_optional_ref(&fields[7], "tombstone")?,
        policy_refs: sorted_unique(&record_ref_sequence(&fields[8], "policy")?),
        evidence_refs: sorted_unique(&record_ref_sequence(&fields[9], "evidence")?),
        value: value.clone(),
    })
}

pub fn set_name_view(root: &Path, input: &ArtifactNameViewInput) -> Result<ArtifactNameViewUpdate> {
    let root = open_capability_artifact_root(root)?;
    set_name_view_with_root(&root, input)
}

pub fn set_name_view_with_root(
    root: &CapabilityArtifactRoot,
    input: &ArtifactNameViewInput,
) -> Result<ArtifactNameViewUpdate> {
    validate_name_view_update_authority(input)?;
    if input.target_kind == "artifact-ref" {
        read_artifact_with_root(root, &input.target_ref)?;
    }
    let scoped_name = scoped_name_view_key(&input.scope, &input.name)?;
    let previous_pointer = read_name_pointer_with_root(root, &input.view_kind, &scoped_name)?;
    let previous_view_ref = previous_pointer.as_ref().map(|pointer| pointer.pointer_ref.as_str());
    let value = name_view_value(input, previous_view_ref)?;
    let view = parse_name_view_value(&value)?;
    let pointer = set_name_pointer_with_root(root, &SetNamePointerInput {
        pointer_kind: &input.view_kind,
        name: &scoped_name,
        artifact_ref: &input.target_ref,
        policy_refs: &input.policy_refs,
        evidence_refs: &input.evidence_refs,
    })?;
    let refs = name_view_update_refs(input, &view, &pointer)?;
    let diagnostics = vec!["artifact name views are discovery metadata, not authority or identity".to_string()];
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "name-view-set",
        decision: "pass",
        subject_ref: &view.view_ref,
        name: Some(&scoped_name),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("name-view-authorized", "pass"),
            ("names-are-metadata", "pass"),
            ("target-is-exact-ref", "pass"),
            ("name-view-is-not-authority", "pass"),
        ],
    })?;
    store_receipt(root, &receipt_value)?;
    Ok(ArtifactNameViewUpdate {
        view,
        pointer,
        receipt_ref: canonical_hash(&receipt_value)?,
        receipt_value,
    })
}

// r[impl molten.artifacts.name_ambiguity_denial]
// r[impl molten.artifacts.exact_ref_pinning]
pub fn resolve_name_view(input: &ArtifactNameResolutionInput) -> Result<ArtifactNameResolution> {
    validate_name_resolution_input(input)?;
    let candidates = active_resolution_candidates(input)?;
    let diagnostics = name_resolution_diagnostics(input, &candidates)?;
    let decision = if diagnostics.iter().any(|diagnostic| diagnostic.contains("deny")) { "deny" } else { "pass" };
    let resolved_ref = if decision == "pass" { candidates.first().map(|view| view.target_ref.clone()) } else { None };
    let candidate_refs = candidates.iter().map(|view| view.target_ref.clone()).collect::<Vec<_>>();
    let resolution_value = record("artifact-name-resolution-v1", vec![
        record("kind", vec![string(&input.view_kind)]),
        record("name", vec![string(&input.name)]),
        record("scope", vec![optional_string_value(input.scope.as_deref())]),
        record("decision", vec![string(decision)]),
        record("resolved", vec![optional_ref_value(resolved_ref.as_deref())]),
        record("candidates", vec![refs_sequence(&candidate_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&["exact-ref-pinning-required", "name-view-is-not-authority"]),
    ]);
    let resolution_ref = canonical_hash(&resolution_value)?;
    let mut refs = vec![resolution_ref.clone()];
    extend_cloned_bounded(&mut refs, &candidate_refs, MAX_ARTIFACT_REF_LIST, "name resolution refs")?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "name-view-resolve",
        decision,
        subject_ref: &resolution_ref,
        name: Some(&input.name),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("ambiguity-denies", pass_fail(decision == "pass")),
            ("exact-ref-pinning-required", "pass"),
            ("name-view-is-not-authority", "pass"),
        ],
    })?;
    Ok(ArtifactNameResolution {
        resolution_ref,
        decision: decision.to_string(),
        resolved_ref,
        candidate_refs,
        diagnostics,
        receipt_value,
    })
}

// r[impl molten.artifacts.name_views_non_authority]
pub fn name_view_use_receipt(input: &ArtifactNameUseInput) -> Result<ArtifactNameUseReceipt> {
    validate_name_use_input(input)?;
    let diagnostics = name_use_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let refs = name_use_refs(input)?;
    let subject_ref = input
        .exact_artifact_ref
        .as_deref()
        .or(input.resolution_receipt_ref.as_deref())
        .unwrap_or_else(|| refs.first().map(String::as_str).unwrap_or("blake3:0000000000000000000000000000000000000000000000000000000000000000"));
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: &input.operation,
        decision,
        subject_ref,
        name: input.name.as_deref(),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("exact-artifact-ref-pinned", pass_fail(input.exact_artifact_ref.is_some())),
            ("resolution-receipt-bound", pass_fail(input.resolution_receipt_ref.is_some())),
            ("name-view-is-not-authority", "pass"),
            ("policy-provenance-capability-bound", pass_fail(diagnostics.is_empty())),
        ],
    })?;
    Ok(ArtifactNameUseReceipt {
        receipt_ref: canonical_hash(&receipt_value)?,
        decision: decision.to_string(),
        diagnostics,
        value: receipt_value,
    })
}

pub fn read_name_pointer(root: &Path, pointer_kind: &str, name: &str) -> Result<Option<ArtifactNamePointer>> {
    let root = open_capability_artifact_root(root)?;
    read_name_pointer_with_root(&root, pointer_kind, name)
}

pub fn read_name_pointer_with_root(
    root: &CapabilityArtifactRoot,
    pointer_kind: &str,
    name: &str,
) -> Result<Option<ArtifactNamePointer>> {
    validate_pointer_kind(pointer_kind)?;
    validate_non_empty(name, "artifact pointer name")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_NAMES).map_err(index_error)?;
    let Some(bytes) = table.get(name_key(pointer_kind, name)?.as_str()).map_err(index_error)? else {
        return Ok(None);
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_name_pointer_value(&value).map(Some)
}

pub fn direct_dependencies(root: &Path, artifact_ref: &str) -> Result<Vec<String>> {
    let root = open_capability_artifact_root(root)?;
    direct_dependencies_with_root(&root, artifact_ref)
}

pub fn direct_dependencies_with_root(root: &CapabilityArtifactRoot, artifact_ref: &str) -> Result<Vec<String>> {
    Ok(read_artifact_with_root(root, artifact_ref)?.dependency_refs)
}

pub fn dependency_closure(root: &Path, roots: &[String]) -> Result<ArtifactClosure> {
    let root = open_capability_artifact_root(root)?;
    dependency_closure_with_root(&root, roots)
}

pub fn dependency_closure_with_root(root: &CapabilityArtifactRoot, roots: &[String]) -> Result<ArtifactClosure> {
    let (closure_refs, missing_refs) = compute_closure_refs(root, roots)?;
    let closure_value = closure_value(roots, &closure_refs, &missing_refs)?;
    let closure_hash = canonical_hash(&closure_value)?;
    let decision = if missing_refs.is_empty() { "pass" } else { "deny" };
    let mut diagnostics = Vec::new();
    for missing in &missing_refs {
        push_bounded(
            &mut diagnostics,
            format!("missing dependency {missing}"),
            MAX_ARTIFACT_DIAGNOSTICS,
            "artifact dependency closure diagnostics",
        )?;
    }
    let mut refs = Vec::new();
    extend_cloned_bounded(&mut refs, roots, MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    extend_cloned_bounded(&mut refs, &closure_refs, MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    extend_cloned_bounded(&mut refs, &missing_refs, MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    push_bounded(&mut refs, closure_hash.clone(), MAX_ARTIFACT_REF_LIST, "artifact dependency closure refs")?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "dependency-closure",
        decision,
        subject_ref: &closure_hash,
        name: None,
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("dependency-closure", if missing_refs.is_empty() { "pass" } else { "fail" }),
            ("closure-hash", "pass"),
            ("ordered-refs", "pass"),
        ],
    })?;
    store_receipt(root, &receipt_value)?;
    Ok(ArtifactClosure {
        roots: sorted_unique(roots),
        closure_refs,
        missing_refs,
        closure_hash,
        receipt_value,
    })
}

pub fn impact(root: &Path, seeds: &[String]) -> Result<ArtifactImpact> {
    let root = open_capability_artifact_root(root)?;
    impact_with_root(&root, seeds)
}

pub fn impact_with_root(root: &CapabilityArtifactRoot, seeds: &[String]) -> Result<ArtifactImpact> {
    let impacted_refs = impact_refs_with_root(root, seeds)?;
    let impact_value = record("artifact-impact-v1", vec![
        refs_record("seeds", &sorted_unique(seeds)),
        refs_record("impacted", &impacted_refs),
    ]);
    let impact_hash = canonical_hash(&impact_value)?;
    let mut refs = sorted_unique(seeds);
    extend_cloned_bounded(&mut refs, &impacted_refs, MAX_ARTIFACT_REF_LIST, "artifact impact refs")?;
    push_bounded(&mut refs, impact_hash.clone(), MAX_ARTIFACT_REF_LIST, "artifact impact refs")?;
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "impact",
        decision: "pass",
        subject_ref: &impact_hash,
        name: None,
        refs: &refs,
        diagnostics: &[],
        checks: &[("reverse-dependency-impact", "pass"), ("impact-hash", "pass")],
    })?;
    store_receipt(root, &receipt_value)?;
    Ok(ArtifactImpact {
        seeds: sorted_unique(seeds),
        impacted_refs,
        impact_hash,
        receipt_value,
    })
}

pub fn impact_refs(root: &Path, seeds: &[String]) -> Result<Vec<String>> {
    let root = open_capability_artifact_root(root)?;
    impact_refs_with_root(&root, seeds)
}
