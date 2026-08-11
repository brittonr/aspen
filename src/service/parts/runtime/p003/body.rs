
fn turn_context_value(
    demand: &crate::service_records::ServiceDemand,
    manifest: &crate::service_records::ServiceManifest,
    readiness_ref: &str,
    events: &[crate::runtime::RuntimeEvent],
) -> Result<preserves::IOValue> {
    let event_labels = events.iter().map(runtime_event_label).collect::<Vec<_>>();
    Ok(record("service-turn-context-v1", vec![
        string(SERVICE_TURN_CONTEXT_SCHEMA),
        record("service-id", vec![string(&manifest.service_id)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("demand", vec![string(&demand.demand_ref)]),
        record("readiness", vec![string(readiness_ref)]),
        record("runtime-events", vec![sequence(event_labels.into_iter().map(string).collect())]),
        checks_value(&["actor-scoped", "owned-assertion-committed", "turn-context-bound"]),
    ]))
}

fn runtime_event_label(event: &crate::runtime::RuntimeEvent) -> &'static str {
    match event {
        crate::runtime::RuntimeEvent::MessageDelivered { .. } => "message-delivered",
        crate::runtime::RuntimeEvent::ObserveRegistered { .. } => "observe-registered",
        crate::runtime::RuntimeEvent::AssertionObserved { .. } => "assertion-observed",
        crate::runtime::RuntimeEvent::AssertionCommitted { .. } => "assertion-committed",
        crate::runtime::RuntimeEvent::AssertionRetracted { .. } => "assertion-retracted",
        crate::runtime::RuntimeEvent::AssertionRetractionObserved { .. } => "assertion-retraction-observed",
        crate::runtime::RuntimeEvent::EffectRequest { .. } => "effect-request",
        crate::runtime::RuntimeEvent::EffectResponse { .. } => "effect-response",
        crate::runtime::RuntimeEvent::AdmissionDecision { .. } => "admission-decision",
        crate::runtime::RuntimeEvent::TurnRolledBack { .. } => "turn-rolled-back",
    }
}

fn ready_status_map(statuses: &[crate::service_records::ServiceStatus]) -> Result<OrderedMap<String, String>> {
    let mut ready = OrderedMap::new();
    for status in statuses {
        if status.state == "ready" && ready.insert(status.service_id.clone(), status.status_ref.clone()).is_some() {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate ready service status for {}",
                status.service_id
            )));
        }
    }
    Ok(ready)
}

fn dependency_status_refs(
    manifest: &crate::service_records::ServiceManifest,
    ready_statuses: &OrderedMap<String, String>,
) -> Vec<String> {
    manifest
        .dependencies
        .iter()
        .filter_map(|service_id| ready_statuses.get(service_id).cloned())
        .collect()
}

fn manifest_ref_mismatch(
    demand: &crate::service_records::ServiceDemand,
    manifest: &crate::service_records::ServiceManifest,
) -> bool {
    demand.manifest_ref.as_ref().is_some_and(|manifest_ref| manifest_ref != &manifest.manifest_ref)
}

// r[impl molten.sam_service_demand_runtime.spec.dependency_resolution]
fn dependency_cycle_exists(manifests: &OrderedMap<String, crate::service_records::ServiceManifest>) -> Result<bool> {
    for service_id in manifests.keys() {
        let mut stack = manifests.get(service_id).map(|manifest| manifest.dependencies.clone()).unwrap_or_default();
        let mut seen = OrderedSet::new();
        while let Some(next) = stack.pop() {
            if &next == service_id {
                return Ok(true);
            }
            if !seen.insert(next.clone()) {
                continue;
            }
            if seen.len() > MAX_RUNTIME_ITEMS {
                return Err(MoltenError::invalid_harness("service dependency graph exceeds bound"));
            }
            if let Some(manifest) = manifests.get(&next) {
                for dependency in &manifest.dependencies {
                    stack.push(dependency.clone());
                }
            }
        }
    }
    Ok(false)
}

fn validate_suite_input(input: &SuiteInput) -> Result<()> {
    ensure_count_at_most(input.manifests.len(), "service manifests")?;
    ensure_count_at_most(input.demands.len(), "service demands")?;
    ensure_count_at_most(input.statuses.len(), "service statuses")?;
    for manifest in &input.manifests {
        crate::service_records::parse_service_manifest(manifest)?;
    }
    for demand in &input.demands {
        crate::service_records::parse_service_demand(demand)?;
    }
    for status in &input.statuses {
        crate::service_records::parse_service_status(status)?;
    }
    validate_runtime_evidence(&input.evidence)
}

