
fn commit_install(
    root: &Path,
    artifact: &ArtifactRecord,
    payload_bytes: &[u8],
    receipt_value: &IoValue,
    should_store_artifact: bool,
) -> Result<()> {
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    if should_store_artifact {
        store_artifact_in_tx(&write_txn, artifact, payload_bytes)?;
    }
    store_receipt_in_tx(&write_txn, receipt_value)?;
    write_txn.commit().map_err(index_error)
}

pub fn artifact_value(input: ArtifactValueInput<'_>) -> Result<IoValue> {
    validate_kind(input.kind)?;
    validate_refs(input.schema_refs, "artifact schema ref")?;
    validate_refs(input.dependency_refs, "artifact dependency ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref {
        validate_ref(effect_manifest_ref, "artifact effect manifest ref")?;
    }
    validate_refs(input.policy_refs, "artifact policy ref")?;
    validate_refs(input.evidence_refs, "artifact evidence ref")?;
    Ok(record("artifact-v1", vec![
        string(crate::preserves_rail::ARTIFACT_SCHEMA),
        record("kind", vec![string(input.kind)]),
        record("domain", vec![string(domain_for_kind(input.kind))]),
        payload_value(input.payload)?,
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("dependencies", vec![refs_sequence(input.dependency_refs)]),
        record("effects", vec![optional_ref_value(input.effect_manifest_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        checks_value(&[
            "domain-separated-identity",
            "canonical-payload-ref",
            "explicit-dependency-edges",
            "names-are-metadata",
            "content-addressing-is-not-trust",
        ]),
    ]))
}

// r[impl molten.artifacts.canonical_id_receipts]
pub fn artifact_identity_receipt(input: &ArtifactIdentityInput<'_>) -> Result<ArtifactIdentityReceipt> {
    let diagnostics = artifact_identity_diagnostics(input)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let artifact_ref = if diagnostics.is_empty() {
        Some(input.artifact_ref.map_or_else(|| artifact_identity_ref(input), |artifact_ref| Ok(artifact_ref.to_string()))?)
    } else {
        None
    };
    let value = artifact_identity_receipt_value(input, decision, artifact_ref.as_deref(), &diagnostics)?;
    Ok(ArtifactIdentityReceipt {
        receipt_ref: canonical_hash(&value)?,
        decision: decision.to_string(),
        artifact_ref,
        diagnostics,
        value,
    })
}

pub fn parse_artifact_identity_receipt(value: &IoValue) -> Result<ArtifactIdentityReceipt> {
    let fields = value
        .collect_simple_record("artifact-identity-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-identity-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_IDENTITY_RECEIPT_SCHEMA, "artifact identity receipt")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "identity-is-not-authority", "artifact identity receipt")?;
    let diagnostics = record_strings(&fields[13], "diagnostics")?;
    Ok(ArtifactIdentityReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        artifact_ref: record_optional_ref(&fields[6], "artifact-ref")?,
        diagnostics,
        value: value.clone(),
    })
}

pub fn parse_artifact_value(value: &IoValue) -> Result<ArtifactRecord> {
    let fields = value
        .collect_simple_record("artifact-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_SCHEMA, "artifact")?;
    let kind = record_string(&fields[1], "kind")?;
    let domain = record_string(&fields[2], "domain")?;
    if domain != domain_for_kind(&kind) {
        return Err(MoltenError::invalid_harness(format!("artifact domain {domain} does not match kind {kind}")));
    }
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "domain-separated-identity", "artifact")?;
    require_check(&checks, "names-are-metadata", "artifact")?;
    Ok(ArtifactRecord {
        artifact_ref: canonical_hash(value)?,
        kind,
        domain,
        payload: parse_payload_ref(&fields[3])?,
        schema_refs: record_ref_sequence(&fields[4], "schemas")?,
        dependency_refs: record_ref_sequence(&fields[5], "dependencies")?,
        effect_manifest_ref: record_optional_ref(&fields[6], "effects")?,
        policy_refs: record_ref_sequence(&fields[7], "policy")?,
        evidence_refs: record_ref_sequence(&fields[8], "evidence")?,
        value: value.clone(),
    })
}

pub fn read_artifact(root: &Path, artifact_ref: &str) -> Result<ArtifactRecord> {
    validate_ref(artifact_ref, "artifact ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
    let Some(bytes) = table.get(artifact_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("artifact {artifact_ref} not found")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    let artifact = parse_artifact_value(&value)?;
    if artifact.artifact_ref != artifact_ref {
        return Err(MoltenError::invalid_harness(format!(
            "artifact registry content hash mismatch: got {}, expected {artifact_ref}",
            artifact.artifact_ref
        )));
    }
    Ok(artifact)
}

pub fn read_payload(root: &Path, artifact_ref: &str) -> Result<IoValue> {
    let artifact = read_artifact(root, artifact_ref)?;
    match &artifact.payload {
        ArtifactPayloadRef::Inline { value_ref, .. } => {
            let db = ensure_index_tables(root)?;
            let read_txn = db.begin_read().map_err(index_error)?;
            let table = read_txn.open_table(INDEX_PAYLOADS).map_err(index_error)?;
            let Some(bytes) = table.get(artifact_ref).map_err(index_error)? else {
                return Err(MoltenError::invalid_harness(format!(
                    "inline payload for artifact {artifact_ref} not found"
                )));
            };
            let value = parse_canonical_bytes(bytes.value())?;
            let actual_ref = canonical_hash(&value)?;
            if &actual_ref != value_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "artifact payload hash mismatch: got {actual_ref}, expected {value_ref}"
                )));
            }
            Ok(value)
        }
        ArtifactPayloadRef::ContentRef { manifest_ref, .. } => {
            let read = read_chunk_object(&chunk_root(root), manifest_ref)?;
            let value = parse_canonical_bytes(&read.bytes)?;
            Ok(value)
        }
    }
}

pub fn list_artifacts(root: &Path, kind_filter: Option<&str>) -> Result<Vec<ArtifactRecord>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let table = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
    let mut artifacts = Vec::new();
    for item in table.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        let value = parse_canonical_bytes(bytes.value())?;
        let artifact = parse_artifact_value(&value)?;
        if kind_filter.is_none_or(|kind| kind == artifact.kind) {
            push_bounded(&mut artifacts, artifact, MAX_ARTIFACT_RECORDS, "artifact registry list artifacts")?;
        }
    }
    artifacts.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(artifacts)
}

