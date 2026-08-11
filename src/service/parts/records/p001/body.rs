
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCleanupReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub owned_assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub live_ref_refs: Vec<String>,
    pub exposed_ref_refs: Vec<String>,
    pub pending_effect_refs: Vec<String>,
    pub retraction_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub retention_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRecord {
    Manifest(ServiceManifest),
    Demand(ServiceDemand),
    Status(ServiceStatus),
    Supervisor(ServiceSupervisor),
    Link(ServiceLink),
    Monitor(ServiceMonitor),
    RestartPolicy(ServiceRestartPolicy),
    RestartDecision(ServiceRestartDecision),
    LifecycleReceipt(ServiceLifecycleReceipt),
    CleanupReceipt(ServiceCleanupReceipt),
}

// r[impl molten.sam_service_records_ledger.spec.explicit_boundaries]
pub fn service_manifest_value(input: &ServiceManifestInput) -> Result<IoValue> {
    validate_manifest_input(input)?;
    Ok(record("service-manifest-v1", vec![
        string(SERVICE_MANIFEST_SCHEMA),
        record("service-id", vec![string(&input.service_id)]),
        record("owner", vec![string(&input.owner_authority_ref)]),
        record("target", vec![string(&input.target_ref)]),
        record("requires", vec![service_id_sequence(&input.dependencies)]),
        record("provides", vec![refs_sequence(&input.provided_assertion_refs)]),
        record("restart-policy", vec![string(&input.restart_policy_ref)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&input.effect_profile_refs)]),
        checks_value(&[
            "schema-known",
            "explicit-authority",
            "target-ref-bound",
            "policy-resource-effect-declared",
            "canonical-service-record",
        ]),
    ]))
}

