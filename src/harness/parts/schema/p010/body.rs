
pub fn boundary_coverage_value(report_value: &IoValue) -> Result<IoValue> {
    let report = parse_report(report_value)?;
    let suite = parse_suite(&report.suite_value)?;
    let mut coverage = Vec::new();
    push_boundary_coverage(
        &mut coverage,
        "envelope-routes",
        suite.steps.iter().any(|step| matches!(step, super::core::CoreStep::Send { .. })),
    );
    push_boundary_coverage(
        &mut coverage,
        "dataspace-semantics",
        suite.steps.iter().any(|step| {
            matches!(
                step,
                super::core::CoreStep::Observe { .. }
                    | super::core::CoreStep::Assert { .. }
                    | super::core::CoreStep::Retract { .. }
            )
        }),
    );
    push_boundary_coverage(
        &mut coverage,
        "policy-gates",
        report.policy_gate.is_some() && report.capability_gate.is_some(),
    );
    push_boundary_coverage(&mut coverage, "policy-denials", report_has_denied_admission(&report)?);
    push_boundary_coverage(&mut coverage, "effects", !report.effect_log.is_empty());
    push_boundary_coverage(
        &mut coverage,
        "receipts",
        report.policy_gate.is_some() && report.capability_gate.is_some() && report.budget_gate.is_some(),
    );
    push_boundary_coverage(&mut coverage, "traces", !report.observations.is_empty());
    push_boundary_coverage(&mut coverage, "storage-paths", false);
    push_boundary_coverage(&mut coverage, "resources", report.budget_gate.is_some());
    push_boundary_coverage(
        &mut coverage,
        "replay-branches",
        matches!(report.replay_status.as_str(), "deterministic" | "replay" | "record"),
    );
    push_boundary_coverage(
        &mut coverage,
        "adapters",
        report.actors.iter().any(|actor| !matches!(actor.kind, ActorKind::Native)),
    );
    push_boundary_coverage(
        &mut coverage,
        "confidentiality-paths",
        report_value_contains_label(report_value, "redaction-gate-v1"),
    );

    let unexercised = coverage
        .iter()
        .filter_map(|value| {
            let fields = value.collect_simple_record("boundary", Some(2))?;
            let name = required_string(&fields[0], "coverage boundary name").ok()?;
            let status = required_string(&fields[1], "coverage boundary status").ok()?;
            (status == "unexercised").then_some(string(name))
        })
        .collect::<Vec<_>>();

    Ok(record("harness-boundary-coverage-v1", vec![
        string("molten.harness.boundary-coverage.v1"),
        record("report-ref", vec![string(&report.report_ref)]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        sequence(coverage),
        record("unexercised", vec![sequence(unexercised)]),
    ]))
}

fn push_boundary_coverage(out: &mut impl crate::bounded::VecSink<IoValue>, name: &str, exercised: bool) {
    out.push_item(record("boundary", vec![
        string(name),
        string(if exercised { "exercised" } else { "unexercised" }),
    ]));
}

fn report_has_denied_admission(report: &Report) -> Result<bool> {
    for observation in &report.observations {
        for event in &observation.events {
            if event.collect_simple_record("admission-decision-v1", None).is_some()
                && !parse_admission_decision_event(event)?.decision.is_allowed()
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn report_value_contains_label(value: &IoValue, label: &str) -> bool {
    to_text(value).is_ok_and(|text| text.contains(label))
}

pub fn golden_trace_update_receipt_value(
    previous_report_ref: Option<&str>,
    updated_report_value: &IoValue,
    reason: &str,
    reviewer_ref: &str,
) -> Result<IoValue> {
    validate_golden_trace_update_reason(reason)?;
    if let Some(previous_report_ref) = previous_report_ref {
        validate_content_ref(previous_report_ref)?;
    }
    validate_content_ref(reviewer_ref)?;
    let report = parse_report(updated_report_value)?;
    Ok(record("golden-trace-update-receipt-v1", vec![
        string(crate::preserves_rail::HARNESS_GOLDEN_TRACE_UPDATE_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("reason", vec![string(reason)]),
        record("previous-report-ref", vec![string(previous_report_ref.unwrap_or("none"))]),
        record("updated-report-ref", vec![string(&report.report_ref)]),
        record("suite-ref", vec![string(&report.suite_ref)]),
        record("trace-ref", vec![string(canonical_hash(&sequence(
            report.observations.iter().map(|observation| observation.value.clone()).collect(),
        ))?)]),
        record("receipt-ref", vec![string(canonical_hash(&record("harness-golden-receipt-anchor", vec![
            string(&report.report_ref),
            string(&report.suite_ref),
            string(&report.final_state_hash),
        ]))?)]),
        record("state-ref", vec![string(&report.final_state_hash)]),
        record("reviewer-ref", vec![string(reviewer_ref)]),
        hostcall_checks_value(&[
            "reviewed-update-receipt",
            "canonical-trace-ref",
            "canonical-receipt-ref",
            "canonical-state-ref",
            "reason-classification",
        ]),
    ]))
}

pub fn validate_golden_trace_update_receipt(value: &IoValue, updated_report_value: &IoValue) -> Result<()> {
    let receipt = simple_record(value, "golden-trace-update-receipt-v1", 11)?;
    let schema = required_string(&receipt[0], "golden trace update receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_GOLDEN_TRACE_UPDATE_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported golden trace update receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_GOLDEN_TRACE_UPDATE_RECEIPT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "golden trace update decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported golden trace update decision {decision}")));
    }
    let reason = required_record_string(&receipt[2], "reason", "golden trace update reason")?;
    validate_golden_trace_update_reason(&reason)?;
    let previous_report_ref = required_record_string(&receipt[3], "previous-report-ref", "previous golden report ref")?;
    if previous_report_ref != "none" {
        validate_content_ref(&previous_report_ref)?;
    }
    let report = parse_report(updated_report_value)?;
    let updated_report_ref = required_record_hash(&receipt[4], "updated-report-ref", "updated golden report ref")?;
    if updated_report_ref != report.report_ref {
        return Err(MoltenError::invalid_harness("golden trace update report ref does not match updated report"));
    }
    let suite_ref = required_record_hash(&receipt[5], "suite-ref", "golden trace suite ref")?;
    if suite_ref != report.suite_ref {
        return Err(MoltenError::invalid_harness("golden trace update suite ref does not match updated report"));
    }
    let expected_trace_ref =
        canonical_hash(&sequence(report.observations.iter().map(|observation| observation.value.clone()).collect()))?;
    let trace_ref = required_record_hash(&receipt[6], "trace-ref", "golden trace ref")?;
    if trace_ref != expected_trace_ref {
        return Err(MoltenError::invalid_harness("golden trace update trace ref does not match report observations"));
    }
    let expected_receipt_ref = canonical_hash(&record("harness-golden-receipt-anchor", vec![
        string(&report.report_ref),
        string(&report.suite_ref),
        string(&report.final_state_hash),
    ]))?;
    let receipt_ref = required_record_hash(&receipt[7], "receipt-ref", "golden receipt ref")?;
    if receipt_ref != expected_receipt_ref {
        return Err(MoltenError::invalid_harness("golden trace update receipt ref does not match report"));
    }
    let state_ref = required_record_hash(&receipt[8], "state-ref", "golden state ref")?;
    if state_ref != report.final_state_hash {
        return Err(MoltenError::invalid_harness("golden trace update state ref does not match final state"));
    }
    let reviewer_ref = required_record_hash(&receipt[9], "reviewer-ref", "golden trace reviewer ref")?;
    validate_content_ref(&reviewer_ref)?;
    let checks = parse_executor_preflight_checks(&receipt[10])?;
    for expected in [
        "reviewed-update-receipt",
        "canonical-trace-ref",
        "canonical-receipt-ref",
        "canonical-state-ref",
        "reason-classification",
    ] {
        require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

fn validate_golden_trace_update_reason(reason: &str) -> Result<()> {
    if matches!(reason, "schema-driven" | "policy-driven" | "migration-driven" | "bug-fix") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported golden trace update reason {reason}; expected schema-driven, policy-driven, migration-driven, or bug-fix"
        )))
    }
}

pub fn upgrade_replay_receipt_value(
    old_report_value: &IoValue,
    new_report_value: &IoValue,
    migration_receipt_ref: Option<&str>,
    compatibility_diagnostic_ref: Option<&str>,
) -> Result<IoValue> {
    let old_report = parse_report(old_report_value)?;
    let new_report = parse_report(new_report_value)?;
    let old_trace_ref = report_trace_ref(&old_report)?;
    let new_trace_ref = report_trace_ref(&new_report)?;
    let is_stable_replay = old_trace_ref == new_trace_ref && old_report.final_state_hash == new_report.final_state_hash;
    if let Some(migration_receipt_ref) = migration_receipt_ref {
        validate_content_ref(migration_receipt_ref)?;
    }
    if let Some(compatibility_diagnostic_ref) = compatibility_diagnostic_ref {
        validate_content_ref(compatibility_diagnostic_ref)?;
    }
    if !is_stable_replay && migration_receipt_ref.is_none() && compatibility_diagnostic_ref.is_none() {
        return Err(MoltenError::invalid_harness(
            "upgrade replay trace drift requires migration receipt or compatibility diagnostic",
        ));
    }
    let outcome = if is_stable_replay {
        "stable"
    } else if migration_receipt_ref.is_some() {
        "migrated"
    } else {
        "diagnosed"
    };
    Ok(record("upgrade-replay-receipt-v1", vec![
        string(crate::preserves_rail::HARNESS_UPGRADE_REPLAY_RECEIPT_SCHEMA),
        record("decision", vec![string("pass")]),
        record("outcome", vec![string(outcome)]),
        record("old-report-ref", vec![string(&old_report.report_ref)]),
        record("new-report-ref", vec![string(&new_report.report_ref)]),
        record("old-trace-ref", vec![string(&old_trace_ref)]),
        record("new-trace-ref", vec![string(&new_trace_ref)]),
        record("old-state-ref", vec![string(&old_report.final_state_hash)]),
        record("new-state-ref", vec![string(&new_report.final_state_hash)]),
        record("migration-receipt-ref", vec![string(migration_receipt_ref.unwrap_or("none"))]),
        record("compatibility-diagnostic-ref", vec![string(compatibility_diagnostic_ref.unwrap_or("none"))]),
        hostcall_checks_value(&[
            "old-report-bound",
            "new-report-bound",
            "canonical-trace-compare",
            "state-hash-compare",
            "drift-explained",
        ]),
    ]))
}

pub fn validate_upgrade_replay_receipt(
    value: &IoValue,
    old_report_value: &IoValue,
    new_report_value: &IoValue,
) -> Result<()> {
    let receipt = simple_record(value, "upgrade-replay-receipt-v1", 12)?;
    let outcome = require_upgrade_replay_header(&receipt)?;
    let old_report = parse_report(old_report_value)?;
    let new_report = parse_report(new_report_value)?;
    let (old_trace_ref, new_trace_ref) = require_upgrade_replay_report_refs(&receipt, &old_report, &new_report)?;
    let migration_receipt_ref = required_record_string(&receipt[9], "migration-receipt-ref", "migration receipt ref")?;
    let compatibility_diagnostic_ref =
        required_record_string(&receipt[10], "compatibility-diagnostic-ref", "compatibility diagnostic ref")?;
    let is_stable_replay = old_trace_ref == new_trace_ref && old_report.final_state_hash == new_report.final_state_hash;
    require_upgrade_replay_drift_evidence(
        &outcome,
        &migration_receipt_ref,
        &compatibility_diagnostic_ref,
        is_stable_replay,
    )?;
    require_upgrade_replay_checks(&receipt[11])
}

fn require_upgrade_replay_header(receipt: &Record<Value<IoValue>>) -> Result<String> {
    let schema = required_string(&receipt[0], "upgrade replay receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_UPGRADE_REPLAY_RECEIPT_SCHEMA {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported upgrade replay receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_UPGRADE_REPLAY_RECEIPT_SCHEMA
        )));
    }
    let decision = required_record_string(&receipt[1], "decision", "upgrade replay decision")?;
    if decision != "pass" {
        return Err(MoltenError::invalid_harness(format!("unsupported upgrade replay decision {decision}")));
    }
    let outcome = required_record_string(&receipt[2], "outcome", "upgrade replay outcome")?;
    if !matches!(outcome.as_str(), "stable" | "migrated" | "diagnosed") {
        return Err(MoltenError::invalid_harness(format!("unsupported upgrade replay outcome {outcome}")));
    }
    Ok(outcome)
}
