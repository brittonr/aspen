
fn optional_value_vec(value: Option<IoValue>) -> Vec<IoValue> {
    value.map_or_else(Vec::new, |value| vec![value])
}

fn validate_suite_input(input: &ServiceSupervisionSuiteInput) -> Result<()> {
    ensure_count_at_most(input.links.len(), "service supervision links")?;
    ensure_count_at_most(input.monitors.len(), "service supervision monitors")?;
    crate::service_records::parse_service_manifest(&input.manifest)?;
    crate::service_records::parse_service_restart_policy(&input.restart_policy)?;
    parse_service_owned_state(&input.owned_state)?;
    for link in &input.links {
        crate::service_records::parse_service_link(link)?;
    }
    for monitor in &input.monitors {
        crate::service_records::parse_service_monitor(monitor)?;
    }
    validate_evidence(&input.evidence)
}

fn validate_suite_parsed(suite: &ServiceSupervisionSuite) -> Result<()> {
    if suite.owned_state.service_id != suite.manifest.service_id {
        return Err(MoltenError::invalid_harness("owned state service id must match manifest service id"));
    }
    if suite.owned_state.manifest_ref.as_deref() != Some(suite.manifest.manifest_ref.as_str()) {
        return Err(MoltenError::invalid_harness("owned state manifest ref must match manifest"));
    }
    validate_evidence(&suite.evidence)
}

fn validate_owned_state_input(input: &ServiceOwnedStateInput) -> Result<()> {
    validate_service_id(&input.service_id, "service owned-state service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service owned-state manifest ref")?;
    validate_refs(&input.owned_assertion_refs, "service owned assertion ref")?;
    validate_refs(&input.observer_refs, "service observer ref")?;
    validate_refs(&input.live_ref_refs, "service live ref")?;
    validate_refs(&input.exposed_ref_refs, "service exposed ref")?;
    validate_refs(&input.pending_effect_refs, "service pending effect ref")?;
    validate_refs(&input.foreign_ref_claims, "service foreign claim ref")
}

fn validate_owned_state_parsed(owned_state: &ServiceOwnedState) -> Result<()> {
    validate_service_id(&owned_state.service_id, "service owned-state service id")?;
    validate_optional_ref(owned_state.manifest_ref.as_deref(), "service owned-state manifest ref")?;
    validate_refs(&owned_state.foreign_ref_claims, "service foreign claim ref")
}

fn validate_evidence(evidence: &ServiceSupervisionEvidenceInput) -> Result<()> {
    validate_refs(&evidence.authority_refs, "service supervision authority ref")?;
    validate_refs(&evidence.resource_refs, "service supervision resource ref")?;
    validate_refs(&evidence.revocation_refs, "service supervision revocation ref")?;
    validate_refs(&evidence.retention_policy_refs, "service supervision retention ref")?;
    validate_refs(&evidence.prior_lifecycle_refs, "service supervision lifecycle ref")?;
    validate_refs(&evidence.effect_log_refs, "service supervision effect log ref")
}

fn validate_report_input(input: &ReportValueInput<'_>) -> Result<()> {
    ensure_count_at_most(input.failure_markers.len(), "service supervision failures")?;
    ensure_count_at_most(input.statuses.len(), "service supervision statuses")?;
    ensure_count_at_most(input.lifecycle_receipts.len(), "service supervision lifecycle receipts")?;
    ensure_count_at_most(input.monitor_notifications.len(), "service monitor notifications")?;
    ensure_count_at_most(input.restart_decisions.len(), "service restart decisions")?;
    ensure_count_at_most(input.scheduled_demands.len(), "service scheduled demands")?;
    ensure_count_at_most(input.cleanup_receipts.len(), "service cleanup receipts")?;
    ensure_count_at_most(input.retractions.len(), "service retractions")?;
    ensure_count_at_most(input.retention_inputs.len(), "service retention inputs")
}

fn ensure_count_at_most(actual: usize, label: &str) -> Result<()> {
    if actual <= MAX_SUPERVISION_ITEMS {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{label} count {actual} exceeds bound {MAX_SUPERVISION_ITEMS}"
        )))
    }
}

fn evidence_value(input: &ServiceSupervisionEvidenceInput) -> IoValue {
    crate::preserves_rail::record("evidence", vec![
        crate::preserves_rail::record("authority", vec![refs_sequence(&input.authority_refs)]),
        crate::preserves_rail::record("resource", vec![refs_sequence(&input.resource_refs)]),
        crate::preserves_rail::record("revocations", vec![refs_sequence(&input.revocation_refs)]),
        crate::preserves_rail::record("retention", vec![refs_sequence(&input.retention_policy_refs)]),
        crate::preserves_rail::record("prior-lifecycle", vec![refs_sequence(&input.prior_lifecycle_refs)]),
        crate::preserves_rail::record("effect-log", vec![refs_sequence(&input.effect_log_refs)]),
    ])
}

