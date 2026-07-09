
fn octet_baseline_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(fields) = value.collect_simple_record("octet-warning-baseline-v1", Some(14)) {
        let expires_at = record_string(&fields[3], "expires-at")?;
        let finding_count = record_sequence_len(&fields[8], "finding-keys")?;
        let critical_count = record_sequence_len(&fields[9], "critical-finding-keys")?;
        let review_refs = record_string_sequence(&fields[12], "review-refs")?;
        let burn_down = value_to_iovalue(&fields[11]);
        let burn_down_fields = simple_record(&burn_down, "burn-down", 3)?;
        let total = record_u64(&burn_down_fields[0], "total")?;
        let target_next = record_u64(&burn_down_fields[1], "target-next")?;
        let deadline = record_string(&burn_down_fields[2], "deadline")?;
        let mut classifications = vec![
            "octet-baseline:warning-quarantine".to_string(),
            format!("octet-baseline-findings:{finding_count}"),
            format!("octet-baseline-critical:{critical_count}"),
            format!("octet-baseline-expires-at:{expires_at}"),
            format!("octet-baseline-burn-down-total:{total}"),
            format!("octet-baseline-burn-down-target-next:{target_next}"),
            format!("octet-baseline-burn-down-deadline:{deadline}"),
        ];
        ensure_count_at_most(classifications.len(), MAX_CATALOG_REFS, "catalog octet classifications")?;
        for review_ref in &review_refs {
            push_bounded(
                &mut classifications,
                format!("octet-review-ref:{review_ref}"),
                MAX_CATALOG_REFS,
                "catalog octet classifications",
            )?;
        }
        return Ok(Some(classifications));
    }
    if let Some(fields) = value.collect_simple_record("octet-baseline-receipt-v1", Some(12)) {
        let decision = record_string(&fields[1], "decision")?;
        let new_count = record_sequence_len(&fields[4], "new-findings")?;
        let removed_count = record_sequence_len(&fields[5], "removed-findings")?;
        let unchanged_count = record_sequence_len(&fields[6], "unchanged-findings")?;
        let critical_unreviewed = record_sequence_len(&fields[7], "critical-unreviewed")?;
        let review_refs = record_string_sequence(&fields[8], "review-refs")?;
        let mut classifications = vec![
            "octet-baseline-receipt:quarantine-check".to_string(),
            format!("octet-baseline-decision:{decision}"),
            format!("octet-baseline-new-findings:{new_count}"),
            format!("octet-baseline-removed-findings:{removed_count}"),
            format!("octet-baseline-unchanged-findings:{unchanged_count}"),
            format!("octet-baseline-critical-unreviewed:{critical_unreviewed}"),
        ];
        ensure_count_at_most(classifications.len(), MAX_CATALOG_REFS, "catalog octet classifications")?;
        for review_ref in &review_refs {
            push_bounded(
                &mut classifications,
                format!("octet-review-ref:{review_ref}"),
                MAX_CATALOG_REFS,
                "catalog octet classifications",
            )?;
        }
        return Ok(Some(classifications));
    }
    if let Some(fields) = value.collect_simple_record("octet-review-manifest-v1", Some(6)) {
        let profile = record_string(&fields[1], "profile")?;
        let expires_at = record_string(&fields[2], "expires-at")?;
        let finding_count = record_sequence_len(&fields[3], "finding-keys")?;
        return Ok(Some(vec![
            "octet-review-manifest:critical-finding-review".to_string(),
            format!("octet-review-profile:{profile}"),
            format!("octet-review-expires-at:{expires_at}"),
            format!("octet-review-finding-count:{finding_count}"),
        ]));
    }
    Ok(None)
}