fn validate_runtime_evidence(evidence: &EvidenceInput) -> Result<()> {
    validate_refs(&evidence.authority_refs, "service runtime authority ref")?;
    validate_refs(&evidence.policy_refs, "service runtime policy ref")?;
    validate_refs(&evidence.resource_refs, "service runtime resource ref")?;
    validate_refs(&evidence.effect_profile_refs, "service runtime effect profile ref")?;
    validate_refs(&evidence.source_gate_refs, "service runtime source gate ref")?;
    validate_optional_ref(evidence.scheduler_ref.as_deref(), "service runtime scheduler ref")?;
    validate_refs(&evidence.effect_log_refs, "service runtime effect log ref")
}

fn startup_admission_diagnostics(evidence: &EvidenceInput) -> Vec<String> {
    let mut diagnostics = Vec::new();
    if evidence.authority_refs.is_empty() {
        diagnostics.push("missing startup authority evidence".to_string());
    }
    if evidence.policy_refs.is_empty() {
        diagnostics.push("missing startup policy evidence".to_string());
    }
    if evidence.resource_refs.is_empty() {
        diagnostics.push("missing startup resource evidence".to_string());
    }
    if evidence.effect_profile_refs.is_empty() {
        diagnostics.push("missing startup effect-handle evidence".to_string());
    }
    if evidence.source_gate_refs.is_empty() {
        diagnostics.push("missing strict source-gate evidence".to_string());
    }
    diagnostics
}

fn evidence_value(evidence: &EvidenceInput) -> preserves::IOValue {
    record("evidence", vec![
        record("authority", vec![refs_sequence(&evidence.authority_refs)]),
        record("policy", vec![refs_sequence(&evidence.policy_refs)]),
        record("resource", vec![refs_sequence(&evidence.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&evidence.effect_profile_refs)]),
        record("source-gate", vec![refs_sequence(&evidence.source_gate_refs)]),
        record("scheduler", vec![optional_ref_value(evidence.scheduler_ref.as_deref())]),
        record("effect-log", vec![refs_sequence(&evidence.effect_log_refs)]),
    ])
}

fn parse_evidence(value: &Value<preserves::IOValue>) -> Result<EvidenceInput> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record("evidence", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <evidence ...>"))?;
    Ok(EvidenceInput {
        authority_refs: parse_ref_sequence(&fields[0], "authority")?,
        policy_refs: parse_ref_sequence(&fields[1], "policy")?,
        resource_refs: parse_ref_sequence(&fields[2], "resource")?,
        effect_profile_refs: parse_ref_sequence(&fields[3], "effect-profile")?,
        source_gate_refs: parse_ref_sequence(&fields[4], "source-gate")?,
        scheduler_ref: record_optional_ref(&fields[5], "scheduler")?,
        effect_log_refs: parse_ref_sequence(&fields[6], "effect-log")?,
    })
}

fn parse_iovalue_sequence(value: &Value<preserves::IOValue>, label: &str) -> Result<Vec<preserves::IOValue>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), label)?;
    Ok(values.iter().map(value_to_iovalue).collect())
}

fn parse_ref_sequence(value: &Value<preserves::IOValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), label)?;
    values.iter().map(|value| required_ref(value, label)).collect()
}

fn field_sequence(value: &Value<preserves::IOValue>, label: &str) -> Result<Vec<Value<preserves::IOValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn record_iovalue(value: &Value<preserves::IOValue>, label: &str) -> Result<preserves::IOValue> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    Ok(value_to_iovalue(&fields[0]))
}

fn record_optional_ref(value: &Value<preserves::IOValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    parse_optional_ref_value(&fields[0])
}

fn parse_optional_ref_value(value: &Value<preserves::IOValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional service runtime ref").map(Some);
    }
    required_ref(value, "optional service runtime ref").map(Some)
}

fn checks_value(names: &[&str]) -> preserves::IOValue {
    record("checks", vec![sequence(
        names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect(),
    )])
}

fn parse_checks(value: &Value<preserves::IOValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    ensure_count_at_most(values.len(), "service runtime checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected service runtime check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing passing {name} check")))
    }
}

fn require_schema(value: &Value<preserves::IOValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")))
    }
}

fn refs_sequence(values: &[String]) -> preserves::IOValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> preserves::IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), field)?;
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn validate_optional_ref(reference: Option<&str>, field: &str) -> Result<()> {
    if let Some(reference) = reference {
        require_ref(reference, field)
    } else {
        Ok(())
    }
}

fn required_ref(value: &Value<preserves::IOValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical blake3 content ref for {field}: {error}"))
    })
}

fn required_string(value: &Value<preserves::IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn ensure_count_at_most(actual: usize, label: &str) -> Result<()> {
    if actual <= MAX_RUNTIME_ITEMS {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {MAX_RUNTIME_ITEMS}")))
    }
}
