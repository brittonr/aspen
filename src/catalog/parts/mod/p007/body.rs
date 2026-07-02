
fn view_value(
    summary: &Summary,
    summary_value: &IoValue,
    payload_or_value: &IoValue,
    include_payload: bool,
    redacted: bool,
) -> Result<IoValue> {
    Ok(record("catalog-view-v1", vec![
        string(crate::preserves_rail::CATALOG_VIEW_SCHEMA),
        record("artifact", vec![string(&summary.artifact_ref), string(&summary.artifact_kind)]),
        record("summary", vec![summary_value.clone()]),
        record("content", vec![payload_or_value.clone()]),
        record("render", vec![
            bool_value(include_payload),
            string(if redacted { "redacted" } else { "raw" }),
        ]),
        record("classifications", vec![sequence(summary.classifications.iter().map(string).collect())]),
        checks_value(&["full-ref-identity", "redacted-before-render", "no-name-identity"]),
    ]))
}

fn build_query_value(input: &QueryValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "catalog operation")?;
    validate_refs(input.root_refs, "catalog query root ref")?;
    validate_filters(input.filters)?;
    validate_visibility(input.visibility)?;
    Ok(record("catalog-query-v1", vec![
        string(crate::preserves_rail::CATALOG_QUERY_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("scope", vec![
            refs_sequence(input.root_refs),
            bool_value(input.include_dependencies),
            bool_value(input.include_dependents),
        ]),
        record("filters", vec![sequence(
            input.filters.iter().map(filter_value).collect::<Result<Vec<_>>>()?,
        )]),
        record("visibility", vec![
            refs_sequence(&input.visibility.policy_refs),
            refs_sequence(&input.visibility.capability_refs),
            refs_sequence(&input.visibility.hidden_refs),
            optional_ref_value(input.visibility.redaction_profile_ref.as_deref()),
        ]),
        record("render", vec![string(input.render_mode), bool_value(input.include_payload)]),
        checks_value(&[
            "no-name-identity",
            "visibility-filtered",
            "bounded-query",
            "short-id-ui-only",
        ]),
    ]))
}

