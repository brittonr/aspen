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
    let mut identity_refs = OrderedSet::new();
    let mut first_divergence_refs = OrderedSet::new();
    let mut divergence_counts = OrderedMap::<String, u64>::new();
    let mut pass_count = 0_u64;
    let mut deny_count = 0_u64;
    for parsed in &parsed_receipts {
        receipt_refs.insert(parsed.receipt_ref.clone());
        identity_refs.extend(parsed.identity_refs.iter().cloned());
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
    let decision = if diagnostics.is_empty() && deny_count == 0 { "pass" } else { "deny" };
    let value = record("deterministic-replay-rollup-v1", vec![
        string(DETERMINISTIC_REPLAY_ROLLUP_SCHEMA),
        record("decision", vec![string(decision)]),
        record("total-count", vec![u64_value(total_count)]),
        record("pass-count", vec![u64_value(pass_count)]),
        record("deny-count", vec![u64_value(deny_count)]),
        record("receipt-refs", vec![refs_value(&receipt_refs)]),
        record("identity-refs", vec![refs_value(&identity_refs)]),
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
    let decision = if diagnostics.is_empty() && summary.deny_count == 0 { "pass" } else { "deny" };
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
    identity_refs: OrderedSet<String>,
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
