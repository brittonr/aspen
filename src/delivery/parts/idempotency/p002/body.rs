
fn gap_or_retry_decision(
    input: CheckInput<'_>,
    db: &redb::Database,
    operation: OperationId,
    window: Window,
) -> Result<Decision> {
    let diagnostics = vec![format!(
        "delivery sequence {} leaves gap before expected {}",
        input.sequence, window.next_sequence
    )];
    let decision = match input.gap_policy {
        GapPolicy::Deny => "gap",
        GapPolicy::Retry => "retry",
    };
    let result = suppressed_decision(db, operation, window, decision, diagnostics)?;
    if matches!(input.gap_policy, GapPolicy::Retry) {
        let retry_value = retry_receipt_value(&result.operation, &result.window, &result.receipt.diagnostics)?;
        let retry_ref = crate::preserves_rail::canonical_hash(&retry_value)?;
        store_raw_receipt(db, &retry_ref, &retry_value)?;
    }
    Ok(result)
}

fn suppressed_decision(
    db: &redb::Database,
    operation: OperationId,
    window: Window,
    decision: &str,
    diagnostics: Vec<String>,
) -> Result<Decision> {
    let receipt_value = idempotency_receipt_value(IdempotencyReceiptValueInput {
        decision,
        operation_ref: &operation.operation_ref,
        scope_ref: &operation.scope_ref,
        window_ref: &window.window_ref,
        prior_receipt_ref: None,
        semantic_result_ref: None,
        side_effect: "suppress",
        diagnostics: &diagnostics,
        checks: &[
            ("dedup-before-commit", "pass"),
            ("sequence-window-bound", "pass"),
            ("no-side-effects", "pass"),
        ],
    })?;
    let receipt = parse_receipt(&receipt_value)?;
    store_receipt(db, &receipt)?;
    Ok(Decision {
        operation,
        window,
        receipt,
        entry: None,
        should_commit_side_effect: false,
        prior_semantic_result_ref: None,
    })
}

fn read_or_create_window(
    db: &redb::Database,
    scope_profile: &str,
    scope_ref: &str,
    retention_refs: &[String],
) -> Result<Window> {
    let read_txn = db.begin_read().map_err(store_error)?;
    let windows = read_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
    if let Some(bytes) = windows.get(scope_ref).map_err(store_error)? {
        let value = crate::preserves_rail::parse_canonical_bytes(bytes.value())?;
        return parse_window(&value);
    }
    drop(windows);
    drop(read_txn);
    let value = window_value(scope_profile, scope_ref, 1, 1, retention_refs)?;
    let window = parse_window(&value)?;
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let bytes = crate::preserves_rail::canonical_bytes(&window.value)?;
        let mut windows = write_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
        windows.insert(scope_ref, bytes.as_slice()).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)?;
    Ok(window)
}

fn read_entry_from_store(db: &redb::Database, dedup_key: &str) -> Result<Option<DedupEntry>> {
    let read_txn = db.begin_read().map_err(store_error)?;
    let entries = read_txn.open_table(STORE_ENTRIES).map_err(store_error)?;
    let Some(bytes) = entries.get(dedup_key).map_err(store_error)? else {
        return Ok(None);
    };
    let value = crate::preserves_rail::parse_canonical_bytes(bytes.value())?;
    parse_dedup_entry(&value).map(Some)
}

fn store_first_decision(db: &redb::Database, window: &Window, entry: &DedupEntry, receipt: &Receipt) -> Result<()> {
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let mut windows = write_txn.open_table(STORE_WINDOWS).map_err(store_error)?;
        let window_bytes = crate::preserves_rail::canonical_bytes(&window.value)?;
        windows.insert(window.scope_ref.as_str(), window_bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut entries = write_txn.open_table(STORE_ENTRIES).map_err(store_error)?;
        let entry_bytes = crate::preserves_rail::canonical_bytes(&entry.value)?;
        entries.insert(entry.dedup_key.as_str(), entry_bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut receipts = write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        let receipt_bytes = crate::preserves_rail::canonical_bytes(&receipt.value)?;
        receipts.insert(receipt.receipt_ref.as_str(), receipt_bytes.as_slice()).map_err(store_error)?;
    }
    {
        let mut pins = write_txn.open_table(STORE_PINS).map_err(store_error)?;
        pins.insert(entry.operation_ref.as_str(), entry.entry_ref.as_bytes()).map_err(store_error)?;
        pins.insert(window.scope_ref.as_str(), window.window_ref.as_bytes()).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)
}

fn store_receipt(db: &redb::Database, receipt: &Receipt) -> Result<()> {
    store_raw_receipt(db, &receipt.receipt_ref, &receipt.value)
}

fn store_raw_receipt(db: &redb::Database, receipt_ref: &str, receipt_value: &IoValue) -> Result<()> {
    let write_txn = db.begin_write().map_err(store_error)?;
    {
        let mut receipts = write_txn.open_table(STORE_RECEIPTS).map_err(store_error)?;
        let bytes = crate::preserves_rail::canonical_bytes(receipt_value)?;
        receipts.insert(receipt_ref, bytes.as_slice()).map_err(store_error)?;
    }
    write_txn.commit().map_err(store_error)
}

fn dedup_entry_value(input: DedupEntryValueInput<'_>) -> Result<IoValue> {
    validate_refs(input.evidence_refs, "delivery dedup evidence ref")?;
    if let Some(result_ref) = input.semantic_result_ref {
        require_ref(result_ref, "delivery dedup semantic result ref")?;
    }
    require_ref(input.first_receipt_ref, "delivery dedup first receipt ref")?;
    Ok(record("dedup-entry-v1", vec![
        string(crate::preserves_rail::DELIVERY_DEDUP_ENTRY_SCHEMA),
        record("dedup-key", vec![string(input.dedup_key)]),
        record("operation", vec![string(&input.operation.operation_ref)]),
        record("scope", vec![string(&input.operation.scope_ref)]),
        record("producer", vec![string(&input.operation.producer)]),
        record("consumer", vec![string(&input.operation.consumer)]),
        record("sequence", vec![crate::preserves_rail::u64_value(input.operation.sequence)]),
        record("intent", vec![string(&input.operation.intent)]),
        record("payload", vec![string(&input.operation.payload_ref)]),
        record("semantic-result", vec![optional_ref_value(input.semantic_result_ref)]),
        record("first-receipt", vec![string(input.first_receipt_ref)]),
        record("evidence", vec![strings_sequence(input.evidence_refs)]),
        checks_value(&[
            ("first-receipt-bound", "pass"),
            ("payload-ref-bound", "pass"),
            ("retention-pinned", "pass"),
        ]),
    ]))
}

struct DedupEntryValueInput<'a> {
    dedup_key: &'a str,
    operation: &'a OperationId,
    semantic_result_ref: Option<&'a str>,
    first_receipt_ref: &'a str,
    evidence_refs: &'a [String],
}

