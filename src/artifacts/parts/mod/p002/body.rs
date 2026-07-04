
pub fn reference_diagnostics(root: &Path, target_ref: &str) -> Result<Vec<String>> {
    validate_ref(target_ref, "artifact reference diagnostic ref")?;
    let mut diagnostics = Vec::new();
    if let Ok(impact) = impact_refs(root, &[target_ref.to_string()])
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
    ensure_dirs(root)?;
    let artifacts = list_artifacts(root, None)?;
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

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<ArtifactReceipt> {
    validate_ref(receipt_ref, "artifact receipt ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = receipts.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("artifact receipt {receipt_ref} not found")));
    };
    let value = parse_canonical_bytes(bytes.value())?;
    parse_artifact_receipt(&value)
}

pub fn list_receipts(root: &Path) -> Result<Vec<ArtifactReceipt>> {
    let mut receipts = Vec::new();
    for value in receipt_values(root)? {
        push_bounded(
            &mut receipts,
            parse_artifact_receipt(&value)?,
            MAX_ARTIFACT_RECEIPTS,
            "artifact registry receipts",
        )?;
    }
    receipts.sort_by(|left, right| left.receipt_ref.cmp(&right.receipt_ref));
    Ok(receipts)
}

pub fn parse_artifact_receipt(value: &IoValue) -> Result<ArtifactReceipt> {
    let fields = value
        .collect_simple_record("artifact-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <artifact-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::ARTIFACT_RECEIPT_SCHEMA, "artifact receipt")?;
    let checks = parse_checks(&fields[7])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("artifact receipt missing checks"));
    }
    Ok(ArtifactReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        subject_ref: record_ref(&fields[3], "subject")?,
        name: record_optional_string(&fields[4], "name")?,
        value: value.clone(),
    })
}

fn store_artifact_in_tx(
    write_txn: &redb::WriteTransaction,
    artifact: &ArtifactRecord,
    payload_bytes: &[u8],
) -> Result<()> {
    {
        let artifact_bytes = canonical_bytes(&artifact.value)?;
        let mut artifacts = write_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
        artifacts.insert(artifact.artifact_ref.as_str(), artifact_bytes.as_slice()).map_err(index_error)?;
    }
    if matches!(artifact.payload, ArtifactPayloadRef::Inline { .. }) {
        let mut payloads = write_txn.open_table(INDEX_PAYLOADS).map_err(index_error)?;
        payloads.insert(artifact.artifact_ref.as_str(), payload_bytes).map_err(index_error)?;
    }
    store_derived_indexes_in_tx(write_txn, artifact)
}

fn store_derived_indexes_in_tx(write_txn: &redb::WriteTransaction, artifact: &ArtifactRecord) -> Result<()> {
    {
        let deps_value = refs_value(&artifact.dependency_refs);
        let mut deps = write_txn.open_table(INDEX_DEPS).map_err(index_error)?;
        deps.insert(artifact.artifact_ref.as_str(), canonical_bytes(&deps_value)?.as_slice())
            .map_err(index_error)?;
    }
    for dependency_ref in &artifact.dependency_refs {
        let mut existing = {
            let reverse = write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
            if let Some(bytes) = reverse.get(dependency_ref.as_str()).map_err(index_error)? {
                parse_refs_value(&parse_canonical_bytes(bytes.value())?, "reverse")?
            } else {
                Vec::new()
            }
        };
        if !existing.iter().any(|value| value == &artifact.artifact_ref) {
            push_bounded(
                &mut existing,
                artifact.artifact_ref.clone(),
                MAX_ARTIFACT_REF_LIST,
                "artifact reverse dependency refs",
            )?;
            existing.sort();
        }
        let mut reverse = write_txn.open_table(INDEX_REVERSE).map_err(index_error)?;
        reverse
            .insert(dependency_ref.as_str(), canonical_bytes(&refs_value(&existing))?.as_slice())
            .map_err(index_error)?;
    }
    insert_str_index(write_txn, INDEX_KIND, "kind", &artifact.kind, &artifact.artifact_ref)?;
    for schema_ref in &artifact.schema_refs {
        insert_str_index(write_txn, INDEX_SCHEMA, "schema", schema_ref, &artifact.artifact_ref)?;
    }
    if let Some(effect_manifest_ref) = artifact.effect_manifest_ref.as_ref() {
        insert_str_index(write_txn, INDEX_EFFECT, "effect", effect_manifest_ref, &artifact.artifact_ref)?;
    }
    for policy_ref in &artifact.policy_refs {
        insert_str_index(write_txn, INDEX_POLICY, "policy", policy_ref, &artifact.artifact_ref)?;
    }
    for evidence_ref in &artifact.evidence_refs {
        insert_str_index(write_txn, INDEX_EVIDENCE, "evidence", evidence_ref, &artifact.artifact_ref)?;
    }
    Ok(())
}

