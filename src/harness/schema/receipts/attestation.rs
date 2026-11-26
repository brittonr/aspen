pub fn run_receipt_value(
    report_value: &preserves::IOValue,
    export_refs: &[&str],
) -> crate::error::Result<preserves::IOValue> {
    let report = super::reading::parse_report(report_value)?;
    for export_ref in export_refs {
        crate::preserves_rail::validate_content_ref(export_ref)?;
    }
    let step_results = report
        .observations
        .iter()
        .map(|observation| {
            crate::preserves_rail::record("step-result", vec![
                crate::preserves_rail::record("index", vec![crate::preserves_rail::u64_value(observation.index)]),
                crate::preserves_rail::record("step-ref", vec![crate::preserves_rail::string(&observation.step_ref)]),
                crate::preserves_rail::record("observation-ref", vec![crate::preserves_rail::string(
                    &observation.observation_ref,
                )]),
                crate::preserves_rail::record("status", vec![crate::preserves_rail::string("pass")]),
            ])
        })
        .collect::<Vec<_>>();
    let adapter_fixture_ref = report
        .executor_preflights
        .as_ref()
        .map(|preflights| crate::preserves_rail::canonical_hash(&preflights.value))
        .transpose()?;
    Ok(crate::preserves_rail::record("harness-run-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_RUN_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("suite-start", vec![crate::preserves_rail::string(&report.suite_ref)]),
        crate::preserves_rail::record("report-ref", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("step-results", vec![crate::preserves_rail::sequence(step_results)]),
        crate::preserves_rail::record("adapter-fixture-decision-ref", vec![crate::preserves_rail::string(
            adapter_fixture_ref.as_deref().unwrap_or("none"),
        )]),
        crate::preserves_rail::record("expected-failure-refs", vec![crate::preserves_rail::sequence(Vec::new())]),
        crate::preserves_rail::record("known-bug-refs", vec![crate::preserves_rail::sequence(Vec::new())]),
        crate::preserves_rail::record("final-status", vec![crate::preserves_rail::string(&report.status)]),
        crate::preserves_rail::record("report-export-refs", vec![refs_sequence_from_strs(export_refs)]),
        super::calls::hostcall_checks_value(&[
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

pub fn validate_harness_run_receipt(
    value: &preserves::IOValue,
    report_value: &preserves::IOValue,
    export_refs: &[&str],
) -> crate::error::Result<()> {
    let expected = run_receipt_value(report_value, export_refs)?;
    if crate::preserves_rail::canonical_hash(value)? != crate::preserves_rail::canonical_hash(&expected)? {
        return Err(crate::error::Failure::invalid_harness(
            "harness run receipt does not match report and export refs",
        ));
    }
    let receipt = super::fields::simple_record(value, "harness-run-receipt-v1", 11)?;
    let schema = super::fields::required_string(&receipt[0], "harness run receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_RUN_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported harness run receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_RUN_RECEIPT_SCHEMA
        )));
    }
    if super::fields::required_record_string(&receipt[1], "decision", "harness run receipt decision")? != "pass" {
        return Err(crate::error::Failure::invalid_harness("harness run receipt decision must be pass"));
    }
    if super::fields::required_record_string(&receipt[8], "final-status", "harness run final status")? != "pass" {
        return Err(crate::error::Failure::invalid_harness("harness run final status must be pass"));
    }
    let checks = super::clearance::parse_executor_preflight_checks(&receipt[10])?;
    for expected in [
        "suite-start-bound",
        "step-results-bound",
        "adapter-fixture-decision-bound",
        "expected-failure-recorded",
        "known-bug-recorded",
        "final-status-bound",
        "report-export-bound",
    ] {
        super::clearance::require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

fn refs_sequence_from_strs(refs: &[&str]) -> preserves::IOValue {
    crate::preserves_rail::sequence(refs.iter().map(|value| crate::preserves_rail::string(*value)).collect())
}

pub fn deterministic_multipeer_receipt_value(
    report_value: &preserves::IOValue,
    seed: u64,
    profile: &str,
    peer_events: &[&str],
) -> crate::error::Result<preserves::IOValue> {
    validate_multipeer_profile(profile)?;
    if peer_events.is_empty() {
        return Err(crate::error::Failure::invalid_harness(
            "deterministic multi-peer receipt requires at least one peer event",
        ));
    }
    for event in peer_events {
        validate_multipeer_event(event)?;
    }
    let report = super::reading::parse_report(report_value)?;
    let event_values = peer_events.iter().map(|event| crate::preserves_rail::string(*event)).collect::<Vec<_>>();
    let schedule_value = crate::preserves_rail::record("multipeer-schedule-v1", vec![
        crate::preserves_rail::record("seed", vec![crate::preserves_rail::u64_value(seed)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(profile)]),
        crate::preserves_rail::record("events", vec![crate::preserves_rail::sequence(event_values.clone())]),
    ]);
    let schedule_ref = crate::preserves_rail::canonical_hash(&schedule_value)?;
    let peer_count = report
        .actors
        .iter()
        .filter(|actor| matches!(actor.kind, super::model::ActorKind::RemoteProxy | super::model::ActorKind::Adapter))
        .count() as u64;
    Ok(crate::preserves_rail::record("deterministic-multipeer-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::HARNESS_DETERMINISTIC_MULTIPEER_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string("pass")]),
        crate::preserves_rail::record("replay", vec![crate::preserves_rail::string("stable")]),
        crate::preserves_rail::record("suite-ref", vec![crate::preserves_rail::string(&report.suite_ref)]),
        crate::preserves_rail::record("report-ref", vec![crate::preserves_rail::string(&report.report_ref)]),
        crate::preserves_rail::record("seed", vec![crate::preserves_rail::u64_value(seed)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(profile)]),
        crate::preserves_rail::record("schedule-ref", vec![crate::preserves_rail::string(&schedule_ref)]),
        schedule_value,
        crate::preserves_rail::record("peer-count", vec![crate::preserves_rail::u64_value(peer_count)]),
        crate::preserves_rail::record("trace-ref", vec![crate::preserves_rail::string(
            super::migration::report_trace_ref(&report)?,
        )]),
        crate::preserves_rail::record("resource-budget-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::canonical_hash(&super::allowance::budget_value(
                &report.budget.limits,
                &report.budget.usage,
            ))?,
        )]),
        super::calls::hostcall_checks_value(&[
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
    value: &preserves::IOValue,
    report_value: &preserves::IOValue,
    seed: u64,
    profile: &str,
    peer_events: &[&str],
) -> crate::error::Result<()> {
    let expected = deterministic_multipeer_receipt_value(report_value, seed, profile, peer_events)?;
    if crate::preserves_rail::canonical_hash(value)? != crate::preserves_rail::canonical_hash(&expected)? {
        return Err(crate::error::Failure::invalid_harness(
            "deterministic multi-peer receipt does not match replayed schedule",
        ));
    }
    let receipt = super::fields::simple_record(value, "deterministic-multipeer-receipt-v1", 13)?;
    let schema = super::fields::required_string(&receipt[0], "deterministic multi-peer receipt schema")?;
    if schema != crate::preserves_rail::HARNESS_DETERMINISTIC_MULTIPEER_RECEIPT_SCHEMA {
        return Err(crate::error::Failure::invalid_harness(format!(
            "unsupported deterministic multi-peer receipt schema {schema}; expected {}",
            crate::preserves_rail::HARNESS_DETERMINISTIC_MULTIPEER_RECEIPT_SCHEMA
        )));
    }
    if super::fields::required_record_string(&receipt[1], "decision", "deterministic multi-peer decision")? != "pass" {
        return Err(crate::error::Failure::invalid_harness("deterministic multi-peer decision must be pass"));
    }
    if super::fields::required_record_string(&receipt[2], "replay", "deterministic multi-peer replay")? != "stable" {
        return Err(crate::error::Failure::invalid_harness("deterministic multi-peer replay must be stable"));
    }
    let checks = super::clearance::parse_executor_preflight_checks(&receipt[12])?;
    for expected in [
        "seeded-peer-delivery",
        "partition-replay-stable",
        "drop-reorder-reconnect-profile",
        "gossip-doc-blob-observations",
        "resource-limit-binding",
        "no-live-unrecorded-peer-io",
    ] {
        super::clearance::require_executor_preflight_check(&checks, expected)?;
    }
    Ok(())
}

fn validate_multipeer_profile(profile: &str) -> crate::error::Result<()> {
    if matches!(profile, "seeded" | "recorded") {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!(
            "unsupported deterministic multi-peer profile {profile}; expected seeded or recorded"
        )))
    }
}

fn validate_multipeer_event(event: &str) -> crate::error::Result<()> {
    if matches!(
        event,
        "deliver" | "partition" | "drop" | "reorder" | "reconnect" | "resource-limit" | "gossip" | "doc" | "blob"
    ) {
        Ok(())
    } else {
        Err(crate::error::Failure::invalid_harness(format!(
            "unsupported deterministic multi-peer event {event}; live or unrecorded peer delivery cannot satisfy replay"
        )))
    }
}
