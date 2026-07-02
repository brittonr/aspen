
fn expected_ref_diagnostic(expected_ref: Option<&str>, actual_ref: &str) -> Result<Option<String>> {
    let Some(expected_ref) = expected_ref else {
        return Ok(None);
    };
    validate_content_ref(expected_ref)?;
    if expected_ref == actual_ref {
        Ok(None)
    } else {
        Ok(Some(format!("replay index ref mismatch expected={expected_ref} actual={actual_ref}")))
    }
}

fn summarize_index_inputs(receipts: &[ParsedReplayVerify], rollups: &[ParsedReplayRollup]) -> IndexSummary {
    let mut summary = empty_index_summary(receipts.len(), rollups.len());
    for parsed in receipts {
        summary.receipt_refs.insert(parsed.receipt_ref.clone());
        *summary.divergence_counts.entry(parsed.divergence.clone()).or_insert(0) += 1;
        if parsed.decision == "pass" {
            summary.pass_count += 1;
        } else {
            summary.deny_count += 1;
        }
        if let Some(reference) = &parsed.first_divergence_ref {
            summary.first_divergence_refs.insert(reference.clone());
        }
        summary.report_refs.extend(parsed.report_refs.iter().cloned());
        summary.final_state_refs.extend(parsed.final_state_refs.iter().cloned());
    }
    for parsed in rollups {
        summary.rollup_refs.insert(parsed.rollup_ref.clone());
        summary.receipt_refs.extend(parsed.receipt_refs.iter().cloned());
        summary.first_divergence_refs.extend(parsed.first_divergence_refs.iter().cloned());
        merge_divergence_counts(&mut summary.divergence_counts, &parsed.divergence_counts);
        summary.pass_count += parsed.pass_count;
        summary.deny_count += parsed.deny_count;
        summary.total_count += parsed.total_count;
    }
    summary
}

fn empty_index_summary(raw_count: usize, rollup_count: usize) -> IndexSummary {
    let raw_receipt_count = raw_count as u64;
    IndexSummary {
        receipt_refs: OrderedSet::new(),
        rollup_refs: OrderedSet::new(),
        first_divergence_refs: OrderedSet::new(),
        report_refs: OrderedSet::new(),
        final_state_refs: OrderedSet::new(),
        divergence_counts: OrderedMap::new(),
        pass_count: 0,
        deny_count: 0,
        raw_receipt_count,
        rollup_count: rollup_count as u64,
        total_count: raw_receipt_count,
    }
}

fn rollup_anomalies(rollups: &[ParsedReplayRollup]) -> Vec<String> {
    rollups
        .iter()
        .filter(|parsed| parsed.decision == "deny" && parsed.deny_count == 0)
        .map(|parsed| format!("replay rollup {} denied without denied receipt count", parsed.rollup_ref))
        .collect()
}

fn index_value(decision: &str, diagnostics: &[String], summary: &IndexSummary) -> IoValue {
    record("deterministic-replay-index-v1", vec![
        string(DETERMINISTIC_REPLAY_INDEX_SCHEMA),
        record("decision", vec![string(decision)]),
        record("total-count", vec![u64_value(summary.total_count)]),
        record("pass-count", vec![u64_value(summary.pass_count)]),
        record("deny-count", vec![u64_value(summary.deny_count)]),
        record("raw-receipt-count", vec![u64_value(summary.raw_receipt_count)]),
        record("rollup-count", vec![u64_value(summary.rollup_count)]),
        record("receipt-refs", vec![refs_value(&summary.receipt_refs)]),
        record("rollup-refs", vec![refs_value(&summary.rollup_refs)]),
        record("divergence-counts", vec![divergence_counts_value(&summary.divergence_counts)]),
        record("first-divergence-refs", vec![refs_value(&summary.first_divergence_refs)]),
        record("report-refs", vec![refs_value(&summary.report_refs)]),
        record("final-state-refs", vec![refs_value(&summary.final_state_refs)]),
        record("diagnostics", vec![sequence(diagnostics.iter().map(string).collect())]),
        sequence(index_checks(decision, diagnostics.is_empty())),
    ])
}

