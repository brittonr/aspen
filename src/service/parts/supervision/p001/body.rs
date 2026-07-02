
pub fn run_service_supervision_suite(suite: &ServiceSupervisionSuite) -> Result<ServiceSupervisionRun> {
    let mut monitors = suite
        .monitors
        .iter()
        .filter(|monitor| monitor.service_id == suite.manifest.service_id)
        .cloned()
        .collect::<Vec<_>>();
    ensure_count_at_most(monitors.len(), "service monitors")?;
    monitors.sort_by(|left, right| {
        left.service_id
            .cmp(&right.service_id)
            .then_with(|| left.monitor_ref.cmp(&right.monitor_ref))
            .then_with(|| left.observer_ref.cmp(&right.observer_ref))
    });

    let failure_marker = failure_marker_value(suite)?;
    let failure_ref = crate::preserves_rail::canonical_hash(&failure_marker)?;
    let monitor_refs = monitors.iter().map(|monitor| monitor.monitor_ref.clone()).collect::<Vec<_>>();
    let failure_status = failure_status_value(suite, &failure_ref, &monitor_refs)?;
    let failure_status_ref = crate::preserves_rail::canonical_hash(&failure_status)?;
    let monitor_notifications = monitor_notification_values(&monitors, suite, &failure_ref, &failure_status_ref)?;
    let notification_refs = refs_for_values(&monitor_notifications)?;
    let supervision_refs = supervision_refs(suite, &monitor_refs, &notification_refs)?;
    let failure_lifecycle = failure_lifecycle_receipt(suite, &failure_status_ref, &supervision_refs)?;
    let failure_lifecycle_ref = crate::preserves_rail::canonical_hash(&failure_lifecycle)?;
    let restart_evaluation = evaluate_restart(suite)?;
    let restart_decision = restart_decision_value(suite, &restart_evaluation, &failure_lifecycle_ref)?;
    let restart_decision_ref = crate::preserves_rail::canonical_hash(&restart_decision)?;
    let scheduled_demands = scheduled_demands(suite, &restart_evaluation)?;
    let cleanup_evaluation = evaluate_cleanup(suite, &restart_evaluation, &restart_decision_ref)?;
    let final_statuses =
        final_status_values(suite, &failure_ref, &restart_evaluation, &restart_decision_ref, &monitor_refs)?;
    let failure_markers = vec![failure_marker];
    let statuses = status_values(&failure_status, &final_statuses)?;
    let lifecycle_receipts = vec![failure_lifecycle];
    let restart_decisions = vec![restart_decision];
    let cleanup_receipts = optional_value_vec(cleanup_evaluation.cleanup_receipt);
    let retractions = cleanup_evaluation.retractions;
    let retention_inputs = optional_value_vec(cleanup_evaluation.retention_input);
    let report_value = service_supervision_report_value(ReportValueInput {
        suite_value: &suite.value,
        failure_markers: &failure_markers,
        statuses: &statuses,
        lifecycle_receipts: &lifecycle_receipts,
        monitor_notifications: &monitor_notifications,
        restart_decisions: &restart_decisions,
        scheduled_demands: &scheduled_demands,
        cleanup_receipts: &cleanup_receipts,
        retractions: &retractions,
        retention_inputs: &retention_inputs,
    })?;
    Ok(ServiceSupervisionRun {
        suite_ref: suite.suite_ref.clone(),
        suite_value: suite.value.clone(),
        report_ref: crate::preserves_rail::canonical_hash(&report_value)?,
        failure_markers,
        statuses,
        lifecycle_receipts,
        monitor_notifications,
        restart_decisions,
        scheduled_demands,
        cleanup_receipts,
        retractions,
        retention_inputs,
        value: report_value,
    })
}