fn octet_gate_labels(value: &IoValue) -> Result<Option<Vec<String>>> {
    if let Some(fields) = value.collect_simple_record("octet-gate-policy-v1", Some(8)) {
        let profile = record_string(&fields[1], "profile")?;
        let required_artifacts = record_sequence_len(&fields[3], "required-artifacts")?;
        let critical_lints = record_sequence_len(&fields[5], "critical-lints")?;
        return Ok(Some(vec![
            "octet-gate-policy:strict-source-gate".to_string(),
            format!("octet-gate-profile:{profile}"),
            format!("octet-gate-required-artifacts:{required_artifacts}"),
            format!("octet-gate-critical-lints:{critical_lints}"),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("octet-gate-receipt-v1", Some(15)) {
        let decision = record_string(&fields[1], "decision")?;
        let counts = value_to_iovalue(&fields[12]);
        let count_fields = simple_record(&counts, "counts", 6)?;
        let findings = record_u64(&count_fields[0], "findings")?;
        let warnings = record_u64(&count_fields[1], "warnings")?;
        let errors = record_u64(&count_fields[2], "errors")?;
        let critical = record_u64(&count_fields[4], "critical")?;
        return Ok(Some(vec![
            "octet-gate-receipt:strict-source-gate".to_string(),
            format!("octet-gate-decision:{decision}"),
            format!("octet-gate-findings:{findings}"),
            format!("octet-gate-warnings:{warnings}"),
            format!("octet-gate-errors:{errors}"),
            format!("octet-gate-critical:{critical}"),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("octet-source-gate-requirement-v1", Some(10)) {
        let consumer = record_string(&fields[1], "consumer")?;
        let source_scope = record_sequence_len(&fields[4], "source-scope")?;
        return Ok(Some(vec![
            "octet-source-gate-requirement:downstream-consumer".to_string(),
            format!("octet-source-gate-consumer:{consumer}"),
            format!("octet-source-gate-scope-paths:{source_scope}"),
        ]));
    }
    if let Some(fields) = value.collect_simple_record("octet-source-gate-validation-v1", Some(13)) {
        let decision = record_string(&fields[1], "decision")?;
        let counts = value_to_iovalue(&fields[10]);
        let count_fields = simple_record(&counts, "counts", 6)?;
        let findings = record_u64(&count_fields[0], "findings")?;
        let critical = record_u64(&count_fields[4], "critical")?;
        return Ok(Some(vec![
            "octet-source-gate-validation:strict-receipt-content".to_string(),
            format!("octet-source-gate-decision:{decision}"),
            format!("octet-source-gate-findings:{findings}"),
            format!("octet-source-gate-critical:{critical}"),
        ]));
    }
    Ok(None)
}

fn deterministic_replay_verify_gate_classifications(
    fields: &PreservesRecord<PreservesValue<IoValue>>,
) -> Result<Vec<String>> {
    require_schema(
        &fields[0],
        crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA,
        "deterministic replay verify",
    )?;
    let decision = required_string(&fields[1], "deterministic replay decision")?;
    let expected_report_ref = record_string(&fields[2], "expected-report-ref")?;
    let actual_report_ref = record_string(&fields[3], "actual-report-ref")?;
    let final_state_ref = record_string(&fields[4], "final-state-ref")?;
    let divergence = record_string(&fields[5], "divergence")?;
    Ok(vec![
        "deterministic-replay:verify".to_string(),
        format!("replay-decision:{decision}"),
        format!("receipt-decision:{decision}"),
        format!("replay-divergence:{divergence}"),
        format!("replay-expected-report:{expected_report_ref}"),
        format!("replay-actual-report:{actual_report_ref}"),
        format!("replay-final-state:{final_state_ref}"),
    ])
}

fn deterministic_replay_verify_fixture_classifications(
    fields: &PreservesRecord<PreservesValue<IoValue>>,
) -> Result<Vec<String>> {
    require_schema(
        &fields[0],
        crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA,
        "deterministic replay verify",
    )?;
    let decision = required_string(&fields[1], "deterministic replay decision")?;
    let expected_identity_ref = record_string(&fields[2], "expected-identity-ref")?;
    let actual_identity_ref = record_string(&fields[3], "actual-identity-ref")?;
    let expected_effect_log_ref = record_string(&fields[4], "expected-effect-log-ref")?;
    let actual_effect_log_ref = record_string(&fields[5], "actual-effect-log-ref")?;
    let expected_output_ref = record_string(&fields[6], "expected-output-ref")?;
    let actual_output_ref = record_string(&fields[7], "actual-output-ref")?;
    let expected_final_state_ref = record_string(&fields[8], "expected-final-state-ref")?;
    let actual_final_state_ref = record_string(&fields[9], "actual-final-state-ref")?;
    let divergence = record_string(&fields[10], "divergence")?;
    Ok(vec![
        "deterministic-replay:verify".to_string(),
        format!("replay-decision:{decision}"),
        format!("receipt-decision:{decision}"),
        format!("replay-divergence:{divergence}"),
        format!("replay-expected-identity:{expected_identity_ref}"),
        format!("replay-actual-identity:{actual_identity_ref}"),
        format!("replay-expected-effect-log:{expected_effect_log_ref}"),
        format!("replay-actual-effect-log:{actual_effect_log_ref}"),
        format!("replay-expected-output:{expected_output_ref}"),
        format!("replay-actual-output:{actual_output_ref}"),
        format!("replay-expected-final-state:{expected_final_state_ref}"),
        format!("replay-actual-final-state:{actual_final_state_ref}"),
    ])
}