fn insert_str_index(
    write_txn: &redb::WriteTransaction,
    table_definition: TableDef<&str, &str>,
    index_name: &str,
    indexed_ref: &str,
    artifact_ref: &str,
) -> Result<()> {
    let key = canonical_hash(&record("artifact-semantic-index-key", vec![
        string(index_name),
        string(indexed_ref),
        string(artifact_ref),
    ]))?;
    let mut table = write_txn.open_table(table_definition).map_err(index_error)?;
    table.insert(key.as_str(), artifact_ref).map_err(index_error)?;
    Ok(())
}

fn missing_dependencies(root: &Path, dependency_refs: &[String]) -> Result<Vec<String>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let artifacts = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
    let mut missing = Vec::new();
    for dependency_ref in dependency_refs {
        if artifacts.get(dependency_ref.as_str()).map_err(index_error)?.is_none() {
            push_bounded(&mut missing, dependency_ref.clone(), MAX_ARTIFACT_REF_LIST, "artifact missing dependencies")?;
        }
    }
    Ok(missing)
}

fn compute_closure_refs(root: &Path, roots: &[String]) -> Result<(Vec<String>, Vec<String>)> {
    validate_refs(roots, "artifact closure root ref")?;
    let db = ensure_index_tables(root)?;
    let mut closure = std::collections::BTreeSet::new();
    let mut missing = std::collections::BTreeSet::new();
    ensure_count_at_most(roots.len(), MAX_ARTIFACT_REF_LIST, "artifact closure roots")?;
    let mut stack = roots.to_vec();
    while let Some(current) = stack.pop() {
        if closure.contains(&current) || missing.contains(&current) {
            continue;
        }
        let deps = {
            let read_txn = db.begin_read().map_err(index_error)?;
            let artifacts = read_txn.open_table(INDEX_ARTIFACTS).map_err(index_error)?;
            if artifacts.get(current.as_str()).map_err(index_error)?.is_none() {
                checked_count_sum(missing.len(), 1, MAX_ARTIFACT_REF_LIST, "artifact closure missing refs")?;
                missing.insert(current.clone());
                Vec::new()
            } else {
                let deps = read_txn.open_table(INDEX_DEPS).map_err(index_error)?;
                if let Some(bytes) = deps.get(current.as_str()).map_err(index_error)? {
                    parse_refs_value(&parse_canonical_bytes(bytes.value())?, "dependencies")?
                } else {
                    Vec::new()
                }
            }
        };
        if !missing.contains(&current) {
            checked_count_sum(closure.len(), 1, MAX_ARTIFACT_REF_LIST, "artifact closure refs")?;
            closure.insert(current);
        }
        for dependency in deps {
            push_bounded(&mut stack, dependency, MAX_ARTIFACT_REF_LIST, "artifact closure traversal stack")?;
        }
    }
    Ok((closure.into_iter().collect(), missing.into_iter().collect()))
}

struct ArtifactReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    subject_ref: &'a str,
    name: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

struct NamePointerValueInput<'a> {
    pointer_kind: &'a str,
    name: &'a str,
    artifact_ref: &'a str,
    previous_ref: Option<&'a str>,
    policy_refs: &'a [String],
    receipt_ref: &'a str,
}

fn artifact_receipt_value(input: &ArtifactReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "artifact receipt operation")?;
    if !matches!(input.decision, "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!("unsupported artifact receipt decision {}", input.decision)));
    }
    validate_ref(input.subject_ref, "artifact receipt subject ref")?;
    validate_refs(input.refs, "artifact receipt ref")?;
    Ok(record("artifact-receipt-v1", vec![
        string(crate::preserves_rail::ARTIFACT_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("subject", vec![string(input.subject_ref)]),
        record("name", vec![optional_string_value(input.name)]),
        record("refs", vec![refs_sequence(input.refs)]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}