pub fn replay_service_supervision_report(value: &IoValue) -> Result<ServiceSupervisionReplay> {
    let report = parse_service_supervision_report(value)?;
    let rerun = run_service_supervision_suite_value(&report.suite_value)?;
    let expected_report_ref = crate::preserves_rail::canonical_hash(value)?;
    let decision = if expected_report_ref == rerun.report_ref {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    if decision == "deny" {
        return Err(MoltenError::invalid_harness(format!(
            "service supervision replay divergence: expected {expected_report_ref}, got {}",
            rerun.report_ref
        )));
    }
    Ok(ServiceSupervisionReplay {
        expected_report_ref,
        actual_report_ref: rerun.report_ref,
        decision,
    })
}

pub fn gate_service_supervision_report(value: &IoValue) -> Result<ServiceSupervisionGate> {
    let report = parse_service_supervision_report(value)?;
    let mut diagnostics = service_supervision_gate_diagnostics(&report)?;
    let restart_decision = report
        .restart_decisions
        .first()
        .map(crate::service_records::parse_service_restart_decision)
        .transpose()?
        .map(|decision| decision.decision);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = service_supervision_gate_receipt_value(&GateReceiptValueInput {
        decision,
        report_ref: &report.report_ref,
        suite_ref: &report.suite_ref,
        restart_decision: restart_decision.as_deref(),
        status_count: report.statuses.len(),
        monitor_count: report.monitor_notifications.len(),
        cleanup_count: report.cleanup_receipts.len(),
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    diagnostics.shrink_to_fit();
    Ok(ServiceSupervisionGate {
        receipt_ref,
        report_ref: report.report_ref,
        suite_ref: report.suite_ref,
        decision: decision.to_string(),
        restart_decision,
        status_count: report.statuses.len(),
        monitor_count: report.monitor_notifications.len(),
        cleanup_count: report.cleanup_receipts.len(),
        diagnostics,
        value: receipt_value,
    })
}

pub fn parse_service_supervision_gate_receipt(value: &IoValue) -> Result<ServiceSupervisionGateReceipt> {
    let fields = value
        .collect_simple_record("service-supervision-gate-receipt-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-supervision-gate-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::SERVICE_SUPERVISION_GATE_RECEIPT_SCHEMA,
        "service supervision gate receipt schema",
    )?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "service-supervision-gate-is-not-authority", "service supervision gate receipt")?;
    let decision = record_string(&fields[1], "decision")?;
    validate_decision(&decision, "service supervision gate decision")?;
    let restart_decision = record_optional_string(&fields[4], "restart-decision")?;
    if let Some(decision) = &restart_decision {
        validate_restart_decision(decision, "service supervision restart decision")?;
    }
    Ok(ServiceSupervisionGateReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision,
        report_ref: record_ref(&fields[2], "report")?,
        suite_ref: record_ref(&fields[3], "suite")?,
        restart_decision,
        status_count: record_u64(&fields[5], "status-count")?,
        monitor_count: record_u64(&fields[6], "monitor-count")?,
        cleanup_count: record_u64(&fields[7], "cleanup-count")?,
        diagnostics: parse_string_sequence(&fields[8], "diagnostics")?,
    })
}

fn service_supervision_gate_diagnostics(report: &ServiceSupervisionRun) -> Result<Vec<String>> {
    let mut diagnostics = Vec::with_capacity(8);
    if let Err(error) = replay_service_supervision_report(&report.value) {
        diagnostics.push(format!("service supervision gate replay failed: {error}"));
    }
    if report.failure_markers.is_empty() {
        diagnostics.push("service supervision gate requires a failure marker".to_string());
    }
    if report.statuses.is_empty() {
        diagnostics.push("service supervision gate requires status evidence".to_string());
    }
    if report.lifecycle_receipts.is_empty() {
        diagnostics.push("service supervision gate requires lifecycle receipt evidence".to_string());
    }
    if report.restart_decisions.is_empty() {
        diagnostics.push("service supervision gate requires restart decision evidence".to_string());
    }
    for value in &report.failure_markers {
        parse_failure_marker_ref(value)?;
    }
    for value in &report.statuses {
        crate::service_records::parse_service_status(value)?;
    }
    for value in &report.lifecycle_receipts {
        crate::service_records::parse_service_lifecycle_receipt(value)?;
    }
    for value in &report.restart_decisions {
        crate::service_records::parse_service_restart_decision(value)?;
    }
    for value in &report.monitor_notifications {
        parse_monitor_notification_ref(value)?;
    }
    for value in &report.cleanup_receipts {
        crate::service_records::parse_service_cleanup_receipt(value)?;
    }
    for value in &report.retractions {
        parse_retraction_ref(value)?;
    }
    Ok(diagnostics)
}

pub fn parse_service_supervision_report(value: &IoValue) -> Result<ServiceSupervisionRun> {
    let fields = value
        .collect_simple_record("service-supervision-report-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-supervision-report-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::SERVICE_SUPERVISION_REPORT_SCHEMA,
        "service supervision report schema",
    )?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-service-supervision-report", "service supervision report")?;
    let suite_value = record_iovalue(&fields[1], "suite")?;
    let suite_ref = crate::preserves_rail::canonical_hash(&suite_value)?;
    Ok(ServiceSupervisionRun {
        suite_ref,
        suite_value,
        report_ref: crate::preserves_rail::canonical_hash(value)?,
        failure_markers: parse_iovalue_sequence(&fields[2], "failures")?,
        statuses: parse_iovalue_sequence(&fields[3], "statuses")?,
        lifecycle_receipts: parse_iovalue_sequence(&fields[4], "lifecycle")?,
        monitor_notifications: parse_iovalue_sequence(&fields[5], "monitor-notifications")?,
        restart_decisions: parse_iovalue_sequence(&fields[6], "restart-decisions")?,
        scheduled_demands: parse_iovalue_sequence(&fields[7], "scheduled-demands")?,
        cleanup_receipts: parse_iovalue_sequence(&fields[8], "cleanup")?,
        retractions: parse_iovalue_sequence(&fields[9], "retractions")?,
        retention_inputs: parse_iovalue_sequence(&fields[10], "retention")?,
        value: value.clone(),
    })
}

pub fn service_supervision_summary(value: &IoValue) -> Result<String> {
    let report = parse_service_supervision_report(value)?;
    let restart_decision = report
        .restart_decisions
        .first()
        .map(crate::service_records::parse_service_restart_decision)
        .transpose()?
        .map_or_else(|| "none".to_string(), |decision| decision.decision);
    Ok(format!(
        "service supervision report ref={} suite={} monitors={} restart={} cleanup={} retractions={}",
        report.report_ref,
        report.suite_ref,
        report.monitor_notifications.len(),
        restart_decision,
        report.cleanup_receipts.len(),
        report.retractions.len()
    ))
}

pub fn service_supervision_report_value(input: ReportValueInput<'_>) -> Result<IoValue> {
    validate_report_input(&input)?;
    Ok(crate::preserves_rail::record("service-supervision-report-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::SERVICE_SUPERVISION_REPORT_SCHEMA),
        crate::preserves_rail::record("suite", vec![input.suite_value.clone()]),
        crate::preserves_rail::record("failures", vec![crate::preserves_rail::sequence(
            input.failure_markers.to_vec(),
        )]),
        crate::preserves_rail::record("statuses", vec![crate::preserves_rail::sequence(input.statuses.to_vec())]),
        crate::preserves_rail::record("lifecycle", vec![crate::preserves_rail::sequence(
            input.lifecycle_receipts.to_vec(),
        )]),
        crate::preserves_rail::record("monitor-notifications", vec![crate::preserves_rail::sequence(
            input.monitor_notifications.to_vec(),
        )]),
        crate::preserves_rail::record("restart-decisions", vec![crate::preserves_rail::sequence(
            input.restart_decisions.to_vec(),
        )]),
        crate::preserves_rail::record("scheduled-demands", vec![crate::preserves_rail::sequence(
            input.scheduled_demands.to_vec(),
        )]),
        crate::preserves_rail::record("cleanup", vec![crate::preserves_rail::sequence(
            input.cleanup_receipts.to_vec(),
        )]),
        crate::preserves_rail::record("retractions", vec![crate::preserves_rail::sequence(input.retractions.to_vec())]),
        crate::preserves_rail::record("retention", vec![crate::preserves_rail::sequence(
            input.retention_inputs.to_vec(),
        )]),
        checks_value(&[
            "canonical-service-supervision-report",
            "monitor-order-bound",
            "cleanup-retention-bound",
        ]),
    ]))
}