fn parse_evidence(value: &Value<IoValue>) -> Result<ServiceSupervisionEvidenceInput> {
    let fields = value
        .collect_simple_record("evidence", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected service supervision evidence"))?;
    Ok(ServiceSupervisionEvidenceInput {
        authority_refs: parse_ref_sequence(&fields[0], "authority")?,
        resource_refs: parse_ref_sequence(&fields[1], "resource")?,
        revocation_refs: parse_ref_sequence(&fields[2], "revocations")?,
        retention_policy_refs: parse_ref_sequence(&fields[3], "retention")?,
        prior_lifecycle_refs: parse_ref_sequence(&fields[4], "prior-lifecycle")?,
        effect_log_refs: parse_ref_sequence(&fields[5], "effect-log")?,
    })
}

fn parse_link_sequence(value: &Value<IoValue>) -> Result<Vec<crate::service_records::ServiceLink>> {
    parse_iovalue_sequence(value, "links")?
        .iter()
        .map(crate::service_records::parse_service_link)
        .collect()
}

fn parse_monitor_sequence(value: &Value<IoValue>) -> Result<Vec<crate::service_records::ServiceMonitor>> {
    parse_iovalue_sequence(value, "monitors")?
        .iter()
        .map(crate::service_records::parse_service_monitor)
        .collect()
}

fn parse_iovalue_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<IoValue>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), label)?;
    Ok(values.iter().map(crate::preserves_rail::value_to_iovalue).collect())
}

fn record_iovalue(value: &Value<IoValue>, label: &str) -> Result<IoValue> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(crate::preserves_rail::value_to_iovalue(&fields[0]))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} N>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} STRING>")))?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} OPTION>")))?;
    if fields[0].collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = fields[0]
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional ref for {label}")))?;
    let reference = required_string(&some[0], label)?;
    require_ref(&reference, label)?;
    Ok(Some(reference))
}

fn record_optional_string(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} OPTION>")))?;
    if fields[0].collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = fields[0]
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected optional string for {label}")))?;
    required_string(&some[0], label).map(Some)
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    let refs = values.iter().map(|value| required_ref(value, label)).collect::<Result<Vec<_>>>()?;
    validate_refs(&refs, label)?;
    Ok(refs)
}

fn parse_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Value<IoValue>>> {
    let value = crate::preserves_rail::value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} [...]>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let checks = field_sequence(value, "checks")?;
    ensure_count_at_most(checks.len(), "checks")?;
    let mut parsed = Vec::with_capacity(checks.len());
    for check in checks {
        let check = crate::preserves_rail::value_to_iovalue(&check);
        let check_fields = check
            .collect_simple_record("check", Some(2))
            .ok_or_else(|| MoltenError::invalid_harness("expected <check NAME STATUS>"))?;
        parsed.push((
            required_string(&check_fields[0], "check name")?,
            required_string(&check_fields[1], "check status")?,
        ));
    }
    Ok(parsed)
}

fn require_schema(value: &Value<IoValue>, expected: &str, label: &str) -> Result<()> {
    let actual = required_string(value, label)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {expected} for {label}, got {actual}")))
    }
}

fn require_check(checks: &[(String, String)], name: &str, label: &str) -> Result<()> {
    if checks.iter().any(|(check_name, status)| check_name == name && status == "pass") {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!("missing passing check {name} for {label}")))
}

fn refs_sequence(values: &[String]) -> IoValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    crate::preserves_rail::sequence(values.iter().map(crate::preserves_rail::string).collect())
}

fn checks_value(values: &[&str]) -> IoValue {
    crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(
        values
            .iter()
            .map(|value| {
                crate::preserves_rail::record("check", vec![
                    crate::preserves_rail::string(value),
                    crate::preserves_rail::string("pass"),
                ])
            })
            .collect(),
    )])
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn optional_string_value(value: Option<&str>) -> IoValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
    )
}

fn count_as_u64(count: usize, label: &str) -> Result<u64> {
    u64::try_from(count).map_err(|_| MoltenError::invalid_harness(format!("{label} does not fit u64")))
}

fn validate_decision(decision: &str, label: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {label} {decision}")))
    }
}

fn validate_restart_decision(decision: &str, label: &str) -> Result<()> {
    if matches!(decision, "pass" | "deny" | "backoff") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {label} {decision}")))
    }
}
