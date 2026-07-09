const CATALOG_REPLAY_ROLLUP_LEGACY_FIELD_COUNT: usize = 10;
const CATALOG_REPLAY_ROLLUP_IDENTITY_FIELD_COUNT: usize = 11;
const CATALOG_REPLAY_ROLLUP_IDENTITY_REFS_INDEX: usize = 6;
const CATALOG_REPLAY_ROLLUP_LEGACY_DIVERGENCE_INDEX: usize = 6;
const CATALOG_REPLAY_ROLLUP_DIVERGENCE_INDEX: usize = 7;
const CATALOG_REPLAY_INDEX_LEGACY_FIELD_COUNT: usize = 15;
const CATALOG_REPLAY_INDEX_IDENTITY_FIELD_COUNT: usize = 16;
const CATALOG_REPLAY_INDEX_RECEIPT_REFS_INDEX: usize = 7;
const CATALOG_REPLAY_INDEX_ROLLUP_REFS_INDEX: usize = 8;
const CATALOG_REPLAY_INDEX_IDENTITY_REFS_INDEX: usize = 9;
const CATALOG_REPLAY_INDEX_LEGACY_DIVERGENCE_INDEX: usize = 9;
const CATALOG_REPLAY_INDEX_DIVERGENCE_INDEX: usize = 10;
const CATALOG_REPLAY_INDEX_LEGACY_REPORT_REFS_INDEX: usize = 11;
const CATALOG_REPLAY_INDEX_REPORT_REFS_INDEX: usize = 12;
const CATALOG_REPLAY_INDEX_LEGACY_FINAL_STATE_REFS_INDEX: usize = 12;
const CATALOG_REPLAY_INDEX_FINAL_STATE_REFS_INDEX: usize = 13;

fn replay_direct_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(7)) {
        return deterministic_replay_verify_gate_classifications(&fields).map(Some);
    }
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(13)) {
        return deterministic_replay_verify_fixture_classifications(&fields).map(Some);
    }
    if let Some(fields) = value.collect_simple_record("deterministic-first-divergence-v1", Some(9)) {
        require_schema(
            &fields[0],
            crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA,
            "deterministic first divergence",
        )?;
        let kind = record_string(&fields[1], "kind")?;
        let actor_id = record_string(&fields[3], "actor-id")?;
        let handler_profile_ref = record_string(&fields[5], "handler-profile-ref")?;
        let expected_ref = record_string(&fields[6], "expected-ref")?;
        let actual_ref = record_string(&fields[7], "actual-ref")?;
        return Ok(Some(vec![
            "deterministic-replay:first-divergence".to_string(),
            format!("replay-divergence:{kind}"),
            format!("replay-actor:{actor_id}"),
            format!("replay-handler-profile:{handler_profile_ref}"),
            format!("replay-expected-ref:{expected_ref}"),
            format!("replay-actual-ref:{actual_ref}"),
        ]));
    }
    if let Some(fields) = value.collect_simple_record(
        "deterministic-replay-rollup-v1",
        Some(CATALOG_REPLAY_ROLLUP_IDENTITY_FIELD_COUNT),
    ) {
        return deterministic_replay_rollup_classifications(&fields, true).map(Some);
    }
    if let Some(fields) = value.collect_simple_record(
        "deterministic-replay-rollup-v1",
        Some(CATALOG_REPLAY_ROLLUP_LEGACY_FIELD_COUNT),
    ) {
        return deterministic_replay_rollup_classifications(&fields, false).map(Some);
    }
    if let Some(fields) = value.collect_simple_record(
        "deterministic-replay-index-v1",
        Some(CATALOG_REPLAY_INDEX_IDENTITY_FIELD_COUNT),
    ) {
        return deterministic_replay_index_classifications(&fields, true).map(Some);
    }
    if let Some(fields) = value.collect_simple_record(
        "deterministic-replay-index-v1",
        Some(CATALOG_REPLAY_INDEX_LEGACY_FIELD_COUNT),
    ) {
        return deterministic_replay_index_classifications(&fields, false).map(Some);
    }
    Ok(None)
}

