pub fn upgrade_replay_receipt_value(
    old_report_value: &preserves::IOValue,
    new_report_value: &preserves::IOValue,
    migration_receipt_ref: Option<&str>,
    compatibility_diagnostic_ref: Option<&str>,
) -> crate::error::Result<preserves::IOValue> {
    let old_report = super::reading::parse_report(old_report_value)?;
    let new_report = super::reading::parse_report(new_report_value)?;
    let old_trace_ref = report_trace_ref(&old_report)?;
    let new_trace_ref = report_trace_ref(&new_report)?;
    let is_stable_replay = old_trace_ref == new_trace_ref && old_report.final_state_hash == new_report.final_state_hash;
    if let Some(migration_receipt_ref) = migration_receipt_ref {
        crate::preserves_rail::validate_content_ref(migration_receipt_ref)?;
    }
    if let Some(compatibility_diagnostic_ref) = compatibility_diagnostic_ref {
        crate::preserves_rail::validate_content_ref(compatibility_diagnostic_ref)?;
    }
    if !is_stable_replay && migration_receipt_ref.is_none() && compatibility_diagnostic_ref.is_none() {
        return Err(crate::error::Failure::invalid_harness(
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
    Ok(crate::preserves_rail::record("upgrade-replay-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_UPGRADE_REPLAY_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("outcome", vec![crate::preserves_rail::string(outcome)]),
        crate::preserves_rail::record("old-report-ref", vec![crate::preserves_rail::string(&old_report.report_ref)]),
        crate::preserves_rail::record("new-report-ref", vec![crate::preserves_rail::string(&new_report.report_ref)]),
        crate::preserves_rail::record("old-trace-ref", vec![crate::preserves_rail::string(&old_trace_ref)]),
        crate::preserves_rail::record("new-trace-ref", vec![crate::preserves_rail::string(&new_trace_ref)]),
        crate::preserves_rail::record("old-state-ref", vec![crate::preserves_rail::string(
            &old_report.final_state_hash,
        )]),
        crate::preserves_rail::record("new-state-ref", vec![crate::preserves_rail::string(
            &new_report.final_state_hash,
        )]),
        crate::preserves_rail::record("migration-receipt-ref", vec![crate::preserves_rail::string(
            migration_receipt_ref.unwrap_or("none"),
        )]),
        crate::preserves_rail::record("compatibility-diagnostic-ref", vec![crate::preserves_rail::string(
            compatibility_diagnostic_ref.unwrap_or("none"),
        )]),
        super::calls::hostcall_checks_value(&[
            "old-report-bound",
            "new-report-bound",
            "canonical-trace-compare",
            "state-hash-compare",
            "drift-explained",
        ]),
    ]))
}

pub fn validate_upgrade_replay_receipt(
    value: &preserves::IOValue,
    old_report_value: &preserves::IOValue,
    new_report_value: &preserves::IOValue,
) -> crate::error::Result<()> {
    let receipt = super::fields::simple_record(value, "upgrade-replay-receipt-v1", 12)?;
    let outcome = require_upgrade_replay_header(&receipt)?;
    let old_report = super::reading::parse_report(old_report_value)?;
    let new_report = super::reading::parse_report(new_report_value)?;
    let (old_trace_ref, new_trace_ref) = require_upgrade_replay_report_refs(&receipt, &old_report, &new_report)?;
    let migration_receipt_ref =
        super::fields::required_record_string(&receipt[9], "migration-receipt-ref", "migration receipt ref")?;
    let compatibility_diagnostic_ref = super::fields::required_record_string(
        &receipt[10],
        "compatibility-diagnostic-ref",
        "compatibility diagnostic ref",
    )?;
    let is_stable_replay = old_trace_ref == new_trace_ref && old_report.final_state_hash == new_report.final_state_hash;
    require_upgrade_replay_drift_evidence(
        &outcome,
        &migration_receipt_ref,
        &compatibility_diagnostic_ref,
        is_stable_replay,
    )?;
    require_upgrade_replay_checks(&receipt[11])
}

fn require_upgrade_replay_header(
    receipt: &preserves::Record<preserves::Value<preserves::IOValue>>,
) -> crate::error::Result<String> {
    let schema = super::fields::required_string(&receipt[0], "upgrade replay receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_UPGRADE_REPLAY_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported upgrade replay receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_UPGRADE_REPLAY_RECEIPT_SCHEMA
        )));
    }
    let decision = super::fields::required_record_string(&receipt[1], "decision", "upgrade replay decision")?;
    if decision != "pass" {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported upgrade replay decision {decision}")));
    }
    let outcome = super::fields::required_record_string(&receipt[2], "outcome", "upgrade replay outcome")?;
    if !matches!(outcome.as_str(), "stable" | "migrated" | "diagnosed") {
        return Err(crate::error::Failure::invalid_harness(format!("unsupported upgrade replay outcome {outcome}")));
    }
    Ok(outcome)
}