fn result_value(
    query_ref: &str,
    decision: &str,
    items: &[IoValue],
    diagnostics: &[String],
    checks: &[(&str, &str)],
) -> Result<IoValue> {
    validate_ref(query_ref, "catalog result query ref")?;
    validate_decision(decision)?;
    Ok(record("catalog-result-v1", vec![
        string(crate::preserves_rail::CATALOG_RESULT_SCHEMA),
        record("query", vec![string(query_ref)]),
        record("decision", vec![string(decision)]),
        record("results", vec![sequence(items.to_vec())]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(checks),
    ]))
}

fn build_receipt_value(input: &ReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "catalog receipt operation")?;
    validate_decision(input.decision)?;
    validate_ref(input.query_ref, "catalog receipt query ref")?;
    if let Some(result_ref) = input.result_ref {
        validate_ref(result_ref, "catalog receipt result ref")?;
    }
    validate_refs(input.refs, "catalog receipt ref")?;
    ensure_count_at_most(input.checks.len(), MAX_CATALOG_CHECKS, "catalog receipt checks")?;
    let mut all_checks = Vec::new();
    push_bounded(&mut all_checks, ("canonical-receipt", "pass"), MAX_CATALOG_CHECKS, "catalog receipt checks")?;
    for check in input.checks {
        push_bounded(&mut all_checks, *check, MAX_CATALOG_CHECKS, "catalog receipt checks")?;
    }
    Ok(record("catalog-receipt-v1", vec![
        string(crate::preserves_rail::CATALOG_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("query", vec![string(input.query_ref)]),
        record("result", vec![optional_ref_value(input.result_ref)]),
        record("refs", vec![refs_sequence(&sorted_unique(input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(&all_checks),
    ]))
}

fn short_id_resolution_value(
    prefix: &str,
    full_ref: Option<&str>,
    candidates: &[String],
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    validate_non_empty(prefix, "catalog short id prefix")?;
    if let Some(full_ref) = full_ref {
        validate_ref(full_ref, "catalog short id full ref")?;
    }
    validate_refs(candidates, "catalog short id candidate ref")?;
    validate_decision(decision)?;
    Ok(record("short-id-resolution-v1", vec![
        string(crate::preserves_rail::CATALOG_SHORT_ID_SCHEMA),
        record("prefix", vec![string(prefix)]),
        record("full-ref", vec![optional_ref_value(full_ref)]),
        record("candidates", vec![refs_sequence(candidates)]),
        record("candidate-count", vec![crate::preserves_rail::u64_value(candidates.len() as u64)]),
        record("decision", vec![string(decision)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        checks_value(&["short-id-ui-only", "visible-candidates-only", "ambiguity-denial"]),
    ]))
}

struct ChunkStoreStatusInput<'a> {
    manifest_refs: &'a [String],
    total_chunk_refs: usize,
    unique_chunks: usize,
    available_chunks: usize,
    missing_chunks: usize,
    manifest_pins: usize,
    chunk_pins: usize,
    dedup_hits: usize,
}

fn chunk_store_status_value(input: &ChunkStoreStatusInput<'_>) -> Result<IoValue> {
    validate_refs(input.manifest_refs, "chunk catalog manifest ref")?;
    Ok(record("chunk-store-catalog-v1", vec![
        string("molten.catalog.chunk-store.v1"),
        record("manifests", vec![
            crate::preserves_rail::u64_value(usize_to_u64(input.manifest_refs.len(), "chunk catalog manifests")?),
            refs_sequence(input.manifest_refs),
        ]),
        record("chunks", vec![
            crate::preserves_rail::u64_value(usize_to_u64(input.total_chunk_refs, "chunk catalog total chunks")?),
            crate::preserves_rail::u64_value(usize_to_u64(input.unique_chunks, "chunk catalog unique chunks")?),
            crate::preserves_rail::u64_value(usize_to_u64(input.available_chunks, "chunk catalog available chunks")?),
            crate::preserves_rail::u64_value(usize_to_u64(input.missing_chunks, "chunk catalog missing chunks")?),
        ]),
        record("pins", vec![
            crate::preserves_rail::u64_value(usize_to_u64(input.manifest_pins, "chunk catalog manifest pins")?),
            crate::preserves_rail::u64_value(usize_to_u64(input.chunk_pins, "chunk catalog chunk pins")?),
        ]),
        record("dedup", vec![
            crate::preserves_rail::u64_value(usize_to_u64(input.dedup_hits, "chunk catalog dedup hits")?),
            crate::preserves_rail::u64_value(dedup_ratio_bps(input.total_chunk_refs, input.dedup_hits)?),
        ]),
        record("classifications", vec![sequence(vec![
            string("chunk-store:status"),
            string("chunk-store:availability"),
            string("chunk-store:dedup"),
            string("chunk-store:pins"),
        ])]),
        checks_value(&["chunk-availability-visible", "pin-state-visible", "dedup-ratio-derived"]),
    ]))
}

struct ChunkManifestInput<'a> {
    manifest: &'a crate::chunk_store::ChunkManifest,
    is_manifest_pinned: bool,
    available_chunks: usize,
    missing_chunks: usize,
    chunk_pins: usize,
    hidden_chunk_count: usize,
    visible_chunks: &'a [String],
}

fn chunk_manifest_value(input: &ChunkManifestInput<'_>) -> Result<IoValue> {
    let manifest = input.manifest;
    validate_ref(&manifest.manifest_ref, "chunk catalog manifest ref")?;
    validate_ref(&manifest.root_ref, "chunk catalog root ref")?;
    validate_refs(input.visible_chunks, "chunk catalog chunk ref")?;
    let availability = if input.missing_chunks == 0 {
        "complete"
    } else if input.available_chunks == 0 {
        "missing"
    } else {
        "partial"
    };
    Ok(record("chunk-manifest-catalog-v1", vec![
        string("molten.catalog.chunk-manifest.v1"),
        record("manifest", vec![
            string(&manifest.manifest_ref),
            string(&manifest.object_kind),
            string(&manifest.root_ref),
        ]),
        record("size", vec![
            crate::preserves_rail::u64_value(manifest.total_len),
            crate::preserves_rail::u64_value(manifest.chunk_size),
            crate::preserves_rail::u64_value(usize_to_u64(manifest.chunks.len(), "chunk catalog manifest chunks")?),
        ]),
        record("availability", vec![
            string(availability),
            crate::preserves_rail::u64_value(usize_to_u64(input.available_chunks, "chunk catalog available chunks")?),
            crate::preserves_rail::u64_value(usize_to_u64(input.missing_chunks, "chunk catalog missing chunks")?),
            refs_sequence(input.visible_chunks),
        ]),
        record("pins", vec![
            bool_value(input.is_manifest_pinned),
            crate::preserves_rail::u64_value(usize_to_u64(input.chunk_pins, "chunk catalog chunk pins")?),
        ]),
        record("redaction", vec![crate::preserves_rail::u64_value(usize_to_u64(
            input.hidden_chunk_count,
            "chunk catalog hidden chunks",
        )?)]),
        record("classifications", vec![sequence(vec![
            string("chunk-store:manifest"),
            string(format!("chunk-store-object-kind:{}", manifest.object_kind)),
            string(format!("chunk-store-availability:{availability}")),
            string(if input.is_manifest_pinned {
                "chunk-store-pin:pinned"
            } else {
                "chunk-store-pin:unpinned"
            }),
        ])]),
        checks_value(&["manifest-identity", "chunk-availability-visible", "pin-state-visible"]),
    ]))
}

fn dedup_ratio_bps(total_chunk_refs: usize, dedup_hits: usize) -> Result<u64> {
    if total_chunk_refs == 0 {
        return Ok(0);
    }
    let numerator = dedup_hits
        .checked_mul(10_000)
        .ok_or_else(|| MoltenError::invalid_harness("chunk catalog dedup ratio overflow"))?;
    let ratio = numerator
        .checked_div(total_chunk_refs)
        .ok_or_else(|| MoltenError::invalid_harness("chunk catalog dedup ratio divisor is zero"))?;
    usize_to_u64(ratio, "chunk catalog dedup ratio")
}

fn usize_to_u64(value: usize, label: &str) -> Result<u64> {
    u64::try_from(value).map_err(|_| MoltenError::invalid_harness(format!("{label} count exceeds u64")))
}

fn filter_value(filter: &Filter) -> Result<IoValue> {
    let (kind, value) = match filter {
        Filter::Ref(value) => ("ref", value.as_str()),
        Filter::ArtifactKind(value) => ("artifact-kind", value.as_str()),
        Filter::LedgerKind(value) => ("ledger-kind", value.as_str()),
        Filter::SchemaRef(value) => ("schema-ref", value.as_str()),
        Filter::StructuralFingerprint(value) => ("structural-fingerprint", value.as_str()),
        Filter::EffectRef(value) => ("effect-ref", value.as_str()),
        Filter::PolicyRef(value) => ("policy-ref", value.as_str()),
        Filter::CapabilityRef(value) => ("capability-ref", value.as_str()),
        Filter::EvidenceRef(value) => ("evidence-ref", value.as_str()),
        Filter::DependencyRef(value) => ("dependency-ref", value.as_str()),
        Filter::DependentRef(value) => ("dependent-ref", value.as_str()),
        Filter::ReceiptOperation(value) => ("receipt-operation", value.as_str()),
        Filter::ReceiptDecision(value) => ("receipt-decision", value.as_str()),
        Filter::TranscriptStatus(value) => ("transcript-status", value.as_str()),
        Filter::UpgradeStatus(value) => ("upgrade-status", value.as_str()),
        Filter::Text(value) => ("text", value.as_str()),
    };
    Ok(record("filter", vec![string(kind), string(value)]))
}

fn maybe_redacted_value(value: &IoValue, redaction_profile_ref: Option<&str>) -> Result<IoValue> {
    crate::secrets::redacted_value(value, redaction_profile_ref)
}

fn payload_identity(payload: &ArtifactPayloadRef) -> String {
    match payload {
        ArtifactPayloadRef::Inline { value_ref, .. } => value_ref.clone(),
        ArtifactPayloadRef::ContentRef { manifest_ref, .. } => manifest_ref.clone(),
    }
}

fn receipt_field_matches(text: &str, field: &str, value: &str) -> bool {
    text.contains(&format!("<{field} \"{value}\">")) || text.contains(&format!("<{field} {value}"))
}

fn hidden_set(visibility: &VisibilityInput) -> Set<String> {
    visibility.hidden_refs.iter().cloned().collect()
}

fn contains_hidden_ref(text: &str, visibility: &VisibilityInput) -> bool {
    visibility.hidden_refs.iter().any(|hidden_ref| text.contains(hidden_ref))
}

fn is_full_ref(value: &str) -> bool {
    validate_content_ref(value).is_ok()
}

enum ShortIdPrefix<'a> {
    FullRef,
    HexPrefix(&'a str),
    Deny(String),
}
