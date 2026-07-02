
fn summary_matches_filters(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    summary: &Summary,
    filters: &[Filter],
    visibility: &VisibilityInput,
) -> Result<bool> {
    if filters.is_empty() {
        return Ok(true);
    }
    let public_text = summary_public_text(registry_root, ledger_root, summary, visibility)?;
    for filter in filters {
        let has_matching_filter = match filter {
            Filter::Ref(value_ref) => &summary.artifact_ref == value_ref || public_text.contains(value_ref),
            Filter::ArtifactKind(kind) => &summary.artifact_kind == kind,
            Filter::LedgerKind(kind) => {
                summary.classifications.iter().any(|item| item == &format!("ledger-kind:{kind}"))
            }
            Filter::SchemaRef(value_ref) => summary.schema_refs.contains(value_ref),
            Filter::StructuralFingerprint(value_ref) => public_text.contains(value_ref),
            Filter::EffectRef(value_ref) => summary.effect_manifest_ref.as_deref() == Some(value_ref.as_str()),
            Filter::PolicyRef(value_ref) => summary.policy_refs.contains(value_ref) || public_text.contains(value_ref),
            Filter::CapabilityRef(value_ref) => public_text.contains(value_ref),
            Filter::EvidenceRef(value_ref) => {
                summary.evidence_refs.contains(value_ref) || public_text.contains(value_ref)
            }
            Filter::DependencyRef(value_ref) => summary.dependency_refs.contains(value_ref),
            Filter::DependentRef(value_ref) => summary.dependent_refs.contains(value_ref),
            Filter::ReceiptOperation(operation) => {
                receipt_field_matches(&public_text, "operation", operation)
                    || public_text.contains(&format!("receipt-operation:{operation}"))
            }
            Filter::ReceiptDecision(decision) => {
                receipt_field_matches(&public_text, "decision", decision)
                    || public_text.contains(&format!("receipt-decision:{decision}"))
            }
            Filter::TranscriptStatus(status) => public_text.contains(&format!("transcript-status:{status}")),
            Filter::UpgradeStatus(status) => public_text.contains(&format!("upgrade-status:{status}")),
            Filter::Text(term) => !term.is_empty() && public_text.contains(term),
        };
        if !has_matching_filter {
            return Ok(false);
        }
    }
    Ok(true)
}

fn summary_public_text(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    summary: &Summary,
    visibility: &VisibilityInput,
) -> Result<String> {
    let mut parts = Vec::new();
    push_bounded(&mut parts, to_text(&summary.value)?, MAX_CATALOG_ITEMS, "catalog public text parts")?;
    if let Ok(artifact) = crate::artifacts::read_artifact(registry_root, &summary.artifact_ref) {
        push_bounded(&mut parts, to_text(&artifact.value)?, MAX_CATALOG_ITEMS, "catalog public text parts")?;
        let payload = crate::artifacts::read_payload(registry_root, &summary.artifact_ref)?;
        push_bounded(
            &mut parts,
            to_text(&maybe_redacted_value(&payload, visibility.redaction_profile_ref.as_deref())?)?,
            MAX_CATALOG_ITEMS,
            "catalog public text parts",
        )?;
    } else if let Some(ledger_root) = ledger_root
        && let Ok(value) = crate::ledger::read_artifact(ledger_root, &summary.artifact_ref)
    {
        push_bounded(
            &mut parts,
            to_text(&maybe_redacted_value(&value, visibility.redaction_profile_ref.as_deref())?)?,
            MAX_CATALOG_ITEMS,
            "catalog public text parts",
        )?;
    }
    Ok(parts.join("\n"))
}

fn direct_dependents(registry_root: &Path, artifact_ref: &str) -> Result<Vec<String>> {
    validate_ref(artifact_ref, "catalog dependent ref")?;
    let mut dependents = Vec::new();
    for artifact in crate::artifacts::list_artifacts(registry_root, None)? {
        if artifact.dependency_refs.iter().any(|dependency| dependency == artifact_ref) {
            push_bounded(&mut dependents, artifact.artifact_ref, MAX_CATALOG_REFS, "catalog dependents")?;
        }
    }
    dependents.sort();
    Ok(dependents)
}

