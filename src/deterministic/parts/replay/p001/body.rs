
#[derive(Clone)]
struct ReplayRunParts {
    identity: IoValue,
    identity_ref: String,
    scheduler_ref: String,
    input_ref: String,
    effect_request_ref: String,
    effect_response_ref: String,
    policy_decision_ref: String,
    action_ref: String,
    receipt_ref: String,
    output_ref: String,
    after_state_ref: String,
    turn_journal: IoValue,
    turn_journal_ref: String,
    effect_log: IoValue,
    effect_log_ref: String,
}

pub fn record_fixture_value() -> Result<ReplayFixtureRecord> {
    let parts = run_parts(ReplayFixtureVariant::Baseline)?;
    let value = record("deterministic-fixture-record-v1", vec![
        string(DETERMINISTIC_FIXTURE_RECORD_SCHEMA),
        record("identity-ref", vec![string(&parts.identity_ref)]),
        parts.identity,
        record("effect-log-ref", vec![string(&parts.effect_log_ref)]),
        parts.effect_log,
        sequence(vec![parts.turn_journal]),
        record("output-ref", vec![string(&parts.output_ref)]),
        record("final-state-ref", vec![string(&parts.after_state_ref)]),
        sequence(vec![
            string("recorded-responses-bound"),
            string("canonical-journal-order"),
            string("no-ambient-observations"),
        ]),
    ]);
    let record_ref = canonical_hash(&value)?;
    Ok(ReplayFixtureRecord {
        value,
        record_ref,
        identity_ref: parts.identity_ref,
        effect_log_ref: parts.effect_log_ref,
        final_state_ref: parts.after_state_ref,
        output_ref: parts.output_ref,
    })
}

pub fn replay_snapshot_manifest_bundle(
    chunk_root: &Path,
    variant: ReplayFixtureVariant,
) -> Result<ReplaySnapshotManifestBundle> {
    let expected = run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = run_parts(variant)?;
    let snapshot = record("deterministic-replay-snapshot-v1", vec![
        string("molten.deterministic-replay.snapshot.v1"),
        record("identity-ref", vec![string(&expected.identity_ref)]),
        record("final-state-ref", vec![string(&expected.after_state_ref)]),
        record("turn-journal-ref", vec![string(&expected.turn_journal_ref)]),
        record("effect-log-ref", vec![string(&expected.effect_log_ref)]),
        sequence(vec![string("manifest-backed"), string("partial-debug-fetch")]),
    ]);
    let effect_log_manifest_ref = store_replay_manifest(chunk_root, "replay-effect-log", &expected.effect_log)?;
    let turn_journal_manifest_ref = store_replay_manifest(chunk_root, "replay-turn-journal", &expected.turn_journal)?;
    let snapshot_manifest_ref = store_replay_manifest(chunk_root, "replay-snapshot", &snapshot)?;
    let divergence = first_divergence(&expected, &actual, variant);
    let (first_divergence_manifest_ref, debug_range_receipt_ref) = if divergence == ReplayDivergenceKind::None {
        (None, None)
    } else {
        let divergence_value = first_divergence_value(divergence, &expected, &actual)?;
        let manifest_ref = store_replay_manifest(chunk_root, "replay-first-divergence", &divergence_value)?;
        let range = range_read(chunk_root, &manifest_ref, 0, 32)?;
        (Some(manifest_ref), Some(canonical_hash(&range.receipt_value)?))
    };
    let value = record("deterministic-replay-snapshot-manifests-v1", vec![
        string("molten.deterministic-replay.snapshot-manifests.v1"),
        record("effect-log-manifest-ref", vec![string(&effect_log_manifest_ref)]),
        record("turn-journal-manifest-ref", vec![string(&turn_journal_manifest_ref)]),
        record("snapshot-manifest-ref", vec![string(&snapshot_manifest_ref)]),
        record("first-divergence-manifest-ref", vec![optional_ref_value(first_divergence_manifest_ref.as_deref())]),
        record("debug-range-receipt-ref", vec![optional_ref_value(debug_range_receipt_ref.as_deref())]),
        sequence(vec![
            string("manifest-backed-replay"),
            string("verified-before-load"),
            string("partial-divergence-debug-fetch"),
        ]),
    ]);
    let bundle_ref = canonical_hash(&value)?;
    Ok(ReplaySnapshotManifestBundle {
        value,
        bundle_ref,
        effect_log_manifest_ref,
        turn_journal_manifest_ref,
        snapshot_manifest_ref,
        first_divergence_manifest_ref,
        debug_range_receipt_ref,
    })
}

