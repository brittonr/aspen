
fn consensus_engine_conformance_receipt_value(
    input: &ConsensusEngineConformanceInput,
    decision: &str,
    diagnostics: &[String],
) -> Result<IoValue> {
    Ok(record("consensus-engine-conformance-receipt-v1", vec![
        string(CONSENSUS_ENGINE_CONFORMANCE_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("profile", vec![string(&input.algorithm_profile)]),
        record("version", vec![string(&input.profile_version)]),
        record("fixture", vec![string(&input.fixture_id)]),
        record("cases", vec![strings_sequence(&input.passed_cases)]),
        record("expected-state", vec![string(&input.expected_state_ref)]),
        record("actual-state", vec![string(&input.actual_state_ref)]),
        record("normalized", vec![strings_sequence(&input.normalized_receipt_refs)]),
        record("diagnostics", vec![strings_sequence(diagnostics)]),
        checks_value(&[
            ("deterministic-engine-conformance", decision),
            ("canonical-replay-state", if input.expected_state_ref == input.actual_state_ref { ENGINE_DECISION_PASS } else { ENGINE_DECISION_DENY }),
            ("normalized-receipt-shape", if input.normalized_receipt_refs.is_empty() { ENGINE_DECISION_DENY } else { ENGINE_DECISION_PASS }),
        ]),
    ]))
}

pub fn parse_consensus_engine_conformance_receipt(value: &IoValue) -> Result<ConsensusEngineConformanceReceipt> {
    let fields = value
        .collect_simple_record("consensus-engine-conformance-receipt-v1", Some(CONSENSUS_ENGINE_CONFORMANCE_FIELD_COUNT))
        .ok_or_else(|| MoltenError::invalid_harness("expected <consensus-engine-conformance-receipt-v1 ...>"))?;
    require_schema(&fields[0], CONSENSUS_ENGINE_CONFORMANCE_RECEIPT_SCHEMA, "consensus conformance schema")?;
    require_check(&parse_checks(&fields[10])?, "deterministic-engine-conformance", "consensus conformance receipt")?;
    Ok(ConsensusEngineConformanceReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        fixture_id: record_string(&fields[4], "fixture")?,
        diagnostics: parse_string_sequence(&fields[9], "diagnostics")?,
        value: value.clone(),
    })
}

fn validate_string_items(values: &[String], label: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_RAFT_REFS, label)?;
    if values.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} list must not be empty")));
    }
    for value in values {
        validate_non_empty(value, label)?;
    }
    Ok(())
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        ENGINE_DECISION_PASS | ENGINE_DECISION_DENY | ENGINE_DECISION_DIAGNOSTIC => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported consensus engine decision {value}"))),
    }
}
