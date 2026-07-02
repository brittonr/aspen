
pub fn service_restart_decision_value(input: &ServiceRestartDecisionInput) -> Result<IoValue> {
    validate_restart_decision_input(input)?;
    Ok(record("service-restart-decision-v1", vec![
        string(SERVICE_RESTART_DECISION_SCHEMA),
        record("decision", vec![string(&input.decision)]),
        record("service-id", vec![string(&input.service_id)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("policy", vec![string(&input.policy_ref)]),
        record("attempt", vec![u64_value(input.attempt)]),
        record("max-attempts", vec![u64_value(input.max_attempts)]),
        record("window-step", vec![u64_value(input.window_step)]),
        record("backoff-slot", vec![u64_value(input.backoff_slot)]),
        record("prior-lifecycle", vec![refs_sequence(&input.prior_lifecycle_refs)]),
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
        checks_value(&["bounded-restart", "logical-window", "replay-identity-bound"]),
    ]))
}

pub fn parse_service_restart_decision(value: &IoValue) -> Result<ServiceRestartDecision> {
    let fields = value
        .collect_simple_record("service-restart-decision-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-restart-decision-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_RESTART_DECISION_SCHEMA, "service restart decision schema")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "bounded-restart", "service restart decision")?;
    let decision = ServiceRestartDecision {
        decision_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        service_id: record_string(&fields[2], "service-id")?,
        manifest_ref: record_optional_ref(&fields[3], "manifest")?,
        policy_ref: record_ref(&fields[4], "policy")?,
        attempt: record_u64(&fields[5], "attempt")?,
        max_attempts: record_u64(&fields[6], "max-attempts")?,
        window_step: record_u64(&fields[7], "window-step")?,
        backoff_slot: record_u64(&fields[8], "backoff-slot")?,
        prior_lifecycle_refs: parse_ref_sequence(&fields[9], "prior-lifecycle")?,
        authority_refs: parse_ref_sequence(&fields[10], "authority")?,
        resource_refs: parse_ref_sequence(&fields[11], "resource")?,
        diagnostics: parse_string_sequence(&fields[12], "diagnostics")?,
        value: value.clone(),
    };
    validate_restart_decision_parsed(&decision)?;
    Ok(decision)
}

pub fn service_lifecycle_receipt_value(input: &ServiceLifecycleReceiptInput) -> Result<IoValue> {
    validate_lifecycle_input(input)?;
    Ok(record("service-lifecycle-receipt-v1", vec![
        string(SERVICE_LIFECYCLE_RECEIPT_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("decision", vec![string(&input.decision)]),
        record("service-id", vec![string(&input.service_id)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("status", vec![optional_ref_value(input.status_ref.as_deref())]),
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&input.effect_profile_refs)]),
        record("supervision", vec![refs_sequence(&input.supervision_refs)]),
        record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
        checks_value(&["canonical-receipt", "decision-before-side-effects", "text-not-evidence"]),
    ]))
}

pub fn parse_service_lifecycle_receipt(value: &IoValue) -> Result<ServiceLifecycleReceipt> {
    let fields = value
        .collect_simple_record("service-lifecycle-receipt-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-lifecycle-receipt-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_LIFECYCLE_RECEIPT_SCHEMA, "service lifecycle receipt schema")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-receipt", "service lifecycle receipt")?;
    let receipt = ServiceLifecycleReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        service_id: record_string(&fields[3], "service-id")?,
        manifest_ref: record_optional_ref(&fields[4], "manifest")?,
        status_ref: record_optional_ref(&fields[5], "status")?,
        authority_refs: parse_ref_sequence(&fields[6], "authority")?,
        resource_refs: parse_ref_sequence(&fields[7], "resource")?,
        effect_profile_refs: parse_ref_sequence(&fields[8], "effect-profile")?,
        supervision_refs: parse_ref_sequence(&fields[9], "supervision")?,
        diagnostics: parse_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    };
    validate_lifecycle_parsed(&receipt)?;
    Ok(receipt)
}

pub fn service_cleanup_receipt_value(input: &ServiceCleanupReceiptInput) -> Result<IoValue> {
    validate_cleanup_input(input)?;
    Ok(record("service-cleanup-receipt-v1", vec![
        string(SERVICE_CLEANUP_RECEIPT_SCHEMA),
        record("decision", vec![string(&input.decision)]),
        record("service-id", vec![string(&input.service_id)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("owned-assertions", vec![refs_sequence(&input.owned_assertion_refs)]),
        record("observers", vec![refs_sequence(&input.observer_refs)]),
        record("live-refs", vec![refs_sequence(&input.live_ref_refs)]),
        record("exposed-refs", vec![refs_sequence(&input.exposed_ref_refs)]),
        record("pending-effects", vec![refs_sequence(&input.pending_effect_refs)]),
        record("retractions", vec![refs_sequence(&input.retraction_refs)]),
        record("revocations", vec![refs_sequence(&input.revocation_refs)]),
        record("retention", vec![refs_sequence(&input.retention_refs)]),
        record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
        checks_value(&["canonical-cleanup", "owned-state-only", "retention-still-gates"]),
    ]))
}

pub fn parse_service_cleanup_receipt(value: &IoValue) -> Result<ServiceCleanupReceipt> {
    let fields = value
        .collect_simple_record("service-cleanup-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-cleanup-receipt-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_CLEANUP_RECEIPT_SCHEMA, "service cleanup receipt schema")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "owned-state-only", "service cleanup receipt")?;
    let receipt = ServiceCleanupReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        service_id: record_string(&fields[2], "service-id")?,
        manifest_ref: record_optional_ref(&fields[3], "manifest")?,
        authority_refs: parse_ref_sequence(&fields[4], "authority")?,
        owned_assertion_refs: parse_ref_sequence(&fields[5], "owned-assertions")?,
        observer_refs: parse_ref_sequence(&fields[6], "observers")?,
        live_ref_refs: parse_ref_sequence(&fields[7], "live-refs")?,
        exposed_ref_refs: parse_ref_sequence(&fields[8], "exposed-refs")?,
        pending_effect_refs: parse_ref_sequence(&fields[9], "pending-effects")?,
        retraction_refs: parse_ref_sequence(&fields[10], "retractions")?,
        revocation_refs: parse_ref_sequence(&fields[11], "revocations")?,
        retention_refs: parse_ref_sequence(&fields[12], "retention")?,
        diagnostics: parse_string_sequence(&fields[13], "diagnostics")?,
        value: value.clone(),
    };
    validate_cleanup_parsed(&receipt)?;
    Ok(receipt)
}

pub fn parse_service_record(value: &IoValue) -> Result<ServiceRecord> {
    if value.collect_simple_record("service-manifest-v1", Some(11)).is_some() {
        return parse_service_manifest(value).map(ServiceRecord::Manifest);
    }
    if value.collect_simple_record("service-demand-v1", Some(7)).is_some() {
        return parse_service_demand(value).map(ServiceRecord::Demand);
    }
    if value.collect_simple_record("service-status-v1", Some(12)).is_some() {
        return parse_service_status(value).map(ServiceRecord::Status);
    }
    if value.collect_simple_record("service-supervisor-v1", Some(7)).is_some() {
        return parse_service_supervisor(value).map(ServiceRecord::Supervisor);
    }
    if value.collect_simple_record("service-link-v1", Some(7)).is_some() {
        return parse_service_link(value).map(ServiceRecord::Link);
    }
    if value.collect_simple_record("service-monitor-v1", Some(7)).is_some() {
        return parse_service_monitor(value).map(ServiceRecord::Monitor);
    }
    if value.collect_simple_record("service-restart-policy-v1", Some(7)).is_some() {
        return parse_service_restart_policy(value).map(ServiceRecord::RestartPolicy);
    }
    if value.collect_simple_record("service-restart-decision-v1", Some(14)).is_some() {
        return parse_service_restart_decision(value).map(ServiceRecord::RestartDecision);
    }
    if value.collect_simple_record("service-lifecycle-receipt-v1", Some(12)).is_some() {
        return parse_service_lifecycle_receipt(value).map(ServiceRecord::LifecycleReceipt);
    }
    if value.collect_simple_record("service-cleanup-receipt-v1", Some(15)).is_some() {
        return parse_service_cleanup_receipt(value).map(ServiceRecord::CleanupReceipt);
    }
    Err(MoltenError::invalid_harness("unknown service record schema"))
}

pub fn service_summary(value: &IoValue) -> Result<String> {
    let has_sensitive_marker = is_sensitive_marker_present(value)?;
    let redaction = if has_sensitive_marker { " redacted=true" } else { "" };
    Ok(summary_text(parse_service_record(value)?, redaction))
}

fn summary_text(record: ServiceRecord, redaction: &str) -> String {
    match record {
        ServiceRecord::Manifest(manifest) => manifest_text(&manifest, redaction),
        ServiceRecord::Demand(demand) => demand_text(&demand, redaction),
        ServiceRecord::Status(status) => status_text(&status, redaction),
        ServiceRecord::Supervisor(supervisor) => supervisor_text(&supervisor, redaction),
        ServiceRecord::Link(link) => link_text(&link, redaction),
        ServiceRecord::Monitor(monitor) => monitor_text(&monitor, redaction),
        ServiceRecord::RestartPolicy(policy) => restart_policy_text(&policy, redaction),
        ServiceRecord::RestartDecision(decision) => restart_decision_text(&decision, redaction),
        ServiceRecord::LifecycleReceipt(receipt) => lifecycle_text(&receipt, redaction),
        ServiceRecord::CleanupReceipt(receipt) => cleanup_text(&receipt, redaction),
    }
}

fn manifest_text(manifest: &ServiceManifest, redaction: &str) -> String {
    format!(
        "service manifest id={} target={} deps={} ref={}{}",
        manifest.service_id,
        manifest.target_ref,
        manifest.dependencies.len(),
        manifest.manifest_ref,
        redaction
    )
}

fn demand_text(demand: &ServiceDemand, redaction: &str) -> String {
    format!(
        "service demand id={} service={} requester={} ref={}{}",
        demand.demand_id, demand.service_id, demand.requester_ref, demand.demand_ref, redaction
    )
}

fn status_text(status: &ServiceStatus, redaction: &str) -> String {
    format!(
        "service status service={} state={} readiness={} ref={}{}",
        status.service_id,
        status.state,
        status.readiness_assertion_refs.len(),
        status.status_ref,
        redaction
    )
}

fn supervisor_text(supervisor: &ServiceSupervisor, redaction: &str) -> String {
    format!(
        "service supervisor id={} services={} ref={}{}",
        supervisor.supervisor_id,
        supervisor.service_ids.len(),
        supervisor.supervisor_ref,
        redaction
    )
}

fn link_text(link: &ServiceLink, redaction: &str) -> String {
    format!(
        "service link supervisor={} parent={} child={} propagation={} ref={}{}",
        link.supervisor_id, link.parent_service_id, link.child_service_id, link.propagation, link.link_ref, redaction
    )
}

fn monitor_text(monitor: &ServiceMonitor, redaction: &str) -> String {
    format!(
        "service monitor id={} service={} observer={} ref={}{}",
        monitor.monitor_id, monitor.service_id, monitor.observer_ref, monitor.monitor_ref, redaction
    )
}

fn restart_policy_text(policy: &ServiceRestartPolicy, redaction: &str) -> String {
    format!(
        "service restart-policy id={} max-attempts={} ref={}{}",
        policy.policy_id, policy.max_attempts, policy.policy_ref, redaction
    )
}

fn restart_decision_text(decision: &ServiceRestartDecision, redaction: &str) -> String {
    format!(
        "service restart decision={} service={} attempt={}/{} ref={}{}",
        decision.decision,
        decision.service_id,
        decision.attempt,
        decision.max_attempts,
        decision.decision_ref,
        redaction
    )
}

fn lifecycle_text(receipt: &ServiceLifecycleReceipt, redaction: &str) -> String {
    format!(
        "service lifecycle operation={} decision={} service={} ref={}{}",
        receipt.operation, receipt.decision, receipt.service_id, receipt.receipt_ref, redaction
    )
}

fn cleanup_text(receipt: &ServiceCleanupReceipt, redaction: &str) -> String {
    format!(
        "service cleanup decision={} service={} retractions={} ref={}{}",
        receipt.decision,
        receipt.service_id,
        receipt.retraction_refs.len(),
        receipt.receipt_ref,
        redaction
    )
}

fn validate_manifest_input(input: &ServiceManifestInput) -> Result<()> {
    validate_service_id(&input.service_id, "service manifest service id")?;
    require_ref(&input.owner_authority_ref, "service manifest owner authority ref")?;
    require_ref(&input.target_ref, "service manifest target ref")?;
    validate_service_ids(&input.dependencies, "service dependency")?;
    validate_refs(&input.provided_assertion_refs, "provided assertion ref")?;
    require_ref(&input.restart_policy_ref, "service restart policy ref")?;
    validate_refs(&input.policy_refs, "service policy ref")?;
    validate_refs(&input.resource_refs, "service resource ref")?;
    validate_refs(&input.effect_profile_refs, "service effect profile ref")?;
    require_non_empty_refs(&input.policy_refs, "service policy refs")?;
    require_non_empty_refs(&input.resource_refs, "service resource refs")?;
    require_non_empty_refs(&input.effect_profile_refs, "service effect profile refs")
}