fn store_replay_manifest(chunk_root: &Path, object_kind: &str, value: &IoValue) -> Result<String> {
    let bytes = canonical_bytes(value)?;
    Ok(put_bytes(chunk_root, object_kind, &bytes, DEFAULT_FIXED_V1_CHUNK_SIZE)?.manifest_ref)
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

pub fn verify_fixture_value(variant: ReplayFixtureVariant) -> Result<ReplayVerifyReceipt> {
    let expected = run_parts(ReplayFixtureVariant::Baseline)?;
    let actual = run_parts(variant)?;
    let divergence = first_divergence(&expected, &actual, variant);
    let first_divergence = if divergence == ReplayDivergenceKind::None {
        None
    } else {
        Some(first_divergence_value(divergence, &expected, &actual)?)
    };
    let first_divergence_ref = match &first_divergence {
        Some(value) => canonical_hash(value)?,
        None => "none".to_string(),
    };
    let decision = if divergence == ReplayDivergenceKind::None {
        "pass"
    } else {
        "deny"
    };
    let value = record("deterministic-replay-verify-v1", vec![
        string(DETERMINISTIC_REPLAY_VERIFY_SCHEMA),
        string(decision),
        record("expected-identity-ref", vec![string(&expected.identity_ref)]),
        record("actual-identity-ref", vec![string(&actual.identity_ref)]),
        record("expected-effect-log-ref", vec![string(&expected.effect_log_ref)]),
        record("actual-effect-log-ref", vec![string(&actual.effect_log_ref)]),
        record("expected-output-ref", vec![string(&expected.output_ref)]),
        record("actual-output-ref", vec![string(&actual.output_ref)]),
        record("expected-final-state-ref", vec![string(&expected.after_state_ref)]),
        record("actual-final-state-ref", vec![string(&actual.after_state_ref)]),
        record("divergence", vec![string(divergence.as_str())]),
        record("first-divergence-ref", vec![string(&first_divergence_ref)]),
        sequence(verify_checks(decision, divergence)),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(ReplayVerifyReceipt {
        value,
        receipt_ref,
        decision,
        divergence,
        first_divergence,
    })
}

pub fn rollup_replay_receipts(inputs: &[ReplayRollupInput]) -> Result<ReplayRollupReceipt> {
    if inputs.len() > MAX_REPLAY_ROLLUP_INPUTS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "replay rollup input count exceeds {MAX_REPLAY_ROLLUP_INPUTS}"
        )));
    }
    let mut diagnostics = Vec::with_capacity(inputs.len());
    let mut parsed_receipts = Vec::with_capacity(inputs.len());
    for input in inputs {
        let actual_ref = canonical_hash(&input.value)?;
        if let Some(expected_ref) = input.expected_ref.as_deref() {
            validate_content_ref(expected_ref)?;
            if expected_ref != actual_ref {
                diagnostics.push(format!("replay receipt ref mismatch expected={expected_ref} actual={actual_ref}"));
                continue;
            }
        }
        match parse_replay_verify_receipt(&input.value, &actual_ref) {
            Ok(parsed) => parsed_receipts.push(parsed),
            Err(error) => diagnostics.push(format!("replay receipt {actual_ref} is invalid: {error}")),
        }
    }
    let mut receipt_refs = OrderedSet::new();
    let mut first_divergence_refs = OrderedSet::new();
    let mut divergence_counts = OrderedMap::<String, u64>::new();
    let mut pass_count = 0_u64;
    let mut deny_count = 0_u64;
    for parsed in &parsed_receipts {
        receipt_refs.insert(parsed.receipt_ref.clone());
        *divergence_counts.entry(parsed.divergence.clone()).or_insert(0) += 1;
        if parsed.decision == "pass" {
            pass_count += 1;
        } else {
            deny_count += 1;
        }
        if let Some(reference) = &parsed.first_divergence_ref {
            first_divergence_refs.insert(reference.clone());
        }
    }
    let total_count = parsed_receipts.len() as u64;
    let decision = if diagnostics.is_empty() && deny_count == 0 {
        "pass"
    } else {
        "deny"
    };
    let value = record("deterministic-replay-rollup-v1", vec![
        string(DETERMINISTIC_REPLAY_ROLLUP_SCHEMA),
        record("decision", vec![string(decision)]),
        record("total-count", vec![u64_value(total_count)]),
        record("pass-count", vec![u64_value(pass_count)]),
        record("deny-count", vec![u64_value(deny_count)]),
        record("receipt-refs", vec![refs_value(&receipt_refs)]),
        record("divergence-counts", vec![divergence_counts_value(&divergence_counts)]),
        record("first-divergence-refs", vec![refs_value(&first_divergence_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        sequence(rollup_checks(decision, diagnostics.is_empty())),
    ]);
    let rollup_ref = canonical_hash(&value)?;
    Ok(ReplayRollupReceipt {
        value,
        rollup_ref,
        decision: decision.to_string(),
        total_count,
        pass_count,
        deny_count,
    })
}

pub fn index_replay_evidence(inputs: &[ReplayIndexInput]) -> Result<ReplayIndexReceipt> {
    if inputs.len() > MAX_REPLAY_INDEX_INPUTS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "replay index input count exceeds {MAX_REPLAY_INDEX_INPUTS}"
        )));
    }
    let parsed = collect_index_inputs(inputs)?;
    let mut diagnostics = parsed.diagnostics;
    diagnostics.extend(rollup_anomalies(&parsed.rollups));
    let summary = summarize_index_inputs(&parsed.receipts, &parsed.rollups);
    let decision = if diagnostics.is_empty() && summary.deny_count == 0 {
        "pass"
    } else {
        "deny"
    };
    let value = index_value(decision, &diagnostics, &summary);
    let index_ref = canonical_hash(&value)?;
    Ok(ReplayIndexReceipt {
        value,
        index_ref,
        decision: decision.to_string(),
        total_count: summary.total_count,
        pass_count: summary.pass_count,
        deny_count: summary.deny_count,
        raw_receipt_count: summary.raw_receipt_count,
        rollup_count: summary.rollup_count,
    })
}

