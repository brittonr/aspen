
fn require_upgrade_replay_report_refs(
    receipt: &Record<Value<IoValue>>,
    old_report: &Report,
    new_report: &Report,
) -> Result<(String, String)> {
    if required_record_hash(&receipt[3], "old-report-ref", "old upgrade report ref")? != old_report.report_ref {
        return Err(MoltenError::invalid_harness("upgrade replay old report ref mismatch"));
    }
    if required_record_hash(&receipt[4], "new-report-ref", "new upgrade report ref")? != new_report.report_ref {
        return Err(MoltenError::invalid_harness("upgrade replay new report ref mismatch"));
    }
    let old_trace_ref = report_trace_ref(old_report)?;
    let new_trace_ref = report_trace_ref(new_report)?;
    if required_record_hash(&receipt[5], "old-trace-ref", "old upgrade trace ref")? != old_trace_ref {
        return Err(MoltenError::invalid_harness("upgrade replay old trace ref mismatch"));
    }
    if required_record_hash(&receipt[6], "new-trace-ref", "new upgrade trace ref")? != new_trace_ref {
        return Err(MoltenError::invalid_harness("upgrade replay new trace ref mismatch"));
    }
    if required_record_hash(&receipt[7], "old-state-ref", "old upgrade state ref")? != old_report.final_state_hash {
        return Err(MoltenError::invalid_harness("upgrade replay old state ref mismatch"));
    }
    if required_record_hash(&receipt[8], "new-state-ref", "new upgrade state ref")? != new_report.final_state_hash {
        return Err(MoltenError::invalid_harness("upgrade replay new state ref mismatch"));
    }
    Ok((old_trace_ref, new_trace_ref))
}

fn require_upgrade_replay_drift_evidence(
    outcome: &str,
    migration_receipt_ref: &str,
    compatibility_diagnostic_ref: &str,
    is_stable_replay: bool,
) -> Result<()> {
    if migration_receipt_ref != "none" {
        validate_content_ref(migration_receipt_ref)?;
    }
    if compatibility_diagnostic_ref != "none" {
        validate_content_ref(compatibility_diagnostic_ref)?;
    }
    if !is_stable_replay && migration_receipt_ref == "none" && compatibility_diagnostic_ref == "none" {
        return Err(MoltenError::invalid_harness(
            "upgrade replay trace drift requires migration receipt or compatibility diagnostic",
        ));
    }
    let expected_outcome = if is_stable_replay {
        "stable"
    } else if migration_receipt_ref != "none" {
        "migrated"
    } else {
        "diagnosed"
    };
    if outcome != expected_outcome {
        return Err(MoltenError::invalid_harness("upgrade replay outcome does not match drift evidence"));
    }
    Ok(())
}

