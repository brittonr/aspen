
pub fn classify_idempotency_decision(input: DecisionLawInput<'_>) -> Result<IdempotencyDecisionLaw> {
    validate_refs(input.evidence_refs, "delivery decision evidence ref")?;
    let law = if let Some(entry) = input.existing_entry {
        let has_same_operation = entry.operation_ref == input.operation.operation_ref;
        let has_same_payload = entry.payload_ref == input.operation.payload_ref;
        let has_same_evidence = entry.evidence_refs == input.evidence_refs;
        if has_same_operation && has_same_payload && has_same_evidence {
            IdempotencyDecisionLaw {
                kind: IdempotencyDecisionKind::Duplicate,
                prior_receipt_ref: Some(entry.first_receipt_ref.clone()),
                prior_semantic_result_ref: entry.semantic_result_ref.clone(),
                diagnostics: Vec::new(),
                should_commit_side_effect: false,
            }
        } else {
            IdempotencyDecisionLaw {
                kind: IdempotencyDecisionKind::Conflict,
                prior_receipt_ref: Some(entry.first_receipt_ref.clone()),
                prior_semantic_result_ref: None,
                diagnostics: vec!["delivery operation sequence reused with different payload or evidence".to_string()],
                should_commit_side_effect: false,
            }
        }
    } else if input.operation.sequence < input.window.next_sequence {
        IdempotencyDecisionLaw {
            kind: IdempotencyDecisionKind::Stale,
            prior_receipt_ref: None,
            prior_semantic_result_ref: None,
            diagnostics: vec![format!(
                "delivery sequence {} is stale for window next {}",
                input.operation.sequence, input.window.next_sequence
            )],
            should_commit_side_effect: false,
        }
    } else if input.operation.sequence > input.window.next_sequence {
        let kind = match input.gap_policy {
            GapPolicy::Deny => IdempotencyDecisionKind::Gap,
            GapPolicy::Retry => IdempotencyDecisionKind::Retry,
        };
        IdempotencyDecisionLaw {
            kind,
            prior_receipt_ref: None,
            prior_semantic_result_ref: None,
            diagnostics: vec![format!(
                "delivery sequence {} leaves gap before expected {}",
                input.operation.sequence, input.window.next_sequence
            )],
            should_commit_side_effect: false,
        }
    } else {
        IdempotencyDecisionLaw {
            kind: IdempotencyDecisionKind::First,
            prior_receipt_ref: None,
            prior_semantic_result_ref: None,
            diagnostics: Vec::new(),
            should_commit_side_effect: true,
        }
    };
    Ok(law)
}

pub fn check(input: CheckInput<'_>) -> Result<Decision> {
    let root = crate::local_store::DeliveryStoreRoot::open(input.root)?;
    check_request(&root, input.request())
}

pub fn check_with_root(input: CapabilityCheckInput<'_>) -> Result<Decision> {
    check_request(input.root, input.request)
}

fn check_request(root: &crate::local_store::DeliveryStoreRoot, input: CheckRequest<'_>) -> Result<Decision> {
    validate_scope_profile(input.scope_profile)?;
    require_ref(input.scope_ref, "delivery scope ref")?;
    validate_refs(input.policy_refs, "delivery policy ref")?;
    validate_refs(input.evidence_refs, "delivery evidence ref")?;
    if let Some(result_ref) = input.semantic_result_ref {
        require_ref(result_ref, "delivery semantic result ref")?;
    }
    let operation = derive_operation_id(OperationIdInput {
        scope_ref: input.scope_ref.to_owned(),
        producer: input.producer.to_owned(),
        consumer: input.consumer.to_owned(),
        sequence: input.sequence,
        intent: input.intent.to_owned(),
        payload_ref: input.payload_ref.to_owned(),
        policy_refs: input.policy_refs.to_vec(),
    })?;
    let dedup_key = dedup_key_ref(&operation)?;
    let db = ensure_store_tables_with_root(root)?;
    let existing_entry = read_entry_from_store(&db, &dedup_key)?;
    let current_window = read_or_create_window(&db, input.scope_profile, input.scope_ref, input.policy_refs)?;
    let law = classify_idempotency_decision(DecisionLawInput {
        operation: &operation,
        window: &current_window,
        existing_entry: existing_entry.as_ref(),
        evidence_refs: input.evidence_refs,
        gap_policy: input.gap_policy,
    })?;
    let decision = match law.kind {
        IdempotencyDecisionKind::First => first_decision(input, &db, operation, current_window, dedup_key)?,
        IdempotencyDecisionKind::Duplicate | IdempotencyDecisionKind::Conflict => {
            let entry = existing_entry.ok_or_else(|| {
                MoltenError::invalid_harness("delivery idempotency invariant violated: duplicate/conflict without entry")
            })?;
            duplicate_or_conflict_decision(input, &db, operation, current_window, entry, &law)?
        }
        IdempotencyDecisionKind::Stale | IdempotencyDecisionKind::Gap | IdempotencyDecisionKind::Retry => {
            suppressed_decision_from_law(&db, operation, current_window, &law)?
        }
    };
    Ok(decision)
}