struct ParsedInputs {
    diagnostics: Vec<String>,
    receipts: Vec<ParsedReplayVerify>,
    rollups: Vec<ParsedReplayRollup>,
}

struct IndexSummary {
    receipt_refs: OrderedSet<String>,
    rollup_refs: OrderedSet<String>,
    first_divergence_refs: OrderedSet<String>,
    report_refs: OrderedSet<String>,
    final_state_refs: OrderedSet<String>,
    divergence_counts: OrderedMap<String, u64>,
    pass_count: u64,
    deny_count: u64,
    raw_receipt_count: u64,
    rollup_count: u64,
    total_count: u64,
}

fn collect_index_inputs(inputs: &[ReplayIndexInput]) -> Result<ParsedInputs> {
    let mut diagnostics = Vec::with_capacity(inputs.len());
    let mut receipts = Vec::with_capacity(inputs.len());
    let mut rollups = Vec::with_capacity(inputs.len());
    for input in inputs {
        let actual_ref = canonical_hash(&input.value)?;
        if let Some(diagnostic) = expected_ref_diagnostic(input.expected_ref.as_deref(), &actual_ref)? {
            diagnostics.push(diagnostic);
            continue;
        }
        if let Ok(parsed) = parse_replay_verify_receipt(&input.value, &actual_ref) {
            receipts.push(parsed);
        } else if let Ok(parsed) = parse_replay_rollup_receipt(&input.value, &actual_ref) {
            rollups.push(parsed);
        } else {
            diagnostics.push(format!("replay index input {actual_ref} is neither verify receipt nor rollup"));
        }
    }
    Ok(ParsedInputs {
        diagnostics,
        receipts,
        rollups,
    })
}
