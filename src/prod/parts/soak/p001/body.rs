
pub fn resource_envelope_value(input: &ResourceEnvelopeInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_metric_bound("queue depth", input.queue_depth, input.max_queue_depth)?;
    validate_metric_bound("receipt bytes", input.receipt_bytes, input.max_receipt_bytes)?;
    validate_metric_bound("store bytes", input.store_bytes, input.max_store_bytes)?;
    validate_metric_bound("delivery latency ms", input.delivery_latency_ms, input.max_delivery_latency_ms)?;
    validate_metric_bound("recovery time ms", input.recovery_time_ms, input.max_recovery_time_ms)?;
    validate_ref_slice("resource pressure", input.pressure_refs)?;
    validate_ref_slice("resource denial", input.denial_refs)?;
    validate_pass_category("resource pressure", input.pressure_refs, input.decision)?;
    validate_pass_category("resource denial", input.denial_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-resource-envelope-v1", vec![
        string(RESOURCE_ENVELOPE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("queue-depth", vec![u64_value(input.queue_depth)]),
        record("max-queue-depth", vec![u64_value(input.max_queue_depth)]),
        record("receipt-bytes", vec![u64_value(input.receipt_bytes)]),
        record("max-receipt-bytes", vec![u64_value(input.max_receipt_bytes)]),
        record("store-bytes", vec![u64_value(input.store_bytes)]),
        record("max-store-bytes", vec![u64_value(input.max_store_bytes)]),
        record("delivery-latency-ms", vec![u64_value(input.delivery_latency_ms)]),
        record("max-delivery-latency-ms", vec![u64_value(input.max_delivery_latency_ms)]),
        record("recovery-time-ms", vec![u64_value(input.recovery_time_ms)]),
        record("max-recovery-time-ms", vec![u64_value(input.max_recovery_time_ms)]),
        record("pressure", vec![sequence(ref_values(input.pressure_refs)?)]),
        record("denials", vec![sequence(ref_values(input.denial_refs)?)]),
        record("diagnostics", vec![sequence(string_values(
            "resource diagnostic",
            input.diagnostics,
            MAX_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "resource caveat",
            input.caveats,
            MAX_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("queue-depth-bound", "pass"),
            check_value("receipt-growth-bound", "pass"),
            check_value("store-growth-bound", "pass"),
            check_value("delivery-latency-bound", "pass"),
            check_value("recovery-time-bound", "pass"),
            check_value("resource-pressure-denial-bound", "pass"),
        ])]),
    ]))
}