fn scoped_refs(
    registry_root: &Path,
    root_refs: &[String],
    include_dependencies: bool,
    include_dependents: bool,
) -> Result<Set<String>> {
    validate_refs(root_refs, "catalog scope ref")?;
    let mut scoped = Set::new();
    ensure_count_at_most(root_refs.len(), MAX_CATALOG_REFS, "catalog scope roots")?;
    let mut frontier = root_refs.to_vec();
    while let Some(current) = frontier.pop() {
        if scoped.contains(&current) {
            continue;
        }
        insert_bounded(&mut scoped, current.clone(), MAX_CATALOG_REFS, "catalog scoped refs")?;
        if include_dependencies && let Ok(deps) = crate::artifacts::direct_dependencies(registry_root, &current) {
            for dependency in deps {
                push_bounded(&mut frontier, dependency, MAX_CATALOG_REFS, "catalog scope frontier")?;
            }
        }
        if include_dependents && let Ok(dependents) = direct_dependents(registry_root, &current) {
            for dependent in dependents {
                push_bounded(&mut frontier, dependent, MAX_CATALOG_REFS, "catalog scope frontier")?;
            }
        }
    }
    Ok(scoped)
}

fn resolve_reference(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    reference: &str,
    visibility: &VisibilityInput,
) -> Result<String> {
    if is_full_ref(reference) {
        if hidden_set(visibility).contains(reference) {
            return Err(MoltenError::invalid_harness(format!("catalog ref {reference} is hidden")));
        }
        return Ok(reference.to_string());
    }
    if crate::preserves_rail::content_ref_has_prefix(reference) {
        let error = validate_content_ref(reference).expect_err("invalid content ref after failed full-ref check");
        return Err(MoltenError::invalid_harness(format!("malformed full content ref: {error}")));
    }
    let resolution = resolve_short_id(registry_root, ledger_root, &ShortIdInput {
        prefix: reference.to_string(),
        min_length: DEFAULT_SHORT_ID_MIN_LENGTH,
        visibility: visibility.clone(),
    })?;
    resolution
        .full_ref
        .ok_or_else(|| MoltenError::invalid_harness(format!("short id {} did not resolve", reference)))
}

fn visible_candidate_refs(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    visibility: &VisibilityInput,
) -> Result<Vec<String>> {
    let hidden = hidden_set(visibility);
    let mut candidates = Set::new();
    for artifact in crate::artifacts::list_artifacts(registry_root, None)? {
        if !hidden.contains(&artifact.artifact_ref) {
            insert_bounded(&mut candidates, artifact.artifact_ref, MAX_CATALOG_REFS, "catalog visible candidates")?;
        }
    }
    if let Some(ledger_root) = ledger_root {
        for entry in crate::ledger::list_artifacts(ledger_root)? {
            if !hidden.contains(&entry.artifact_ref) {
                insert_bounded(&mut candidates, entry.artifact_ref, MAX_CATALOG_REFS, "catalog visible candidates")?;
            }
        }
    }
    let mut candidate_refs = Vec::new();
    for candidate in candidates {
        push_bounded(&mut candidate_refs, candidate, MAX_CATALOG_REFS, "catalog visible candidates")?;
    }
    Ok(candidate_refs)
}