fn require_upgrade_replay_checks(checks_value: &Value<IoValue>) -> Result<()> {
    let checks = parse_executor_preflight_checks(checks_value)?;
    for expected in [
        "old-report-bound",
        "new-report-bound",
        "canonical-trace-compare",
        "state-hash-compare",
        "drift-explained",
    ] {
        require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

fn report_trace_ref(report: &Report) -> Result<String> {
    canonical_hash(&sequence(report.observations.iter().map(|observation| observation.value.clone()).collect()))
}

pub fn run_receipt_value(report_value: &IoValue, export_refs: &[&str]) -> Result<IoValue> {
    let report = parse_report(report_value)?;
    for export_ref in export_refs {
        validate_content_ref(export_ref)?;
    }
    let step_results = report
        .observations
        .iter()
        .map(|observation| {
            record("step-result", vec![
                record("index", vec![u64_value(observation.index)]),
                record("step-ref", vec![string(&observation.step_ref)]),
                record("observation-ref", vec![string(&observation.observation_ref)]),
                record("status", vec![string("pass")]),
            ])
        })
        .collect::<Vec<_>>();
    let adapter_fixture_ref = report
        .executor_preflights
        .as_ref()
        .map(|preflights| canonical_hash(&preflights.value))
        .transpose()?;
    Ok(record("harness-run-receipt-v1", vec![
        string(crate::preserves_rail::HARNESS_RUN_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("suite-start", vec![string(&report.suite_ref)]),
        record("report-ref", vec![string(&report.report_ref)]),
        record("step-results", vec![sequence(step_results)]),
        record("adapter-fixture-decision-ref", vec![string(adapter_fixture_ref.as_deref().unwrap_or("none"))]),
        record("expected-failure-refs", vec![sequence(Vec::new())]),
        record("known-bug-refs", vec![sequence(Vec::new())]),
        record("final-status", vec![string(&report.status)]),
        record("report-export-refs", vec![refs_sequence_from_strs(export_refs)]),
        hostcall_checks_value(&[
            "suite-start-bound",
            "step-results-bound",
            "adapter-fixture-decision-bound",
            "expected-failure-recorded",
            "known-bug-recorded",
            "final-status-bound",
            "report-export-bound",
        ]),
    ]))
}

pub fn validate_harness_run_receipt(value: &IoValue, report_value: &IoValue, export_refs: &[&str]) -> Result<()> {
    let expected = run_receipt_value(report_value, export_refs)?;
    if canonical_hash(value)? != canonical_hash(&expected)? {
        return Err(MoltenError::invalid_harness("harness run receipt does not match report and export refs"));
    }
    let receipt = simple_record(value, "harness-run-receipt-v1", 11)?;
    let schema = required_string(&receipt[0], "harness run receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_RUN_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported harness run receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_RUN_RECEIPT_SCHEMA
        )));
    }
    if required_record_string(&receipt[1], "decision", "harness run receipt decision")? != "pass" {
        return Err(MoltenError::invalid_harness("harness run receipt decision must be pass"));
    }
    if required_record_string(&receipt[8], "final-status", "harness run final status")? != "pass" {
        return Err(MoltenError::invalid_harness("harness run final status must be pass"));
    }
    let checks = parse_executor_preflight_checks(&receipt[10])?;
    for expected in [
        "suite-start-bound",
        "step-results-bound",
        "adapter-fixture-decision-bound",
        "expected-failure-recorded",
        "known-bug-recorded",
        "final-status-bound",
        "report-export-bound",
    ] {
        require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

fn refs_sequence_from_strs(refs: &[&str]) -> IoValue {
    sequence(refs.iter().map(|value| string(*value)).collect())
}

pub fn deterministic_multipeer_receipt_value(
    report_value: &IoValue,
    seed: u64,
    profile: &str,
    peer_events: &[&str],
) -> Result<IoValue> {
    validate_multipeer_profile(profile)?;
    if peer_events.is_empty() {
        return Err(MoltenError::invalid_harness("deterministic multi-peer receipt requires at least one peer event"));
    }
    for event in peer_events {
        validate_multipeer_event(event)?;
    }
    let report = parse_report(report_value)?;
    let event_values = peer_events.iter().map(|event| string(*event)).collect::<Vec<_>>();
    let schedule_value = record("multipeer-schedule-v1", vec![
        record("seed", vec![u64_value(seed)]),
        record("profile", vec![string(profile)]),
        record("events", vec![sequence(event_values.clone())]),
    ]);
    let schedule_ref = canonical_hash(&schedule_value)?;
    let peer_count = report
        .actors
        .iter()
        .filter(|actor| matches!(actor.kind, ActorKind::RemoteProxy | ActorKind::Adapter))
        .count() as u64;
    Ok(record("deterministic-multipeer-receipt-v1", vec![
        string(crate::preserves_rail::HARNESS_DETERMINISTIC_MULTIPEER_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("replay", vec![string("stable")]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        record("report-ref", vec![string(&report.report_ref)]),
        record("seed", vec![u64_value(seed)]),
        record("profile", vec![string(profile)]),
        record("schedule-ref", vec![string(&schedule_ref)]),
        schedule_value,
        record("peer-count", vec![u64_value(peer_count)]),
        record("trace-ref", vec![string(report_trace_ref(&report)?)]),
        record("resource-budget-ref", vec![string(canonical_hash(&budget_value(
            &report.budget.limits,
            &report.budget.usage,
        ))?)]),
        hostcall_checks_value(&[
            "seeded-peer-delivery",
            "partition-replay-stable",
            "drop-reorder-reconnect-profile",
            "gossip-doc-blob-observations",
            "resource-limit-binding",
            "no-live-unrecorded-peer-io",
        ]),
    ]))
}

pub fn validate_deterministic_multipeer_receipt(
    value: &IoValue,
    report_value: &IoValue,
    seed: u64,
    profile: &str,
    peer_events: &[&str],
) -> Result<()> {
    let expected = deterministic_multipeer_receipt_value(report_value, seed, profile, peer_events)?;
    if canonical_hash(value)? != canonical_hash(&expected)? {
        return Err(MoltenError::invalid_harness("deterministic multi-peer receipt does not match replayed schedule"));
    }
    let receipt = simple_record(value, "deterministic-multipeer-receipt-v1", 13)?;
    let schema = required_string(&receipt[0], "deterministic multi-peer receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_DETERMINISTIC_MULTIPEER_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported deterministic multi-peer receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_DETERMINISTIC_MULTIPEER_RECEIPT_SCHEMA
        )));
    }
    if required_record_string(&receipt[1], "decision", "deterministic multi-peer decision")? != "pass" {
        return Err(MoltenError::invalid_harness("deterministic multi-peer decision must be pass"));
    }
    if required_record_string(&receipt[2], "replay", "deterministic multi-peer replay")? != "stable" {
        return Err(MoltenError::invalid_harness("deterministic multi-peer replay must be stable"));
    }
    let checks = parse_executor_preflight_checks(&receipt[12])?;
    for expected in [
        "seeded-peer-delivery",
        "partition-replay-stable",
        "drop-reorder-reconnect-profile",
        "gossip-doc-blob-observations",
        "resource-limit-binding",
        "no-live-unrecorded-peer-io",
    ] {
        require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

fn validate_multipeer_profile(profile: &str) -> Result<()> {
    if matches!(profile, "seeded" | "recorded") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported deterministic multi-peer profile {profile}; expected seeded or recorded"
        )))
    }
}

fn validate_multipeer_event(event: &str) -> Result<()> {
    if matches!(
        event,
        "deliver" | "partition" | "drop" | "reorder" | "reconnect" | "resource-limit" | "gossip" | "doc" | "blob"
    ) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported deterministic multi-peer event {event}; live or unrecorded peer delivery cannot satisfy replay"
        )))
    }
}

pub fn repro_bundle_value(report_value: &IoValue) -> Result<IoValue> {
    repro_bundle_value_with_command(report_value, &default_report_bundle_command())
}
