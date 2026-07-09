fn parse_replay_rollup_receipt(value: &IoValue, rollup_ref: &str) -> Result<ParsedReplayRollup> {
    if let Some(fields) = value.collect_simple_record(
        "deterministic-replay-rollup-v1",
        Some(REPLAY_ROLLUP_IDENTITY_FIELD_COUNT),
    ) {
        return parse_replay_rollup_fields(&fields, rollup_ref, true);
    }
    let fields = value
        .collect_simple_record("deterministic-replay-rollup-v1", Some(REPLAY_ROLLUP_LEGACY_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected <deterministic-replay-rollup-v1 ...>"))?;
    parse_replay_rollup_fields(&fields, rollup_ref, false)
}

fn parse_replay_rollup_fields(
    fields: &preserves::Record<PreservesValue<IoValue>>,
    rollup_ref: &str,
    has_identity_refs: bool,
) -> Result<ParsedReplayRollup> {
    require_schema_value(
        &fields[REPLAY_ROLLUP_SCHEMA_INDEX],
        DETERMINISTIC_REPLAY_ROLLUP_SCHEMA,
        "deterministic replay rollup",
    )?;
    let decision = record_string_value(&fields[REPLAY_ROLLUP_DECISION_INDEX], "decision")?;
    validate_replay_decision(&decision)?;
    let total_count = record_u64_value(&fields[REPLAY_ROLLUP_TOTAL_COUNT_INDEX], "total-count")?;
    let pass_count = record_u64_value(&fields[REPLAY_ROLLUP_PASS_COUNT_INDEX], "pass-count")?;
    let deny_count = record_u64_value(&fields[REPLAY_ROLLUP_DENY_COUNT_INDEX], "deny-count")?;
    let receipt_refs = record_ref_list_value(&fields[REPLAY_ROLLUP_RECEIPT_REFS_INDEX], "receipt-refs")?;
    let identity_refs = if has_identity_refs {
        record_ref_list_value(&fields[REPLAY_ROLLUP_IDENTITY_REFS_INDEX], "identity-refs")?
    } else {
        Vec::new()
    };
    let divergence_index = if has_identity_refs {
        REPLAY_ROLLUP_DIVERGENCE_INDEX
    } else {
        REPLAY_ROLLUP_LEGACY_DIVERGENCE_INDEX
    };
    let first_divergence_index = if has_identity_refs {
        REPLAY_ROLLUP_FIRST_DIVERGENCE_INDEX
    } else {
        REPLAY_ROLLUP_LEGACY_FIRST_DIVERGENCE_INDEX
    };
    let divergence_counts = record_divergence_counts_value(&fields[divergence_index])?;
    let first_divergence_refs = record_ref_list_value(&fields[first_divergence_index], "first-divergence-refs")?;
    Ok(ParsedReplayRollup {
        rollup_ref: rollup_ref.to_string(),
        decision,
        total_count,
        pass_count,
        deny_count,
        receipt_refs,
        identity_refs,
        divergence_counts,
        first_divergence_refs,
    })
}
