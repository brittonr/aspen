
pub fn connectivity_probe_receipt(input: &ConnectivityProbeInput) -> crate::error::Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_text(&input.source_node, "source node", &mut diagnostics)?;
    validate_text(&input.target_node, "target node", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.expected_endpoint_ref), "expected endpoint", &mut diagnostics)?;
    if let Some(observed) = &input.observed_endpoint_ref {
        collect_ref_diagnostics(std::slice::from_ref(observed), "observed endpoint", &mut diagnostics)?;
        if observed != &input.expected_endpoint_ref {
            push_diagnostic(&mut diagnostics, "observed endpoint identity does not match expected endpoint")?;
        }
    } else {
        push_diagnostic(&mut diagnostics, "no observed endpoint identity")?;
    }
    collect_ref_diagnostics(&input.authority_refs, "authority", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
    collect_ref_diagnostics(&input.evidence_refs, "evidence", &mut diagnostics)?;
    let path_status = if input.direct_path_status == "pass" {
        "direct"
    } else if input.relay_path_status == "pass" {
        push_diagnostic(&mut diagnostics, "relay-only diagnostic path; direct path did not pass")?;
        "relay-only"
    } else if input.timeout_ms.is_some() {
        push_diagnostic(&mut diagnostics, "connectivity probe timed out")?;
        "timeout"
    } else {
        push_diagnostic(&mut diagnostics, "connectivity probe did not find a passing path")?;
        "deny"
    };
    let decision = if diagnostics.iter().any(|diagnostic| {
        diagnostic.contains("identity") || diagnostic.contains("timed out") || diagnostic.contains("did not find")
    }) {
        "deny"
    } else if path_status == "relay-only" {
        "degraded"
    } else {
        "pass"
    }
    .to_string();
    let receipt_value = crate::preserves_rail::record("network-connectivity-probe-receipt-v1", vec![
        crate::preserves_rail::string(NETWORK_CONNECTIVITY_PROBE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("source", vec![crate::preserves_rail::string(&input.source_node)]),
        crate::preserves_rail::record("target", vec![crate::preserves_rail::string(&input.target_node)]),
        crate::preserves_rail::record("path", vec![crate::preserves_rail::string(path_status)]),
        crate::preserves_rail::record("expected-endpoint", vec![crate::preserves_rail::string(
            &input.expected_endpoint_ref,
        )]),
        crate::preserves_rail::record("observed-endpoint", vec![optional_string_value(
            input.observed_endpoint_ref.as_deref(),
        )]),
        crate::preserves_rail::record("authority", vec![refs_value(&input.authority_refs)?]),
        crate::preserves_rail::record("policy", vec![refs_value(&input.policy_refs)?]),
        crate::preserves_rail::record("resource", vec![refs_value(&input.resource_refs)?]),
        crate::preserves_rail::record("evidence", vec![refs_value(&input.evidence_refs)?]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("connectivity-diagnostic-only", "pass"),
            ("no-state-mutation", pass_fail(decision != "pass" || !input.evidence_refs.is_empty())),
            ("transport-does-not-grant-authority", "pass"),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

pub fn port_mapping_receipt(input: &PortMappingInput) -> crate::error::Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_status(&input.mode, &["probe", "mutate"], "port mapping mode")?;
    validate_bounded_value_count(input.available_protocols.len(), MAX_NETWORK_OBSERVATIONS, "available protocol")?;
    let is_protocol_available = input.available_protocols.iter().any(|protocol| protocol == &input.protocol);
    if !is_protocol_available {
        push_diagnostic(&mut diagnostics, "requested port mapping protocol unavailable")?;
    }
    if input.mode == "mutate" {
        collect_required_optional_ref(input.requester_ref.as_deref(), "requester", &mut diagnostics)?;
        collect_required_optional_ref(input.identity_ref.as_deref(), "node identity", &mut diagnostics)?;
        collect_ref_diagnostics(&input.authority_refs, "authority", &mut diagnostics)?;
        collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
        collect_ref_diagnostics(&input.resource_refs, "resource", &mut diagnostics)?;
        collect_ref_diagnostics(&input.operator_evidence_refs, "operator evidence", &mut diagnostics)?;
        validate_port(input.external_port, "external port", &mut diagnostics)?;
        validate_port(input.internal_port, "internal port", &mut diagnostics)?;
        match input.duration_seconds {
            Some(duration) if duration <= MAX_PORT_DURATION_SECONDS => {}
            Some(_) => push_diagnostic(&mut diagnostics, "port mapping duration exceeds bound")?,
            None => push_diagnostic(&mut diagnostics, "port mapping mutation requires duration")?,
        }
    }
    let decision = if diagnostics.is_empty() {
        "pass"
    } else if input.mode == "probe" {
        "degraded"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = crate::preserves_rail::record("network-port-mapping-receipt-v1", vec![
        crate::preserves_rail::string(NETWORK_PORT_MAPPING_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(&input.mode)]),
        crate::preserves_rail::record("protocol", vec![crate::preserves_rail::string(&input.protocol)]),
        crate::preserves_rail::record("requester", vec![optional_string_value(input.requester_ref.as_deref())]),
        crate::preserves_rail::record("node", vec![optional_string_value(input.identity_ref.as_deref())]),
        crate::preserves_rail::record("external-port", vec![optional_u64_value(input.external_port)]),
        crate::preserves_rail::record("internal-port", vec![optional_u64_value(input.internal_port)]),
        crate::preserves_rail::record("duration-seconds", vec![optional_u64_value(input.duration_seconds)]),
        crate::preserves_rail::record("available-protocols", vec![crate::preserves_rail::sequence(
            input.available_protocols.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("authority", vec![refs_value(&input.authority_refs)?]),
        crate::preserves_rail::record("policy", vec![refs_value(&input.policy_refs)?]),
        crate::preserves_rail::record("resource", vec![refs_value(&input.resource_refs)?]),
        crate::preserves_rail::record("operator-evidence", vec![refs_value(&input.operator_evidence_refs)?]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("probe-does-not-mutate", pass_fail(input.mode == "probe")),
            ("mutation-deny-by-default", pass_fail(input.mode != "mutate" || decision == "pass")),
            ("authority-policy-resource-explicit", pass_fail(input.mode != "mutate" || diagnostics.is_empty())),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

pub fn watcher_snapshot_value(input: &NetworkWatcherInput) -> crate::error::Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_text(&input.node, "watcher node", &mut diagnostics)?;
    validate_text(&input.interface_state, "interface state", &mut diagnostics)?;
    validate_text(&input.address_state, "address state", &mut diagnostics)?;
    validate_text(&input.default_route, "default route", &mut diagnostics)?;
    validate_text(&input.relay_state, "relay state", &mut diagnostics)?;
    validate_text(&input.endpoint_state, "endpoint state", &mut diagnostics)?;
    if input.retained_event_count > input.observed_event_count {
        push_diagnostic(&mut diagnostics, "retained watcher event count exceeds observed event count")?;
    }
    let retained_event_count = usize::try_from(input.retained_event_count).map_err(|error| {
        crate::error::MoltenError::invalid_harness(format!("retained watcher event count unsupported: {error}"))
    })?;
    if retained_event_count > MAX_WATCHER_ITEMS {
        push_diagnostic(&mut diagnostics, "retained watcher events exceed latest-state bound")?;
    }
    collect_ref_diagnostics(&input.evidence_refs, "watcher evidence", &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "degraded" }.to_string();
    let receipt_value = crate::preserves_rail::record("network-watcher-snapshot-v1", vec![
        crate::preserves_rail::string(NETWORK_WATCHER_SNAPSHOT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.node)]),
        crate::preserves_rail::record("interface", vec![crate::preserves_rail::string(&input.interface_state)]),
        crate::preserves_rail::record("address", vec![crate::preserves_rail::string(&input.address_state)]),
        crate::preserves_rail::record("default-route", vec![crate::preserves_rail::string(&input.default_route)]),
        crate::preserves_rail::record("relay", vec![crate::preserves_rail::string(&input.relay_state)]),
        crate::preserves_rail::record("endpoint", vec![crate::preserves_rail::string(&input.endpoint_state)]),
        crate::preserves_rail::record("observed-events", vec![crate::preserves_rail::u64_value(
            input.observed_event_count,
        )]),
        crate::preserves_rail::record("retained-events", vec![crate::preserves_rail::u64_value(
            input.retained_event_count,
        )]),
        crate::preserves_rail::record("evidence", vec![refs_value(&input.evidence_refs)?]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("latest-state-only", "pass"),
            ("bounded-event-buffer", pass_fail(retained_event_count <= MAX_WATCHER_ITEMS)),
            ("watcher-diagnostic-only", "pass"),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}

pub fn metrics_snapshot(input: &MetricsSnapshotInput) -> crate::error::Result<MetricsSnapshotDecision> {
    let mut diagnostics = Vec::new();
    validate_text(&input.node, "metrics node", &mut diagnostics)?;
    collect_ref_diagnostics(std::slice::from_ref(&input.scrape_ref), "scrape", &mut diagnostics)?;
    collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
    collect_ref_diagnostics(&input.redaction_refs, "redaction", &mut diagnostics)?;
    validate_bounded_value_count(input.samples.len(), MAX_METRIC_SAMPLES, "metric sample")?;
    let openmetrics = render_openmetrics(input, &mut diagnostics)?;
    if openmetrics.len() > MAX_OPENMETRICS_BYTES {
        push_diagnostic(&mut diagnostics, "OpenMetrics snapshot exceeds byte bound")?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" }.to_string();
    let metric_refs = input
        .samples
        .iter()
        .map(|sample| {
            crate::preserves_rail::record("metric", vec![
                crate::preserves_rail::string(&sample.name),
                crate::preserves_rail::string(&sample.kind),
                crate::preserves_rail::u64_value(sample.value),
            ])
        })
        .collect();
    let receipt_value = crate::preserves_rail::record("metrics-snapshot-receipt-v1", vec![
        crate::preserves_rail::string(METRICS_SNAPSHOT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.node)]),
        crate::preserves_rail::record("scrape", vec![crate::preserves_rail::string(&input.scrape_ref)]),
        crate::preserves_rail::record("policy", vec![refs_value(&input.policy_refs)?]),
        crate::preserves_rail::record("redaction", vec![refs_value(&input.redaction_refs)?]),
        crate::preserves_rail::record("metrics", vec![crate::preserves_rail::sequence(metric_refs)]),
        crate::preserves_rail::record("openmetrics-ref", vec![crate::preserves_rail::string(
            crate::preserves_rail::content_ref_from_bytes(openmetrics.as_bytes()),
        )]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("labels-bounded", pass_fail(diagnostics.is_empty())),
            ("labels-redacted", pass_fail(diagnostics.is_empty())),
            ("metrics-do-not-grant-admission", "pass"),
        ]),
    ]);
    Ok(MetricsSnapshotDecision {
        decision,
        diagnostics,
        openmetrics,
        receipt_value,
    })
}

pub fn external_diagnostics_bridge_receipt(
    input: &ExternalDiagnosticsBridgeInput,
) -> crate::error::Result<DiagnosticDecision> {
    let mut diagnostics = Vec::new();
    validate_status(&input.mode, &["push", "remote-request"], "external diagnostics mode")?;
    if !input.enabled {
        push_diagnostic(&mut diagnostics, "external diagnostics bridge disabled by default")?;
    } else {
        collect_required_optional_ref(input.target_service_ref.as_deref(), "target service", &mut diagnostics)?;
        collect_required_optional_ref(
            input.api_secret_provenance_ref.as_deref(),
            "api secret provenance",
            &mut diagnostics,
        )?;
        collect_required_optional_ref(input.expiry_ref.as_deref(), "bridge expiry", &mut diagnostics)?;
        collect_ref_diagnostics(&input.capability_refs, "capability", &mut diagnostics)?;
        collect_ref_diagnostics(&input.policy_refs, "policy", &mut diagnostics)?;
        collect_ref_diagnostics(&input.redaction_policy_refs, "redaction policy", &mut diagnostics)?;
        collect_ref_diagnostics(&input.operator_evidence_refs, "operator evidence", &mut diagnostics)?;
    }
    let decision = if input.enabled && diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    }
    .to_string();
    let receipt_value = crate::preserves_rail::record("external-diagnostics-bridge-receipt-v1", vec![
        crate::preserves_rail::string(EXTERNAL_DIAGNOSTICS_BRIDGE_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(&decision)]),
        crate::preserves_rail::record("mode", vec![crate::preserves_rail::string(&input.mode)]),
        crate::preserves_rail::record("enabled", vec![crate::preserves_rail::string(if input.enabled {
            "true"
        } else {
            "false"
        })]),
        crate::preserves_rail::record("target-service", vec![optional_string_value(
            input.target_service_ref.as_deref(),
        )]),
        crate::preserves_rail::record("capability", vec![refs_value(&input.capability_refs)?]),
        crate::preserves_rail::record("policy", vec![refs_value(&input.policy_refs)?]),
        crate::preserves_rail::record("redaction-policy", vec![refs_value(&input.redaction_policy_refs)?]),
        crate::preserves_rail::record("api-secret-provenance", vec![optional_string_value(
            input.api_secret_provenance_ref.as_deref(),
        )]),
        crate::preserves_rail::record("operator-evidence", vec![refs_value(&input.operator_evidence_refs)?]),
        crate::preserves_rail::record("expiry", vec![optional_string_value(input.expiry_ref.as_deref())]),
        crate::preserves_rail::record("diagnostics", vec![strings_value(&diagnostics)?]),
        checks_value(&[
            ("disabled-by-default", pass_fail(!input.enabled || decision == "pass")),
            ("secret-redacted", "pass"),
            ("remote-requests-still-router-admitted", "pass"),
        ]),
    ]);
    Ok(DiagnosticDecision {
        decision,
        diagnostics,
        receipt_value,
    })
}