pub fn chaos_schedule_receipt(input: &ChaosScheduleInput) -> Result<ChaosScheduleReceipt> {
    validate_content_ref(&input.seed_ref)?;
    validate_content_ref(&input.event_ref)?;
    validate_chaos_fault_kind(&input.fault_kind)?;
    if input.intensity_percent > 100 {
        return Err(crate::error::MoltenError::invalid_harness("chaos schedule intensity exceeds 100"));
    }
    let preimage = record("deterministic-chaos-schedule-preimage-v1", vec![
        record("seed-ref", vec![string(&input.seed_ref)]),
        record("position", vec![u64_value(input.schedule_position)]),
        record("event-ref", vec![string(&input.event_ref)]),
        record("fault-kind", vec![string(&input.fault_kind)]),
    ]);
    let sample_ref = canonical_hash(&preimage)?;
    let sample = chaos_sample_percent(&sample_ref)?;
    let decision = if sample < input.intensity_percent {
        "inject"
    } else {
        "pass"
    };
    let value = record("deterministic-chaos-schedule-v1", vec![
        string(DETERMINISTIC_CHAOS_SCHEDULE_SCHEMA),
        record("seed-ref", vec![string(&input.seed_ref)]),
        record("position", vec![u64_value(input.schedule_position)]),
        record("event-ref", vec![string(&input.event_ref)]),
        record("fault-kind", vec![string(&input.fault_kind)]),
        record("intensity-percent", vec![u64_value(input.intensity_percent)]),
        record("sample-ref", vec![string(&sample_ref)]),
        record("decision", vec![string(decision)]),
        sequence(vec![
            record("check", vec![string("deterministic-schedule"), string("pass")]),
            record("check", vec![string("replay-identity-bound"), string("pass")]),
            record("check", vec![string("evidence-only-no-authority"), string("pass")]),
        ]),
    ]);
    let schedule_ref = canonical_hash(&value)?;
    Ok(ChaosScheduleReceipt {
        value,
        schedule_ref,
        decision: decision.to_string(),
    })
}

pub fn deterministic_integration_receipt(
    input: &DeterministicIntegrationInput,
) -> Result<DeterministicIntegrationReceipt> {
    validate_integration_kind(&input.integration_kind)?;
    validate_content_ref(&input.handler_profile_ref)?;
    validate_content_ref(&input.effect_log_ref)?;
    validate_content_ref(&input.snapshot_ref)?;
    validate_content_ref(&input.gate_ref)?;
    let decision = if input.admitted_live_effects { "deny" } else { "pass" };
    let value = record("deterministic-integration-gate-v1", vec![
        string(DETERMINISTIC_INTEGRATION_GATE_SCHEMA),
        record("integration-kind", vec![string(&input.integration_kind)]),
        record("decision", vec![string(decision)]),
        record("handler-profile-ref", vec![string(&input.handler_profile_ref)]),
        record("effect-log-ref", vec![string(&input.effect_log_ref)]),
        record("snapshot-ref", vec![string(&input.snapshot_ref)]),
        record("gate-ref", vec![string(&input.gate_ref)]),
        sequence(vec![
            record("check", vec![string("handler-profile-bound"), string("pass")]),
            record("check", vec![string("effect-log-bound"), string("pass")]),
            record("check", vec![string("snapshot-bound"), string("pass")]),
            record("check", vec![
                string("no-live-effect-during-replay"),
                string(if input.admitted_live_effects { "deny" } else { "pass" }),
            ]),
            record("check", vec![string("integration-gate-decision"), string(decision)]),
        ]),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(DeterministicIntegrationReceipt {
        value,
        receipt_ref,
        decision: decision.to_string(),
    })
}

pub fn trace_privacy_receipt(input: &TracePrivacyInput) -> Result<TracePrivacyReceipt> {
    validate_content_ref(&input.trace_ref)?;
    validate_content_ref(&input.snapshot_ref)?;
    validate_content_ref(&input.requester_ref)?;
    validate_content_ref(&input.policy_ref)?;
    let decision = match (input.has_export_authority, input.contains_sensitive_refs) {
        (false, true) => "deny",
        (true, true) => "redacted",
        _ => "pass",
    };
    let value = record("deterministic-trace-privacy-v1", vec![
        string(DETERMINISTIC_TRACE_PRIVACY_SCHEMA),
        record("decision", vec![string(decision)]),
        record("trace-ref", vec![string(&input.trace_ref)]),
        record("snapshot-ref", vec![string(&input.snapshot_ref)]),
        record("requester-ref", vec![string(&input.requester_ref)]),
        record("policy-ref", vec![string(&input.policy_ref)]),
        record("contains-sensitive-refs", vec![string(if input.contains_sensitive_refs { "yes" } else { "no" })]),
        sequence(trace_privacy_checks(decision, input.has_export_authority, input.contains_sensitive_refs)),
    ]);
    let receipt_ref = canonical_hash(&value)?;
    Ok(TracePrivacyReceipt {
        value,
        receipt_ref,
        decision: decision.to_string(),
    })
}

fn trace_privacy_checks(decision: &str, has_export_authority: bool, contains_sensitive_refs: bool) -> Vec<IoValue> {
    vec![
        record("check", vec![string("policy-admission-before-render"), string("pass")]),
        record("check", vec![
            string("sensitive-trace-gated"),
            string(if !contains_sensitive_refs || has_export_authority {
                "pass"
            } else {
                "deny"
            }),
        ]),
        record("check", vec![
            string("redacted-view-when-authorized-sensitive"),
            string(if decision == "redacted" || !contains_sensitive_refs {
                "pass"
            } else {
                "deny"
            }),
        ]),
        record("check", vec![string("trace-privacy-decision"), string(decision)]),
    ]
}

fn validate_integration_kind(kind: &str) -> Result<()> {
    match kind {
        "remote-sync" | "storage" | "job-dag" | "upgrade" => Ok(()),
        _ => Err(crate::error::MoltenError::invalid_harness(format!(
            "unsupported deterministic integration kind {kind}"
        ))),
    }
}

fn validate_chaos_fault_kind(kind: &str) -> Result<()> {
    match kind {
        "fault" | "delay" | "drop" | "reorder" | "partition" | "resource-limit" => Ok(()),
        _ => Err(crate::error::MoltenError::invalid_harness(format!("unsupported chaos fault kind {kind}"))),
    }
}

fn chaos_sample_percent(sample_ref: &str) -> Result<u64> {
    let hex = content_ref_hex(sample_ref)?;
    let sample = u64::from_str_radix(&hex[..16], 16)
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("invalid chaos sample ref: {error}")))?;
    Ok(sample % 100)
}

