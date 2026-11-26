pub fn golden_trace_update_receipt_value(
    previous_report_ref: Option<&str>,
    updated_report_value: &preserves::IOValue,
    reason: &str,
    reviewer_ref: &str,
) -> crate::error::Result<preserves::IOValue> {
    validate_golden_trace_update_reason(reason)?;
    if let Some(previous_report_ref) = previous_report_ref {
        crate::preserves_rail::validate_content_ref(previous_report_ref)?;
    }
    crate::preserves_rail::validate_content_ref(reviewer_ref)?;
    let report = super::reading::parse_report(updated_report_value)?;
    Ok(crate::preserves_rail::record("golden-trace-update-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_GOLDEN_TRACE_UPDATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("reason", vec![crate::preserves_rail::string(reason)]),
        crate::preserves_rail::record("previous-report-ref", vec![crate::preserves_rail::string(
            previous_report_ref.unwrap_or("none"),
        )]),
        crate::preserves_rail::record("updated-report-ref", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&report.suite_ref)]),
        crate::preserves_rail::record("trace-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::sequence(
                report.observations.iter().map(|observation| observation.value.clone()).collect(),
            ))?,
        )]),
        crate::preserves_rail::record("receipt-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&crate::preserves_rail::record(
                "harness-golden-receipt-anchor",
                vec![
                    crate::preserves_rail::string(&report.report_ref),
                    crate::preserves_rail::string(&report.suite_ref),
                    crate::preserves_rail::string(&report.final_state_hash),
                ],
            ))?,
        )]),
        crate::preserves_rail::record("state-ref", vec![crate::preserves_rail::string(&report.final_state_hash)]),
        crate::preserves_rail::record("reviewer-ref", vec![crate::preserves_rail::string(reviewer_ref)]),
        super::calls::hostcall_checks_value(&[
            "reviewed-update-receipt",
            "canonical-trace-ref",
            "canonical-receipt-ref",
            "canonical-state-ref",
            "reason-classification",
        ]),
    ]))
}

pub fn validate_golden_trace_update_receipt(
    value: &preserves::IOValue,
    updated_report_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let receipt = super::fields::simple_record(value, "golden-trace-update-receipt-v1", 11)?;
    validate_golden_trace_update_header(&receipt)?;
    let report = super::reading::parse_report(updated_report_value)?;
    let updated_report_ref =
        super::fields::required_record_hash(&receipt[4], "updated-report-ref", "updated golden report ref")?;
    if updated_report_ref != report.report_ref {
        return Err(crate::error::Failure::invalid_harness(
            "golden trace update report ref does not match updated report",
        ));
    }
    let suite_ref = super::fields::required_record_hash(&receipt[5], "suite-ref", "golden trace suite ref")?;
    if suite_ref != report.suite_ref {
        return Err(crate::error::Failure::invalid_harness(
            "golden trace update suite ref does not match updated report",
        ));
    }
    let expected_trace_ref = crate::preserves_rail::canonical_hash(&crate::preserves_rail::sequence(
        report.observations.iter().map(|observation| observation.value.clone()).collect(),
    ))?;
    let trace_ref = super::fields::required_record_hash(&receipt[6], "trace-ref", "golden trace ref")?;
    if trace_ref != expected_trace_ref {
        return Err(crate::error::Failure::invalid_harness(
            "golden trace update trace ref does not match report observations",
        ));
    }
    let expected_receipt_ref =
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("harness-golden-receipt-anchor", vec![
            crate::preserves_rail::string(&report.report_ref),
            crate::preserves_rail::string(&report.suite_ref),
            crate::preserves_rail::string(&report.final_state_hash),
        ]))?;
    let receipt_ref = super::fields::required_record_hash(&receipt[7], "receipt-ref", "golden receipt ref")?;
    if receipt_ref != expected_receipt_ref {
        return Err(crate::error::Failure::invalid_harness("golden trace update receipt ref does not match report"));
    }
    let state_ref = super::fields::required_record_hash(&receipt[8], "state-ref", "golden state ref")?;
    if state_ref != report.final_state_hash {
        return Err(crate::error::Failure::invalid_harness("golden trace update state ref does not match final state"));
    }
    let reviewer_ref = super::fields::required_record_hash(&receipt[9], "reviewer-ref", "golden trace reviewer ref")?;
    crate::preserves_rail::validate_content_ref(&reviewer_ref)?;
    let checks = super::clearance::parse_executor_preflight_checks(&receipt[10])?;
    for expected in [
        "reviewed-update-receipt",
        "canonical-trace-ref",
        "canonical-receipt-ref",
        "canonical-state-ref",
        "reason-classification",
    ] {
        super::clearance::require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

fn validate_golden_trace_update_header(
    receipt: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<()> {
    let schema = super::fields::required_string(&receipt[0], "golden trace update receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_GOLDEN_TRACE_UPDATE_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported golden trace update receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_GOLDEN_TRACE_UPDATE_RECEIPT_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&receipt[1], "decision", "golden trace update decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported golden trace update decision {decision}"
        )));
    }
    let reason = super::fields::required_record_string(&receipt[2], "reason", "golden trace update reason")?;
    validate_golden_trace_update_reason(&reason)?;
    let previous_report_ref =
        super::fields::required_record_string(&receipt[3], "previous-report-ref", "previous golden report ref")?;
    if previous_report_ref != "none" {
        crate::preserves_rail::validate_content_ref(&previous_report_ref)?;
    }
    Ok(())
}

fn validate_golden_trace_update_reason(reason: &str) -> crate::error::Result<()> {
    if matches!(reason, "schema-driven" | "policy-driven" | "migration-driven" | "bug-fix") {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!(
            "unsupported golden trace update reason {reason}; expected schema-driven, policy-driven, migration-driven, or bug-fix"
        )))
    }
}