fn finish_query(
    operation: &str,
    query_value: IoValue,
    items: Vec<IoValue>,
    diagnostics: Vec<String>,
) -> Result<QueryResult> {
    let query_ref = canonical_hash(&query_value)?;
    let decision = "pass";
    let result_value = result_value(&query_ref, decision, &items, &diagnostics, &[
        ("visibility-filtered", "pass"),
        ("canonical-result-ref", "pass"),
        ("no-name-identity", "pass"),
    ])?;
    let result_ref = canonical_hash(&result_value)?;
    let mut refs = Vec::new();
    push_bounded(&mut refs, query_ref.clone(), MAX_CATALOG_REFS, "catalog receipt refs")?;
    push_bounded(&mut refs, result_ref.clone(), MAX_CATALOG_REFS, "catalog receipt refs")?;
    for item in &items {
        push_bounded(&mut refs, canonical_hash(item)?, MAX_CATALOG_REFS, "catalog receipt refs")?;
    }
    let receipt_value = build_receipt_value(&ReceiptValueInput {
        operation,
        decision,
        query_ref: &query_ref,
        result_ref: Some(&result_ref),
        refs: &refs,
        diagnostics: &diagnostics,
        checks: &[
            ("canonical-result-ref", "pass"),
            ("visibility-filtered", "pass"),
            ("no-name-identity", "pass"),
        ],
    })?;
    Ok(QueryResult {
        query_ref,
        result_ref,
        decision: decision.to_string(),
        items,
        diagnostics,
        value: result_value,
        receipt_value,
    })
}

struct SummaryValueInput<'a> {
    artifact_ref: &'a str,
    artifact_kind: &'a str,
    payload_ref: &'a str,
    name_refs: &'a [String],
    schema_refs: &'a [String],
    dependency_refs: &'a [String],
    dependent_refs: &'a [String],
    effect_manifest_ref: Option<&'a str>,
    policy_refs: &'a [String],
    evidence_refs: &'a [String],
    classifications: &'a [String],
    visibility_decision: &'a str,
    redaction_profile_ref: Option<&'a str>,
}

struct QueryValueInput<'a> {
    operation: &'a str,
    root_refs: &'a [String],
    include_dependencies: bool,
    include_dependents: bool,
    filters: &'a [Filter],
    visibility: &'a VisibilityInput,
    render_mode: &'a str,
    include_payload: bool,
}

struct ReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    query_ref: &'a str,
    result_ref: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn build_summary_value(input: &SummaryValueInput<'_>) -> Result<IoValue> {
    validate_ref(input.artifact_ref, "catalog artifact ref")?;
    validate_non_empty(input.artifact_kind, "catalog artifact kind")?;
    validate_ref(input.payload_ref, "catalog payload ref")?;
    validate_refs(input.name_refs, "catalog name ref")?;
    validate_refs(input.schema_refs, "catalog schema ref")?;
    validate_refs(input.dependency_refs, "catalog dependency ref")?;
    validate_refs(input.dependent_refs, "catalog dependent ref")?;
    if let Some(effect_manifest_ref) = input.effect_manifest_ref {
        validate_ref(effect_manifest_ref, "catalog effect ref")?;
    }
    validate_refs(input.policy_refs, "catalog policy ref")?;
    validate_refs(input.evidence_refs, "catalog evidence ref")?;
    Ok(record("catalog-summary-v1", vec![
        string(crate::preserves_rail::CATALOG_SUMMARY_SCHEMA),
        record("artifact", vec![
            string(input.artifact_ref),
            string(input.artifact_kind),
            string(input.payload_ref),
        ]),
        record("names", vec![refs_sequence(input.name_refs)]),
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("dependencies", vec![refs_sequence(input.dependency_refs)]),
        record("dependents", vec![refs_sequence(input.dependent_refs)]),
        record("effects", vec![optional_ref_value(input.effect_manifest_ref)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("evidence", vec![refs_sequence(input.evidence_refs)]),
        record("classifications", vec![sequence(input.classifications.iter().map(string).collect())]),
        record("visibility", vec![
            string(input.visibility_decision),
            optional_ref_value(input.redaction_profile_ref),
        ]),
        checks_value(&[
            "full-ref-identity",
            "names-are-metadata",
            "visibility-filtered",
            "redaction-profile-bound",
        ]),
    ]))
}
