
fn validate_manifest_parsed(manifest: &ServiceManifest) -> Result<()> {
    validate_service_id(&manifest.service_id, "service manifest service id")?;
    require_ref(&manifest.owner_authority_ref, "service manifest owner authority ref")?;
    require_ref(&manifest.target_ref, "service manifest target ref")?;
    validate_service_ids(&manifest.dependencies, "service dependency")?;
    require_ref(&manifest.restart_policy_ref, "service restart policy ref")?;
    require_non_empty_refs(&manifest.policy_refs, "service policy refs")?;
    require_non_empty_refs(&manifest.resource_refs, "service resource refs")?;
    require_non_empty_refs(&manifest.effect_profile_refs, "service effect profile refs")
}

fn validate_demand_input(input: &ServiceDemandInput) -> Result<()> {
    validate_non_empty(&input.demand_id, "service demand id")?;
    validate_service_id(&input.service_id, "service demand service id")?;
    require_ref(&input.requester_ref, "service demand requester ref")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service demand manifest ref")?;
    validate_refs(&input.policy_refs, "service demand policy ref")
}

fn validate_status_input(input: &ServiceStatusInput) -> Result<()> {
    validate_service_id(&input.service_id, "service status service id")?;
    validate_state(&input.state)?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service status manifest ref")?;
    validate_refs(&input.demand_refs, "service status demand ref")?;
    validate_refs(&input.dependency_status_refs, "service dependency status ref")?;
    validate_refs(&input.readiness_assertion_refs, "service readiness assertion ref")?;
    validate_refs(&input.failure_refs, "service failure ref")?;
    validate_refs(&input.monitor_refs, "service monitor ref")?;
    validate_refs(&input.replay_refs, "service replay ref")
}

fn validate_supervisor_input(input: &ServiceSupervisorInput) -> Result<()> {
    validate_non_empty(&input.supervisor_id, "service supervisor id")?;
    validate_service_ids(&input.service_ids, "supervised service")?;
    validate_refs(&input.link_refs, "service link ref")?;
    validate_refs(&input.monitor_refs, "service monitor ref")?;
    validate_refs(&input.policy_refs, "service supervisor policy ref")
}

fn validate_link_input(input: &ServiceLinkInput) -> Result<()> {
    validate_non_empty(&input.supervisor_id, "service link supervisor id")?;
    validate_service_id(&input.parent_service_id, "service link parent service id")?;
    validate_service_id(&input.child_service_id, "service link child service id")?;
    validate_propagation(&input.propagation)?;
    validate_refs(&input.policy_refs, "service link policy ref")
}

fn validate_link_parsed(link: &ServiceLink) -> Result<()> {
    validate_non_empty(&link.supervisor_id, "service link supervisor id")?;
    validate_service_id(&link.parent_service_id, "service link parent service id")?;
    validate_service_id(&link.child_service_id, "service link child service id")?;
    validate_propagation(&link.propagation)?;
    validate_refs(&link.policy_refs, "service link policy ref")
}

fn validate_monitor_input(input: &ServiceMonitorInput) -> Result<()> {
    validate_non_empty(&input.monitor_id, "service monitor id")?;
    validate_service_id(&input.service_id, "service monitor service id")?;
    require_ref(&input.observer_ref, "service monitor observer ref")?;
    validate_notification_policy(&input.notification_policy)?;
    validate_refs(&input.policy_refs, "service monitor policy ref")
}

fn validate_monitor_parsed(monitor: &ServiceMonitor) -> Result<()> {
    validate_non_empty(&monitor.monitor_id, "service monitor id")?;
    validate_service_id(&monitor.service_id, "service monitor service id")?;
    require_ref(&monitor.observer_ref, "service monitor observer ref")?;
    validate_notification_policy(&monitor.notification_policy)?;
    validate_refs(&monitor.policy_refs, "service monitor policy ref")
}

fn validate_restart_policy_input(input: &ServiceRestartPolicyInput) -> Result<()> {
    validate_non_empty(&input.policy_id, "service restart policy id")?;
    if input.window_steps == 0 {
        return Err(MoltenError::invalid_harness("service restart policy window must be positive"));
    }
    validate_refs(&input.resource_refs, "service restart resource ref")?;
    require_non_empty_refs(&input.resource_refs, "service restart resource refs")
}

fn validate_restart_policy_parsed(policy: &ServiceRestartPolicy) -> Result<()> {
    validate_non_empty(&policy.policy_id, "service restart policy id")?;
    if policy.window_steps == 0 {
        return Err(MoltenError::invalid_harness("service restart policy window must be positive"));
    }
    require_non_empty_refs(&policy.resource_refs, "service restart resource refs")
}

fn validate_restart_decision_input(input: &ServiceRestartDecisionInput) -> Result<()> {
    validate_decision(&input.decision)?;
    validate_service_id(&input.service_id, "service restart decision service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service restart decision manifest ref")?;
    require_ref(&input.policy_ref, "service restart decision policy ref")?;
    validate_refs(&input.prior_lifecycle_refs, "service restart decision lifecycle ref")?;
    validate_refs(&input.authority_refs, "service restart decision authority ref")?;
    validate_refs(&input.resource_refs, "service restart decision resource ref")?;
    validate_diagnostics(&input.diagnostics)
}

