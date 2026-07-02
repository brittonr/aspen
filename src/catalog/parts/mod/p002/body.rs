
fn short_id_outcome(prefix: &ShortIdPrefix<'_>, candidates: &[String], min_length: usize) -> ShortIdOutcome {
    match prefix {
        ShortIdPrefix::Deny(message) => ShortIdOutcome {
            decision: "deny".to_string(),
            full_ref: None,
            diagnostics: vec![message.clone()],
        },
        ShortIdPrefix::HexPrefix(hex_prefix) if hex_prefix.len() < min_length => ShortIdOutcome {
            decision: "deny".to_string(),
            full_ref: None,
            diagnostics: vec![format!("short id prefix requires at least {min_length} hex characters")],
        },
        _ if candidates.len() == 1 => ShortIdOutcome {
            decision: "pass".to_string(),
            full_ref: Some(candidates[0].clone()),
            diagnostics: Vec::new(),
        },
        _ if candidates.is_empty() => ShortIdOutcome {
            decision: "deny".to_string(),
            full_ref: None,
            diagnostics: vec!["short id prefix matched no visible refs".to_string()],
        },
        _ => ShortIdOutcome {
            decision: "deny".to_string(),
            full_ref: None,
            diagnostics: vec![format!(
                "short id prefix is ambiguous across {} visible refs",
                candidates.len()
            )],
        },
    }
}

struct ShortIdResultInput<'a, 'p> {
    query_ref: &'a str,
    prefix: &'a ShortIdPrefix<'p>,
    min_length: usize,
    value: &'a IoValue,
    outcome: &'a ShortIdOutcome,
    candidates: &'a [String],
}

fn short_id_result_value(input: ShortIdResultInput<'_, '_>) -> Result<IoValue> {
    result_value(
        input.query_ref,
        &input.outcome.decision,
        std::slice::from_ref(input.value),
        &input.outcome.diagnostics,
        &[
            ("short-id-minimum", short_id_minimum_status(input.prefix, input.min_length)),
            ("ambiguity-denial", if input.candidates.len() <= 1 { "pass" } else { "fail" }),
            ("visible-candidates-only", "pass"),
        ],
    )
}