pub fn read_idempotency_receipt(root: &std::path::Path, receipt_ref: &str) -> Result<IoValue> {
    require_ref(receipt_ref, "delivery idempotency receipt ref")?;
    let db = ensure_store_tables(root)?;
    let read_txn = db.begin_read().map_err(store_error)?;
    let table = read_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
    let Some(bytes) = table.get(receipt_ref).map_err(store_error)? else {
        return Err(MoltenError::invalid_harness(format!("unknown delivery idempotency receipt {receipt_ref}")));
    };
    crate::preserves_rail::parse_canonical_bytes(bytes.value())
}

pub fn retry_receipt_value(operation: &OperationId, window: &Window, diagnostics: &[String]) -> Result<IoValue> {
    validate_diagnostics(diagnostics)?;
    Ok(record("retry-receipt-v1", vec![
        string(crate::preserves_rail::DELIVERY_RETRY_RECEIPT_SCHEMA),
        record("operation", vec![string(&operation.operation_ref)]),
        record("scope", vec![string(&operation.scope_ref)]),
        record("window", vec![string(&window.window_ref)]),
        record("retry-after-sequence", vec![crate::preserves_rail::u64_value(window.next_sequence)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[("retry-before-side-effects", "pass"), ("sequence-window-bound", "pass")]),
    ]))
}

pub fn parse_receipt(value: &IoValue) -> Result<Receipt> {
    let fields = value
        .collect_simple_record("delivery-idempotency-receipt-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <delivery-idempotency-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::DELIVERY_IDEMPOTENCY_RECEIPT_SCHEMA,
        "delivery idempotency receipt schema",
    )?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision)?;
    let side_effect = record_string(&fields[7], "side-effect")?;
    validate_side_effect(&side_effect)?;
    require_check(&parse_checks(&fields[9])?, "dedup-before-commit", "delivery idempotency receipt")?;
    Ok(Receipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        operation_ref: record_ref(&fields[2], "operation")?,
        scope_ref: record_ref(&fields[3], "scope")?,
        window_ref: record_ref(&fields[4], "window")?,
        prior_receipt_ref: record_optional_ref(&fields[5], "prior")?,
        semantic_result_ref: record_optional_ref(&fields[6], "semantic-result")?,
        side_effect,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn parse_dedup_entry(value: &IoValue) -> Result<DedupEntry> {
    let fields = value
        .collect_simple_record("dedup-entry-v1", Some(13))
        .ok_or_else(|| MoltenError::invalid_harness("expected <dedup-entry-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::DELIVERY_DEDUP_ENTRY_SCHEMA, "delivery dedup entry schema")?;
    require_check(&parse_checks(&fields[12])?, "first-receipt-bound", "delivery dedup entry")?;
    Ok(DedupEntry {
        entry_ref: crate::preserves_rail::canonical_hash(value)?,
        dedup_key: record_ref(&fields[1], "dedup-key")?,
        operation_ref: record_ref(&fields[2], "operation")?,
        scope_ref: record_ref(&fields[3], "scope")?,
        producer: record_string(&fields[4], "producer")?,
        consumer: record_string(&fields[5], "consumer")?,
        sequence: record_u64(&fields[6], "sequence")?,
        intent: record_string(&fields[7], "intent")?,
        payload_ref: record_ref(&fields[8], "payload")?,
        semantic_result_ref: record_optional_ref(&fields[9], "semantic-result")?,
        first_receipt_ref: record_ref(&fields[10], "first-receipt")?,
        evidence_refs: record_ref_sequence(&fields[11], "evidence")?,
        value: value.clone(),
    })
}

pub fn summary(value: &IoValue) -> Result<String> {
    if let Ok(operation) = parse_operation_id(value) {
        return Ok(format!(
            "delivery operation ref={} scope={} producer={} consumer={} sequence={} intent={} payload={}",
            operation.operation_ref,
            operation.scope_ref,
            operation.producer,
            operation.consumer,
            operation.sequence,
            operation.intent,
            operation.payload_ref
        ));
    }
    if let Ok(window) = parse_window(value) {
        return Ok(format!(
            "delivery window ref={} scope={} profile={} next_sequence={} lowest_retained={} retention_refs={}",
            window.window_ref,
            window.scope_ref,
            window.scope_profile,
            window.next_sequence,
            window.lowest_retained,
            window.retention_refs.len()
        ));
    }
    if let Ok(entry) = parse_dedup_entry(value) {
        return Ok(format!(
            "delivery dedup entry ref={} operation={} scope={} sequence={} first_receipt={} evidence_refs={}",
            entry.entry_ref,
            entry.operation_ref,
            entry.scope_ref,
            entry.sequence,
            entry.first_receipt_ref,
            entry.evidence_refs.len()
        ));
    }
    if let Ok(receipt) = parse_receipt(value) {
        return Ok(format!(
            "delivery idempotency receipt ref={} decision={} operation={} scope={} side_effect={} diagnostics={}",
            receipt.receipt_ref,
            receipt.decision,
            receipt.operation_ref,
            receipt.scope_ref,
            receipt.side_effect,
            receipt.diagnostics.len()
        ));
    }
    if let Some(fields) = value.collect_simple_record("retry-receipt-v1", Some(7)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::DELIVERY_RETRY_RECEIPT_SCHEMA,
            "delivery retry receipt schema",
        )?;
        require_check(&parse_checks(&fields[6])?, "retry-before-side-effects", "delivery retry receipt")?;
        return Ok(format!(
            "delivery retry receipt ref={} operation={} scope={} retry_after_sequence={} diagnostics={}",
            crate::preserves_rail::canonical_hash(value)?,
            record_ref(&fields[1], "operation")?,
            record_ref(&fields[2], "scope")?,
            record_u64(&fields[4], "retry-after-sequence")?,
            record_string_sequence(&fields[5], "diagnostics")?.len()
        ));
    }
    Err(MoltenError::invalid_harness("unsupported delivery artifact"))
}

fn first_decision(
    input: CheckRequest<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: Window,
    dedup_key: String,
) -> Result<Decision> {
    let next_sequence = operation
        .sequence
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("delivery sequence overflow"))?;
    let updated_window = parse_window(&window_value(
        input.scope_profile,
        input.scope_ref,
        next_sequence,
        window.lowest_retained,
        input.policy_refs,
    )?)?;
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision: "first",
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &updated_window.window_ref,
        prior_receipt_ref: None,
        semantic_result_ref: input.semantic_result_ref,
        side_effect: "commit",
        diagnostics: &[],
        checks: &[
            ("dedup-before-commit", "pass"),
            ("sequence-window-advanced", "pass"),
            ("retention-pinned", "pass"),
        ],
    })?;
    let receipt = parse_receipt(&receipt_value)?;
    let entry_value = dedup_entry_value(DedupEntryValueInput {
        dedup_key: &dedup_key,
        operation: &operation,
        semantic_result_ref: input.semantic_result_ref,
        first_receipt_ref: &receipt.receipt_ref,
        evidence_refs: input.evidence_refs,
    })?;
    let entry = parse_dedup_entry(&entry_value)?;
    store_first_decision(db, &updated_window, &entry, &receipt)?;
    Ok(Decision {
        operation,
        window: updated_window,
        receipt,
        entry: Some(entry),
        should_commit_side_effect: true,
        prior_semantic_result_ref: None,
    })
}

fn duplicate_or_conflict_decision(
    _input: CheckRequest<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: Window,
    entry: DedupEntry,
    law: &IdempotencyDecisionLaw,
) -> Result<Decision> {
    let decision = law.kind.as_str();
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision,
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &window.window_ref,
        prior_receipt_ref: law.prior_receipt_ref.as_deref(),
        semantic_result_ref: law.prior_semantic_result_ref.as_deref(),
        side_effect: "suppress",
        diagnostics: &law.diagnostics,
        checks: &[
            ("dedup-before-commit", "pass"),
            (
                "duplicate-suppresses-side-effects",
                if matches!(law.kind, IdempotencyDecisionKind::Duplicate) { "pass" } else { "n/a" },
            ),
            (
                "conflict-denies-before-side-effects",
                if matches!(law.kind, IdempotencyDecisionKind::Conflict) { "pass" } else { "n/a" },
            ),
        ],
    })?;
    let receipt = parse_receipt(&receipt_value)?;
    store_receipt(db, &receipt)?;
    Ok(Decision {
        operation,
        window,
        receipt,
        entry: Some(entry),
        should_commit_side_effect: law.should_commit_side_effect,
        prior_semantic_result_ref: law.prior_semantic_result_ref.clone(),
    })
}