pub fn set_name_pointer(root: &Path, input: &SetNamePointerInput<'_>) -> Result<ArtifactNamePointer> {
    validate_pointer_kind(input.pointer_kind)?;
    validate_non_empty(input.name, "artifact pointer name")?;
    validate_ref(input.artifact_ref, "artifact pointer ref")?;
    validate_refs(input.policy_refs, "artifact pointer policy ref")?;
    validate_refs(input.evidence_refs, "artifact pointer evidence ref")?;
    read_artifact(root, input.artifact_ref)?;
    let previous = read_name_pointer(root, input.pointer_kind, input.name)?.map(|pointer| pointer.artifact_ref);
    let mut refs = Vec::new();
    push_bounded(&mut refs, input.artifact_ref.to_string(), MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    extend_cloned_bounded(&mut refs, input.policy_refs, MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    extend_cloned_bounded(&mut refs, input.evidence_refs, MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    if let Some(previous) = previous.as_ref() {
        push_bounded(&mut refs, previous.clone(), MAX_ARTIFACT_REF_LIST, "artifact name pointer refs")?;
    }
    let receipt_value = artifact_receipt_value(&ArtifactReceiptValueInput {
        operation: "name-set",
        decision: "pass",
        subject_ref: input.artifact_ref,
        name: Some(input.name),
        refs: &refs,
        diagnostics: &[],
        checks: &[("names-are-metadata", "pass"), ("artifact-content-immutable", "pass")],
    })?;
    let receipt = parse_artifact_receipt(&receipt_value)?;
    let pointer = name_pointer_value(&NamePointerValueInput {
        pointer_kind: input.pointer_kind,
        name: input.name,
        artifact_ref: input.artifact_ref,
        previous_ref: previous.as_deref(),
        policy_refs: input.policy_refs,
        receipt_ref: &receipt.receipt_ref,
    })?;
    let parsed = parse_name_pointer_value(&pointer)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        let mut names = write_txn.open_table(INDEX_NAMES).map_err(index_error)?;
        names
            .insert(name_key(input.pointer_kind, input.name)?.as_str(), canonical_bytes(&pointer)?.as_slice())
            .map_err(index_error)?;
    }
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    Ok(parsed)
}

pub fn read_name_pointer(root: &Path, pointer_kind: &str, name: &str) -> Result<Option<ArtifactNamePointer>> {
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
    Ok(read_artifact(root, artifact_ref)?.dependency_refs)
}

pub fn dependency_closure(root: &Path, roots: &[String]) -> Result<ArtifactClosure> {
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
    let impacted_refs = impact_refs(root, seeds)?;
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
    validate_refs(seeds, "artifact impact seed ref")?;
    let db = ensure_index_tables(root)?;
    let mut impacted: std::collections::BTreeSet<String> = seeds.iter().cloned().collect();
    let mut frontier: Vec<String> = seeds.to_vec();
    while let Some(current) = frontier.pop() {
        let dependents = {
            let read_txn = db.begin_read().map_err(index_error)?;
            let reverse = read_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
            if let Some(bytes) = reverse.get(current.as_str()).map_err(index_error)? {
                parse_refs_value(&parse_canonical_bytes(bytes.value())?, "reverse")?
            } else {
                Vec::new()
            }
        };
        for dependent in dependents {
            if impacted.insert(dependent.clone()) {
                push_bounded(&mut frontier, dependent, MAX_ARTIFACT_RECORDS, "artifact impact frontier")?;
            }
        }
    }
    Ok(impacted.into_iter().collect())
}