fn short_id_minimum_status(prefix: &ShortIdPrefix<'_>, min_length: usize) -> &'static str {
    match prefix {
        ShortIdPrefix::FullRef => "pass",
        ShortIdPrefix::HexPrefix(hex_prefix) if hex_prefix.len() >= min_length => "pass",
        ShortIdPrefix::HexPrefix(_) | ShortIdPrefix::Deny(_) => "fail",
    }
}

fn short_id_refs(candidates: &[String], query_ref: &str, result_ref: &str) -> Result<Vec<String>> {
    let mut refs = Vec::new();
    for candidate in candidates {
        push_bounded(&mut refs, candidate.clone(), MAX_CATALOG_REFS, "catalog short-id refs")?;
    }
    push_bounded(&mut refs, query_ref.to_string(), MAX_CATALOG_REFS, "catalog short-id refs")?;
    push_bounded(&mut refs, result_ref.to_string(), MAX_CATALOG_REFS, "catalog short-id refs")?;
    Ok(refs)
}

fn short_id_receipt_value(
    query_ref: &str,
    result_ref: &str,
    refs: &[String],
    outcome: &ShortIdOutcome,
) -> Result<IoValue> {
    build_receipt_value(&ReceiptValueInput {
        operation: "short-id",
        decision: &outcome.decision,
        query_ref,
        result_ref: Some(result_ref),
        refs,
        diagnostics: &outcome.diagnostics,
        checks: &[
            ("canonical-result-ref", "pass"),
            ("full-ref-expansion", if outcome.full_ref.is_some() { "pass" } else { "fail" }),
            ("no-name-identity", "pass"),
        ],
    })
}

pub fn parse_receipt(value: &IoValue) -> Result<Receipt> {
    let fields = value
        .collect_simple_record("catalog-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <catalog-receipt-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::CATALOG_RECEIPT_SCHEMA, "catalog receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-receipt", "catalog receipt")?;
    Ok(Receipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        query_ref: record_ref(&fields[3], "query")?,
        result_ref: record_optional_ref(&fields[4], "result")?,
        refs: record_ref_sequence(&fields[5], "refs")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn summary(value: &IoValue) -> Result<String> {
    if let Ok(receipt) = parse_receipt(value) {
        return Ok(format!(
            "catalog receipt operation={} decision={} query={} result={}",
            receipt.operation,
            receipt.decision,
            receipt.query_ref,
            receipt.result_ref.as_deref().unwrap_or("<none>")
        ));
    }
    if let Some(fields) = value.collect_simple_record("catalog-result-v1", Some(6)) {
        require_schema(&fields[0], crate::preserves_rail::CATALOG_RESULT_SCHEMA, "catalog result")?;
        let items_value = value_to_iovalue(&fields[3]);
        let items = simple_record(&items_value, "results", 1)?;
        let count = required_sequence(&items[0], "catalog result items")?.len();
        return Ok(format!("catalog result ref={} items={count}", canonical_hash(value)?));
    }
    if value.collect_simple_record("catalog-summary-v1", Some(11)).is_some() {
        return Ok(format!("catalog summary ref={}", canonical_hash(value)?));
    }
    if value.collect_simple_record("catalog-view-v1", Some(7)).is_some() {
        return Ok(format!("catalog view ref={}", canonical_hash(value)?));
    }
    if value.collect_simple_record("chunk-store-catalog-v1", Some(7)).is_some() {
        return Ok(format!("chunk store catalog ref={}", canonical_hash(value)?));
    }
    if value.collect_simple_record("chunk-manifest-catalog-v1", Some(8)).is_some() {
        return Ok(format!("chunk manifest catalog ref={}", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported catalog artifact for show"))
}

fn graph_query(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    input: &GraphInput,
    operation: &str,
) -> Result<QueryResult> {
    validate_visibility(&input.visibility)?;
    let full_ref = resolve_reference(registry_root, ledger_root, &input.reference, &input.visibility)?;
    let query_value = build_query_value(&QueryValueInput {
        operation,
        root_refs: std::slice::from_ref(&full_ref),
        include_dependencies: operation == "deps" && input.transitive,
        include_dependents: operation == "dependents" && input.transitive,
        filters: &[Filter::Ref(full_ref.clone())],
        visibility: &input.visibility,
        render_mode: "summary",
        include_payload: false,
    })?;
    let refs = if operation == "deps" {
        if input.transitive {
            let closure = crate::artifacts::dependency_closure(registry_root, std::slice::from_ref(&full_ref))?;
            closure.closure_refs.into_iter().filter(|item| item != &full_ref).collect::<Vec<_>>()
        } else {
            crate::artifacts::direct_dependencies(registry_root, &full_ref)?
        }
    } else if input.transitive {
        crate::artifacts::impact_refs(registry_root, std::slice::from_ref(&full_ref))?
            .into_iter()
            .filter(|item| item != &full_ref)
            .collect::<Vec<_>>()
    } else {
        direct_dependents(registry_root, &full_ref)?
    };
    let hidden = hidden_set(&input.visibility);
    let summaries = collect_summaries(registry_root, ledger_root, &input.visibility)?;
    let items = refs
        .into_iter()
        .filter(|item| !hidden.contains(item))
        .filter_map(|reference| summaries.iter().find(|summary| summary.artifact_ref == reference).cloned())
        .map(|summary| summary.value)
        .collect::<Vec<_>>();
    finish_query(operation, query_value, items, Vec::new())
}

fn append_registry_receipt_views(
    registry_root: &Path,
    subject_ref: &str,
    visibility: &VisibilityInput,
    items: &mut impl crate::bounded::VecSink<IoValue>,
) -> Result<()> {
    for receipt in crate::artifacts::list_receipts(registry_root)? {
        if receipt.subject_ref != subject_ref && !to_text(&receipt.value)?.contains(subject_ref) {
            continue;
        }
        let text = to_text(&receipt.value)?;
        if contains_hidden_ref(&text, visibility) {
            continue;
        }
        push_bounded(
            items,
            maybe_redacted_value(&receipt.value, visibility.redaction_profile_ref.as_deref())?,
            MAX_CATALOG_ITEMS,
            "catalog receipt items",
        )?;
    }
    Ok(())
}

fn append_ledger_receipt_views(
    ledger_root: &Path,
    subject_ref: &str,
    visibility: &VisibilityInput,
    items: &mut impl crate::bounded::VecSink<IoValue>,
) -> Result<()> {
    for entry in crate::ledger::list_artifacts(ledger_root)? {
        if hidden_set(visibility).contains(&entry.artifact_ref) {
            continue;
        }
        let value = crate::ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
        let kind = crate::ledger::artifact_kind(&value);
        if !kind.contains("receipt") {
            continue;
        }
        let text = to_text(&value)?;
        if !text.contains(subject_ref) || contains_hidden_ref(&text, visibility) {
            continue;
        }
        push_bounded(
            items,
            maybe_redacted_value(&value, visibility.redaction_profile_ref.as_deref())?,
            MAX_CATALOG_ITEMS,
            "catalog receipt items",
        )?;
    }
    Ok(())
}

fn collect_summaries(
    registry_root: &Path,
    ledger_root: Option<&Path>,
    visibility: &VisibilityInput,
) -> Result<Vec<Summary>> {
    let hidden = hidden_set(visibility);
    let mut summaries = Vec::new();
    let mut seen = Set::new();
    for artifact in crate::artifacts::list_artifacts(registry_root, None)? {
        if hidden.contains(&artifact.artifact_ref) {
            continue;
        }
        checked_count_sum(seen.len(), 1, MAX_CATALOG_ITEMS, "catalog summary refs")?;
        seen.insert(artifact.artifact_ref.clone());
        push_bounded(
            &mut summaries,
            registry_summary(registry_root, ledger_root, artifact, visibility)?,
            MAX_CATALOG_ITEMS,
            "catalog summaries",
        )?;
    }
    if let Some(ledger_root) = ledger_root {
        for entry in crate::ledger::list_artifacts(ledger_root)? {
            if seen.contains(&entry.artifact_ref) || hidden.contains(&entry.artifact_ref) {
                continue;
            }
            let value = crate::ledger::read_artifact(ledger_root, &entry.artifact_ref)?;
            push_bounded(
                &mut summaries,
                ledger_summary(registry_root, ledger_root, &entry.artifact_ref, value, visibility)?,
                MAX_CATALOG_ITEMS,
                "catalog summaries",
            )?;
        }
    }
    summaries.sort_by(|left, right| left.artifact_ref.cmp(&right.artifact_ref));
    Ok(summaries)
}