pub fn fault_case_value(input: &FaultCaseInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_fault_kind(input.fault_kind)?;
    validate_text_field("injection", input.injection)?;
    validate_text_field("expected outcome", input.expected_outcome)?;
    validate_ref_slice("fault evidence", input.evidence_refs)?;
    validate_ref_slice("fault denial", input.denial_refs)?;
    validate_text_field("replay status", input.replay_status)?;
    validate_pass_category("fault evidence", input.evidence_refs, input.decision)?;
    validate_pass_fault_denials(input.expected_outcome, input.denial_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-fault-case-v1", vec![
        string(FAULT_CASE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("fault-kind", vec![string(input.fault_kind)]),
        record("injection", vec![string(input.injection)]),
        record("expected-outcome", vec![string(input.expected_outcome)]),
        record("evidence", vec![sequence(ref_values(input.evidence_refs)?)]),
        record("denials", vec![sequence(ref_values(input.denial_refs)?)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("diagnostics", vec![sequence(string_values(
            "fault diagnostic",
            input.diagnostics,
            MAX_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values("fault caveat", input.caveats, MAX_TEXT_FIELDS)?)]),
        record("checks", vec![sequence(vec![
            check_value("fault-kind-covered", "pass"),
            check_value("fault-evidence-bound", "pass"),
            check_value(
                "deny-before-side-effects-bound",
                status(denial_required(input.expected_outcome) && input.denial_refs.is_empty()),
            ),
            check_value("fault-evidence-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

pub fn fault_matrix_value(input: &FaultMatrixInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_ref_slice("fault case", input.fault_case_refs)?;
    validate_fault_kinds(input.fault_kinds)?;
    validate_pass_category("fault case", input.fault_case_refs, input.decision)?;
    validate_fault_matrix_coverage(input.fault_kinds, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-fault-matrix-v1", vec![
        string(FAULT_MATRIX_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("fault-cases", vec![sequence(ref_values(input.fault_case_refs)?)]),
        record("fault-kinds", vec![sequence(input.fault_kinds.iter().map(string).collect())]),
        record("required-faults", vec![sequence(REQUIRED_NETWORK_FAULTS.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(string_values(
            "fault matrix diagnostic",
            input.diagnostics,
            MAX_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "fault matrix caveat",
            input.caveats,
            MAX_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("network-transport-fault-matrix", "pass"),
            check_value("required-fault-kinds-covered", status(missing_required_faults(input.fault_kinds).is_some())),
            check_value("fault-cases-bound", status(input.fault_case_refs.is_empty())),
            check_value("simulated-faults-marked-diagnostic", "pass"),
        ])]),
    ]))
}

fn validate_metric_bound(label: &str, actual: u64, maximum: u64) -> Result<()> {
    if actual > maximum {
        Err(MoltenError::invalid_harness(format!("prod soak {label} {actual} exceeds bound {maximum}")))
    } else {
        Ok(())
    }
}

fn validate_pass_category(label: &str, refs: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && refs.is_empty() {
        Err(MoltenError::invalid_harness(format!("passing prod soak run requires at least one {label} ref")))
    } else {
        Ok(())
    }
}

fn validate_pass_caveats(caveats: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && caveats.is_empty() {
        Err(MoltenError::invalid_harness("passing prod soak run requires explicit evidence-only caveats"))
    } else {
        Ok(())
    }
}

fn validate_fault_profile_refs(fault_profile: &str, fault_refs: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && fault_profile != "none" && fault_refs.is_empty() {
        Err(MoltenError::invalid_harness(
            "passing prod soak run with non-none fault profile requires fault refs",
        ))
    } else {
        Ok(())
    }
}

fn validate_pass_fault_denials(expected_outcome: &str, denial_refs: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && denial_required(expected_outcome) && denial_refs.is_empty() {
        Err(MoltenError::invalid_harness(
            "passing prod soak deny-before-side-effects fault requires denial refs",
        ))
    } else {
        Ok(())
    }
}

fn denial_required(expected_outcome: &str) -> bool {
    expected_outcome.contains("deny") || expected_outcome.contains("fail-closed")
}

fn validate_fault_kind(kind: &str) -> Result<()> {
    if REQUIRED_NETWORK_FAULTS.contains(&kind) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported prod soak fault kind {kind}; expected one of {}",
            REQUIRED_NETWORK_FAULTS.join(", ")
        )))
    }
}

fn validate_fault_kinds(kinds: &[String]) -> Result<()> {
    if kinds.len() > MAX_TEXT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak fault kind count {} exceeds bound {MAX_TEXT_FIELDS}",
            kinds.len()
        )));
    }
    for kind in kinds {
        validate_fault_kind(kind)?;
    }
    Ok(())
}

fn validate_fault_matrix_coverage(kinds: &[String], decision: &str) -> Result<()> {
    if decision == "pass"
        && let Some(missing) = missing_required_faults(kinds)
    {
        Err(MoltenError::invalid_harness(format!(
            "passing prod soak fault matrix missing fault kinds: {}",
            missing.join(", ")
        )))
    } else {
        Ok(())
    }
}

fn missing_required_faults(kinds: &[String]) -> Option<Vec<String>> {
    let present = kinds.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>();
    let missing = REQUIRED_NETWORK_FAULTS
        .iter()
        .filter(|kind| !present.contains(**kind))
        .map(|kind| (*kind).to_string())
        .collect::<Vec<_>>();
    if missing.is_empty() { None } else { Some(missing) }
}

fn validate_text_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("prod soak {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak {label} ref count {} exceeds bound {MAX_REFS}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid prod soak {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "unavailable" | "skipped" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported prod soak decision {other}; expected pass, deny, unavailable, or skipped"
        ))),
    }
}

fn ref_values(refs: &[String]) -> Result<Vec<IoValue>> {
    validate_ref_slice("artifact", refs)?;
    Ok(refs.iter().map(string).collect())
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IoValue>> {
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak {label} count {} exceeds bound {maximum}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text_field(label, value)?;
        output.push(string(value));
    }
    Ok(output)
}

fn check_value(name: &'static str, status: &'static str) -> IoValue {
    record("check", vec![string(name), string(status)])
}

fn status(is_problem: bool) -> &'static str {
    if is_problem { "deny" } else { "pass" }
}