fn validate_restart_decision_parsed(decision: &ServiceRestartDecision) -> Result<()> {
    validate_decision(&decision.decision)?;
    validate_service_id(&decision.service_id, "service restart decision service id")?;
    require_ref(&decision.policy_ref, "service restart decision policy ref")?;
    validate_diagnostics(&decision.diagnostics)
}

fn validate_lifecycle_input(input: &ServiceLifecycleReceiptInput) -> Result<()> {
    validate_operation(&input.operation)?;
    validate_decision(&input.decision)?;
    validate_service_id(&input.service_id, "service lifecycle service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service lifecycle manifest ref")?;
    validate_optional_ref(input.status_ref.as_deref(), "service lifecycle status ref")?;
    validate_refs(&input.authority_refs, "service lifecycle authority ref")?;
    validate_refs(&input.resource_refs, "service lifecycle resource ref")?;
    validate_refs(&input.effect_profile_refs, "service lifecycle effect profile ref")?;
    validate_refs(&input.supervision_refs, "service lifecycle supervision ref")?;
    validate_diagnostics(&input.diagnostics)
}

fn validate_lifecycle_parsed(receipt: &ServiceLifecycleReceipt) -> Result<()> {
    validate_operation(&receipt.operation)?;
    validate_decision(&receipt.decision)?;
    validate_service_id(&receipt.service_id, "service lifecycle service id")?;
    validate_refs(&receipt.supervision_refs, "service lifecycle supervision ref")?;
    validate_diagnostics(&receipt.diagnostics)
}

fn validate_cleanup_input(input: &ServiceCleanupReceiptInput) -> Result<()> {
    validate_decision(&input.decision)?;
    validate_service_id(&input.service_id, "service cleanup service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service cleanup manifest ref")?;
    validate_refs(&input.authority_refs, "service cleanup authority ref")?;
    validate_refs(&input.owned_assertion_refs, "service cleanup owned assertion ref")?;
    validate_refs(&input.observer_refs, "service cleanup observer ref")?;
    validate_refs(&input.live_ref_refs, "service cleanup live ref")?;
    validate_refs(&input.exposed_ref_refs, "service cleanup exposed ref")?;
    validate_refs(&input.pending_effect_refs, "service cleanup pending effect ref")?;
    validate_refs(&input.retraction_refs, "service cleanup retraction ref")?;
    validate_refs(&input.revocation_refs, "service cleanup revocation ref")?;
    validate_refs(&input.retention_refs, "service cleanup retention ref")?;
    validate_diagnostics(&input.diagnostics)
}

fn validate_cleanup_parsed(receipt: &ServiceCleanupReceipt) -> Result<()> {
    validate_decision(&receipt.decision)?;
    validate_service_id(&receipt.service_id, "service cleanup service id")?;
    validate_refs(&receipt.retraction_refs, "service cleanup retraction ref")?;
    validate_refs(&receipt.revocation_refs, "service cleanup revocation ref")?;
    validate_refs(&receipt.retention_refs, "service cleanup retention ref")?;
    validate_diagnostics(&receipt.diagnostics)
}

fn validate_service_ids(ids: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(ids.len(), MAX_SERVICE_IDS, field)?;
    for service_id in ids {
        validate_service_id(service_id, field)?;
    }
    Ok(())
}

fn validate_service_id(value: &str, field: &str) -> Result<()> {
    validate_non_empty(value, field)?;
    if value.starts_with("svc:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected svc: service id for {field}, got {value}")))
    }
}

fn validate_state(state: &str) -> Result<()> {
    match state {
        "demanded" | "waiting" | "starting" | "ready" | "degraded" | "failed" | "stopped" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service state {state}"))),
    }
}

fn validate_operation(operation: &str) -> Result<()> {
    match operation {
        "declare" | "demand" | "status" | "start" | "ready" | "fail" | "restart" | "stop" | "cleanup"
        | "dependency-wait" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service lifecycle operation {operation}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "diagnostic" | "backoff" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service decision {decision}"))),
    }
}

fn validate_propagation(propagation: &str) -> Result<()> {
    match propagation {
        "restart" | "stop" | "notify" | "ignore" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service failure propagation {propagation}"))),
    }
}

fn validate_notification_policy(policy: &str) -> Result<()> {
    match policy {
        "failure" | "status" | "all" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service monitor notification policy {policy}"))),
    }
}

fn validate_diagnostics(diagnostics: &[String]) -> Result<()> {
    ensure_count_at_most(diagnostics.len(), MAX_SERVICE_DIAGNOSTICS, "service diagnostics")?;
    for diagnostic in diagnostics {
        validate_non_empty(diagnostic, "service diagnostic")?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_non_empty_refs(refs: &[String], field: &str) -> Result<()> {
    if refs.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        validate_refs(refs, field)
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_SERVICE_REFS, field)?;
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

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
    }
}

fn service_id_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn refs_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(names: &[&str]) -> IoValue {
    record("checks", vec![sequence(
        names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect(),
    )])
}

fn parse_service_id_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_SERVICE_IDS, label)?;
    values
        .iter()
        .map(|value| {
            let service_id = required_string(value, label)?;
            validate_service_id(&service_id, label)?;
            Ok(service_id)
        })
        .collect()
}