pub fn parse_service_manifest(value: &IoValue) -> Result<ServiceManifest> {
    let fields = value
        .collect_simple_record("service-manifest-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-manifest-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_MANIFEST_SCHEMA, "service manifest schema")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "explicit-authority", "service manifest")?;
    require_check(&checks, "policy-resource-effect-declared", "service manifest")?;
    let manifest = ServiceManifest {
        manifest_ref: canonical_hash(value)?,
        service_id: record_string(&fields[1], "service-id")?,
        owner_authority_ref: record_ref(&fields[2], "owner")?,
        target_ref: record_ref(&fields[3], "target")?,
        dependencies: parse_service_id_sequence(&fields[4], "requires")?,
        provided_assertion_refs: parse_ref_sequence(&fields[5], "provides")?,
        restart_policy_ref: record_ref(&fields[6], "restart-policy")?,
        policy_refs: parse_ref_sequence(&fields[7], "policy")?,
        resource_refs: parse_ref_sequence(&fields[8], "resource")?,
        effect_profile_refs: parse_ref_sequence(&fields[9], "effect-profile")?,
        value: value.clone(),
    };
    validate_manifest_parsed(&manifest)?;
    Ok(manifest)
}

pub fn service_demand_value(input: &ServiceDemandInput) -> Result<IoValue> {
    validate_demand_input(input)?;
    Ok(record("service-demand-v1", vec![
        string(SERVICE_DEMAND_SCHEMA),
        record("demand-id", vec![string(&input.demand_id)]),
        record("service-id", vec![string(&input.service_id)]),
        record("requester", vec![string(&input.requester_ref)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["canonical-demand", "explicit-requester", "startup-admission-required"]),
    ]))
}

pub fn parse_service_demand(value: &IoValue) -> Result<ServiceDemand> {
    let fields = value
        .collect_simple_record("service-demand-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-demand-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_DEMAND_SCHEMA, "service demand schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "startup-admission-required", "service demand")?;
    let demand = ServiceDemand {
        demand_ref: canonical_hash(value)?,
        demand_id: record_string(&fields[1], "demand-id")?,
        service_id: record_string(&fields[2], "service-id")?,
        requester_ref: record_ref(&fields[3], "requester")?,
        manifest_ref: record_optional_ref(&fields[4], "manifest")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_service_id(&demand.service_id, "service demand service id")?;
    validate_non_empty(&demand.demand_id, "service demand id")?;
    Ok(demand)
}

pub fn service_status_value(input: &ServiceStatusInput) -> Result<IoValue> {
    validate_status_input(input)?;
    Ok(record("service-status-v1", vec![
        string(SERVICE_STATUS_SCHEMA),
        record("service-id", vec![string(&input.service_id)]),
        record("state", vec![string(&input.state)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("demands", vec![refs_sequence(&input.demand_refs)]),
        record("dependencies", vec![refs_sequence(&input.dependency_status_refs)]),
        record("readiness", vec![refs_sequence(&input.readiness_assertion_refs)]),
        record("failures", vec![refs_sequence(&input.failure_refs)]),
        record("restart-count", vec![u64_value(input.restart_count)]),
        record("monitors", vec![refs_sequence(&input.monitor_refs)]),
        record("replay", vec![refs_sequence(&input.replay_refs)]),
        checks_value(&["canonical-status", "owned-assertion-refs", "replay-identity-bound"]),
    ]))
}

pub fn parse_service_status(value: &IoValue) -> Result<ServiceStatus> {
    let fields = value
        .collect_simple_record("service-status-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-status-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_STATUS_SCHEMA, "service status schema")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "replay-identity-bound", "service status")?;
    let status = ServiceStatus {
        status_ref: canonical_hash(value)?,
        service_id: record_string(&fields[1], "service-id")?,
        state: record_string(&fields[2], "state")?,
        manifest_ref: record_optional_ref(&fields[3], "manifest")?,
        demand_refs: parse_ref_sequence(&fields[4], "demands")?,
        dependency_status_refs: parse_ref_sequence(&fields[5], "dependencies")?,
        readiness_assertion_refs: parse_ref_sequence(&fields[6], "readiness")?,
        failure_refs: parse_ref_sequence(&fields[7], "failures")?,
        restart_count: record_u64(&fields[8], "restart-count")?,
        monitor_refs: parse_ref_sequence(&fields[9], "monitors")?,
        replay_refs: parse_ref_sequence(&fields[10], "replay")?,
        value: value.clone(),
    };
    validate_service_id(&status.service_id, "service status service id")?;
    validate_state(&status.state)?;
    Ok(status)
}

pub fn service_supervisor_value(input: &ServiceSupervisorInput) -> Result<IoValue> {
    validate_supervisor_input(input)?;
    Ok(record("service-supervisor-v1", vec![
        string(SERVICE_SUPERVISOR_SCHEMA),
        record("supervisor-id", vec![string(&input.supervisor_id)]),
        record("services", vec![service_id_sequence(&input.service_ids)]),
        record("links", vec![refs_sequence(&input.link_refs)]),
        record("monitors", vec![refs_sequence(&input.monitor_refs)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["logical-supervision", "no-os-parentage", "policy-declared"]),
    ]))
}

pub fn parse_service_supervisor(value: &IoValue) -> Result<ServiceSupervisor> {
    let fields = value
        .collect_simple_record("service-supervisor-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-supervisor-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_SUPERVISOR_SCHEMA, "service supervisor schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "logical-supervision", "service supervisor")?;
    let supervisor = ServiceSupervisor {
        supervisor_ref: canonical_hash(value)?,
        supervisor_id: record_string(&fields[1], "supervisor-id")?,
        service_ids: parse_service_id_sequence(&fields[2], "services")?,
        link_refs: parse_ref_sequence(&fields[3], "links")?,
        monitor_refs: parse_ref_sequence(&fields[4], "monitors")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_non_empty(&supervisor.supervisor_id, "service supervisor id")?;
    Ok(supervisor)
}

pub fn service_link_value(input: &ServiceLinkInput) -> Result<IoValue> {
    validate_link_input(input)?;
    Ok(record("service-link-v1", vec![
        string(SERVICE_LINK_SCHEMA),
        record("supervisor-id", vec![string(&input.supervisor_id)]),
        record("parent-service", vec![string(&input.parent_service_id)]),
        record("child-service", vec![string(&input.child_service_id)]),
        record("propagation", vec![string(&input.propagation)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["logical-supervision", "no-os-parentage", "failure-propagation-declared"]),
    ]))
}

pub fn parse_service_link(value: &IoValue) -> Result<ServiceLink> {
    let fields = value
        .collect_simple_record("service-link-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-link-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_LINK_SCHEMA, "service link schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "no-os-parentage", "service link")?;
    let link = ServiceLink {
        link_ref: canonical_hash(value)?,
        supervisor_id: record_string(&fields[1], "supervisor-id")?,
        parent_service_id: record_string(&fields[2], "parent-service")?,
        child_service_id: record_string(&fields[3], "child-service")?,
        propagation: record_string(&fields[4], "propagation")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_link_parsed(&link)?;
    Ok(link)
}

pub fn service_monitor_value(input: &ServiceMonitorInput) -> Result<IoValue> {
    validate_monitor_input(input)?;
    Ok(record("service-monitor-v1", vec![
        string(SERVICE_MONITOR_SCHEMA),
        record("monitor-id", vec![string(&input.monitor_id)]),
        record("service-id", vec![string(&input.service_id)]),
        record("observer", vec![string(&input.observer_ref)]),
        record("notification-policy", vec![string(&input.notification_policy)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["logical-monitor", "observer-ref-bound", "no-os-parentage"]),
    ]))
}

pub fn parse_service_monitor(value: &IoValue) -> Result<ServiceMonitor> {
    let fields = value
        .collect_simple_record("service-monitor-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-monitor-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_MONITOR_SCHEMA, "service monitor schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "observer-ref-bound", "service monitor")?;
    let monitor = ServiceMonitor {
        monitor_ref: canonical_hash(value)?,
        monitor_id: record_string(&fields[1], "monitor-id")?,
        service_id: record_string(&fields[2], "service-id")?,
        observer_ref: record_ref(&fields[3], "observer")?,
        notification_policy: record_string(&fields[4], "notification-policy")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_monitor_parsed(&monitor)?;
    Ok(monitor)
}

pub fn service_restart_policy_value(input: &ServiceRestartPolicyInput) -> Result<IoValue> {
    validate_restart_policy_input(input)?;
    Ok(record("service-restart-policy-v1", vec![
        string(SERVICE_RESTART_POLICY_SCHEMA),
        record("policy-id", vec![string(&input.policy_id)]),
        record("max-attempts", vec![u64_value(input.max_attempts)]),
        record("window-steps", vec![u64_value(input.window_steps)]),
        record("backoff-steps", vec![u64_value(input.backoff_steps)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        checks_value(&["bounded-restart", "logical-time", "resource-declared"]),
    ]))
}

pub fn parse_service_restart_policy(value: &IoValue) -> Result<ServiceRestartPolicy> {
    let fields = value
        .collect_simple_record("service-restart-policy-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-restart-policy-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_RESTART_POLICY_SCHEMA, "service restart policy schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "bounded-restart", "service restart policy")?;
    let policy = ServiceRestartPolicy {
        policy_ref: canonical_hash(value)?,
        policy_id: record_string(&fields[1], "policy-id")?,
        max_attempts: record_u64(&fields[2], "max-attempts")?,
        window_steps: record_u64(&fields[3], "window-steps")?,
        backoff_steps: record_u64(&fields[4], "backoff-steps")?,
        resource_refs: parse_ref_sequence(&fields[5], "resource")?,
        value: value.clone(),
    };
    validate_restart_policy_parsed(&policy)?;
    Ok(policy)
}