fn require_upgrade_replay_report_refs(
    receipt: &preserves::Record<preserves::Value<preserves::IOValue>>,
    old_report: &super::model::Report,
    new_report: &super::model::Report,
) -> crate::error::Result<(String, String)> {
    if super::fields::required_record_hash(&receipt[3], "old-report-ref", "old upgrade report ref")?
        != old_report.report_ref
    {
        return Err(crate::error::Failure::invalid_harness("upgrade replay old report ref mismatch"));
    }
    if super::fields::required_record_hash(&receipt[4], "new-report-ref", "new upgrade report ref")?
        != new_report.report_ref
    {
        return Err(crate::error::Failure::invalid_harness("upgrade replay new report ref mismatch"));
    }
    let old_trace_ref = report_trace_ref(old_report)?;
    let new_trace_ref = report_trace_ref(new_report)?;
    if super::fields::required_record_hash(&receipt[5], "old-trace-ref", "old upgrade trace ref")? != old_trace_ref {
        return Err(crate::error::Failure::invalid_harness("upgrade replay old trace ref mismatch"));
    }
    if super::fields::required_record_hash(&receipt[6], "new-trace-ref", "new upgrade trace ref")? != new_trace_ref {
        return Err(crate::error::Failure::invalid_harness("upgrade replay new trace ref mismatch"));
    }
    if super::fields::required_record_hash(&receipt[7], "old-state-ref", "old upgrade state ref")?
        != old_report.final_state_hash
    {
        return Err(crate::error::Failure::invalid_harness("upgrade replay old state ref mismatch"));
    }
    if super::fields::required_record_hash(&receipt[8], "new-state-ref", "new upgrade state ref")?
        != new_report.final_state_hash
    {
        return Err(crate::error::Failure::invalid_harness("upgrade replay new state ref mismatch"));
    }
    Ok((old_trace_ref, new_trace_ref))
}

fn require_upgrade_replay_drift_evidence(
    outcome: &str,
    migration_receipt_ref: &str,
    compatibility_diagnostic_ref: &str,
    is_stable_replay: bool,
) -> crate::error::Result<()> {
    if migration_receipt_ref != "none" {
        crate::preserves_rail::validate_content_ref(migration_receipt_ref)?;
    }
    if compatibility_diagnostic_ref != "none" {
        crate::preserves_rail::validate_content_ref(compatibility_diagnostic_ref)?;
    }
    if !is_stable_replay && migration_receipt_ref == "none" && compatibility_diagnostic_ref == "none" {
        return Err(crate::error::Failure::invalid_harness(
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
        return Err(crate::error::Failure::invalid_harness("upgrade replay outcome does not match drift evidence"));
    }
    Ok(())
}

fn require_upgrade_replay_checks(checks_value: &preserves::Value<preserves::IOValue>) -> crate::error::Result<()> {
    let checks = super::clearance::parse_executor_preflight_checks(checks_value)?;
    for expected in [
        "old-report-bound",
        "new-report-bound",
        "canonical-trace-compare",
        "state-hash-compare",
        "drift-explained",
    ] {
        super::clearance::require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

pub(super) fn report_trace_ref(report: &super::model::Report) -> crate::error::Result<String> {
    crate::preserves_rail::canonical_hash(&crate::preserves_rail::sequence(
        report.observations.iter().map(|observation| observation.value.clone()).collect(),
    ))
}