fn deterministic_replay_rollup_classifications(
    fields: &PreservesRecord<PreservesValue<IoValue>>,
    has_identity_refs: bool,
) -> Result<Vec<String>> {
    require_schema(
        &fields[0],
        crate::preserves_rail::DETERMINISTIC_REPLAY_ROLLUP_SCHEMA,
        "deterministic replay rollup",
    )?;
    let decision = record_string(&fields[1], "decision")?;
    let total_count = record_u64(&fields[2], "total-count")?;
    let pass_count = record_u64(&fields[3], "pass-count")?;
    let deny_count = record_u64(&fields[4], "deny-count")?;
    let mut classifications = vec![
        "deterministic-replay:rollup".to_string(),
        format!("replay-rollup-decision:{decision}"),
        format!("replay-rollup-total:{total_count}"),
        format!("replay-rollup-pass:{pass_count}"),
        format!("replay-rollup-deny:{deny_count}"),
    ];
    if has_identity_refs {
        for reference in record_ref_sequence(&fields[CATALOG_REPLAY_ROLLUP_IDENTITY_REFS_INDEX], "identity-refs")? {
            classifications.push(format!("replay-identity-ref:{reference}"));
            classifications.push(format!("replay-rollup-identity:{reference}"));
        }
    }
    let divergence_index = if has_identity_refs {
        CATALOG_REPLAY_ROLLUP_DIVERGENCE_INDEX
    } else {
        CATALOG_REPLAY_ROLLUP_LEGACY_DIVERGENCE_INDEX
    };
    let counts = value_to_iovalue(&fields[divergence_index]);
    let count_fields = simple_record(&counts, "divergence-counts", 1)?;
    for item in required_sequence(&count_fields[0], "divergence-counts")?.iter() {
        let item = value_to_iovalue(item);
        let count = simple_record(&item, "divergence-count", 2)?;
        let kind = required_string(&count[0], "divergence kind")?;
        classifications.push(format!("replay-rollup-divergence:{kind}"));
    }
    Ok(classifications)
}

fn deterministic_replay_index_classifications(
    fields: &PreservesRecord<PreservesValue<IoValue>>,
    has_identity_refs: bool,
) -> Result<Vec<String>> {
    require_schema(&fields[0], crate::preserves_rail::DETERMINISTIC_REPLAY_INDEX_SCHEMA, "deterministic replay index")?;
    let decision = record_string(&fields[1], "decision")?;
    let total_count = record_u64(&fields[2], "total-count")?;
    let pass_count = record_u64(&fields[3], "pass-count")?;
    let deny_count = record_u64(&fields[4], "deny-count")?;
    let raw_receipt_count = record_u64(&fields[5], "raw-receipt-count")?;
    let rollup_count = record_u64(&fields[6], "rollup-count")?;
    let mut classifications = vec![
        "deterministic-replay:index".to_string(),
        format!("replay-decision:{decision}"),
        format!("replay-index-decision:{decision}"),
        format!("replay-index-total:{total_count}"),
        format!("replay-index-pass:{pass_count}"),
        format!("replay-index-deny:{deny_count}"),
        format!("replay-index-raw-receipts:{raw_receipt_count}"),
        format!("replay-index-rollups:{rollup_count}"),
    ];
    let divergence_index = if has_identity_refs {
        CATALOG_REPLAY_INDEX_DIVERGENCE_INDEX
    } else {
        CATALOG_REPLAY_INDEX_LEGACY_DIVERGENCE_INDEX
    };
    let counts = value_to_iovalue(&fields[divergence_index]);
    let count_fields = simple_record(&counts, "divergence-counts", 1)?;
    for item in required_sequence(&count_fields[0], "divergence-counts")?.iter() {
        let item = value_to_iovalue(item);
        let count = simple_record(&item, "divergence-count", 2)?;
        let kind = required_string(&count[0], "divergence kind")?;
        classifications.push(format!("replay-index-divergence:{kind}"));
    }
    for reference in record_ref_sequence(&fields[CATALOG_REPLAY_INDEX_RECEIPT_REFS_INDEX], "receipt-refs")? {
        classifications.push(format!("replay-index-receipt:{reference}"));
    }
    for reference in record_ref_sequence(&fields[CATALOG_REPLAY_INDEX_ROLLUP_REFS_INDEX], "rollup-refs")? {
        classifications.push(format!("replay-index-rollup:{reference}"));
    }
    if has_identity_refs {
        for reference in record_ref_sequence(&fields[CATALOG_REPLAY_INDEX_IDENTITY_REFS_INDEX], "identity-refs")? {
            classifications.push(format!("replay-identity-ref:{reference}"));
            classifications.push(format!("replay-index-identity:{reference}"));
        }
    }
    let report_refs_index = if has_identity_refs {
        CATALOG_REPLAY_INDEX_REPORT_REFS_INDEX
    } else {
        CATALOG_REPLAY_INDEX_LEGACY_REPORT_REFS_INDEX
    };
    let final_state_refs_index = if has_identity_refs {
        CATALOG_REPLAY_INDEX_FINAL_STATE_REFS_INDEX
    } else {
        CATALOG_REPLAY_INDEX_LEGACY_FINAL_STATE_REFS_INDEX
    };
    for reference in record_ref_sequence(&fields[report_refs_index], "report-refs")? {
        classifications.push(format!("replay-index-report:{reference}"));
    }
    for reference in record_ref_sequence(&fields[final_state_refs_index], "final-state-refs")? {
        classifications.push(format!("replay-index-final-state:{reference}"));
    }
    Ok(classifications)
}