fn idempotency_receipt_value(input: IdempotencyReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    require_ref(input.operation_ref, "delivery idempotency operation ref")?;
    require_ref(input.scope_ref, "delivery idempotency scope ref")?;
    require_ref(input.window_ref, "delivery idempotency window ref")?;
    if let Some(prior) = input.prior_receipt_ref {
        require_ref(prior, "delivery idempotency prior receipt ref")?;
    }
    if let Some(result) = input.semantic_result_ref {
        require_ref(result, "delivery idempotency semantic result ref")?;
    }
    validate_side_effect(input.side_effect)?;
    validate_diagnostics(input.diagnostics)?;
    Ok(record("delivery-idempotency-receipt-v1", vec![
        string(crate::preserves_rail::DELIVERY_IDEMPOTENCY_RECEIPT_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("operation", vec![string(input.operation_ref)]),
        record("scope", vec![string(input.scope_ref)]),
        record("window", vec![string(input.window_ref)]),
        record("prior", vec![optional_ref_value(input.prior_receipt_ref)]),
        record("semantic-result", vec![optional_ref_value(input.semantic_result_ref)]),
        record("side-effect", vec![string(input.side_effect)]),
        record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        record("checks", vec![crate::preserves_rail::sequence(
            input
                .checks
                .iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
    ]))
}

struct IdempotencyReceiptValueInput<'a> {
    decision: &'a str,
    operation_ref: &'a str,
    scope_ref: &'a str,
    window_ref: &'a str,
    prior_receipt_ref: Option<&'a str>,
    semantic_result_ref: Option<&'a str>,
    side_effect: &'a str,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

fn dedup_key_ref(operation: &OperationId) -> Result<String> {
    crate::preserves_rail::canonical_hash(&record("dedup-key-v1", vec![
        record("scope", vec![string(&operation.scope_ref)]),
        record("producer", vec![string(&operation.producer)]),
        record("consumer", vec![string(&operation.consumer)]),
        record("sequence", vec![crate::preserves_rail::u64_value(operation.sequence)]),
        record("intent", vec![string(&operation.intent)]),
    ]))
}

fn validate_operation_input(input: &OperationIdInput) -> Result<()> {
    require_ref(&input.scope_ref, "delivery operation scope ref")?;
    validate_name(&input.producer, "delivery operation producer")?;
    validate_name(&input.consumer, "delivery operation consumer")?;
    validate_name(&input.intent, "delivery operation intent")?;
    require_ref(&input.payload_ref, "delivery operation payload ref")?;
    validate_refs(&input.policy_refs, "delivery operation policy ref")?;
    ensure_count_at_most(input.policy_refs.len(), MAX_REFS, "delivery operation policy refs")
}

fn validate_scope_profile(profile: &str) -> Result<()> {
    match profile {
        SCOPE_ACTOR_TURN
        | SCOPE_SERVICE_LIFECYCLE
        | SCOPE_PROTOCOL_SESSION
        | SCOPE_REMOTE_TOPIC
        | SCOPE_JOB_WORKER
        | SCOPE_CONTROL_COMMAND => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported delivery scope profile {profile}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "first" | "duplicate" | "conflict" | "stale" | "gap" | "retry" | "deny" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported idempotency decision {decision}"))),
    }
}

fn validate_side_effect(side_effect: &str) -> Result<()> {
    match side_effect {
        "commit" | "suppress" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported delivery side effect {side_effect}"))),
    }
}

fn validate_diagnostics(diagnostics: &[String]) -> Result<()> {
    ensure_count_at_most(diagnostics.len(), MAX_DIAGNOSTICS, "delivery diagnostics")?;
    for diagnostic in diagnostics {
        validate_name(diagnostic, "delivery diagnostic")?;
    }
    Ok(())
}

fn validate_name(value: &str, label: &str) -> Result<()> {
    if value.trim().is_empty() || value.contains('\0') || value.len() > MAX_SCOPE_NAME_LEN {
        return Err(MoltenError::invalid_harness(format!("invalid {label} {value:?}")));
    }
    Ok(())
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_REFS, label)?;
    for reference in refs {
        require_ref(reference, label)?;
    }
    Ok(())
}

fn require_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "unsupported {label} {reference}; expected canonical content ref: {error}"
        ))
    })
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
    }
}