fn parse_replay_verify_receipt(value: &IoValue, receipt_ref: &str) -> Result<ParsedReplayVerify> {
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(13)) {
        require_schema_value(&fields[0], DETERMINISTIC_REPLAY_VERIFY_SCHEMA, "deterministic replay verify")?;
        let decision = required_string_value(&fields[1], "deterministic replay decision")?;
        let divergence = record_string_value(&fields[10], "divergence")?;
        let first_divergence_ref = record_string_value(&fields[11], "first-divergence-ref")?;
        validate_replay_decision(&decision)?;
        validate_divergence_ref(&first_divergence_ref)?;
        let expected_final_state_ref = record_string_value(&fields[8], "expected-final-state-ref")?;
        let actual_final_state_ref = record_string_value(&fields[9], "actual-final-state-ref")?;
        validate_content_ref(&expected_final_state_ref)?;
        validate_content_ref(&actual_final_state_ref)?;
        return Ok(ParsedReplayVerify {
            receipt_ref: receipt_ref.to_string(),
            decision,
            divergence,
            first_divergence_ref: (first_divergence_ref != "none").then_some(first_divergence_ref),
            report_refs: Vec::new(),
            final_state_refs: vec![expected_final_state_ref, actual_final_state_ref],
        });
    }
    if let Some(fields) = value.collect_simple_record("deterministic-replay-verify-v1", Some(7)) {
        require_schema_value(&fields[0], DETERMINISTIC_REPLAY_VERIFY_SCHEMA, "deterministic replay verify")?;
        let decision = required_string_value(&fields[1], "deterministic replay decision")?;
        let divergence = record_string_value(&fields[5], "divergence")?;
        validate_replay_decision(&decision)?;
        let expected_report_ref = record_string_value(&fields[2], "expected-report-ref")?;
        let actual_report_ref = record_string_value(&fields[3], "actual-report-ref")?;
        let final_state_ref = record_string_value(&fields[4], "final-state-ref")?;
        validate_content_ref(&expected_report_ref)?;
        validate_content_ref(&actual_report_ref)?;
        validate_content_ref(&final_state_ref)?;
        return Ok(ParsedReplayVerify {
            receipt_ref: receipt_ref.to_string(),
            decision,
            divergence,
            first_divergence_ref: None,
            report_refs: vec![expected_report_ref, actual_report_ref],
            final_state_refs: vec![final_state_ref],
        });
    }
    Err(crate::error::MoltenError::invalid_harness("expected <deterministic-replay-verify-v1 ...>"))
}
