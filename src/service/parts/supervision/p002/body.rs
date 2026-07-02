
fn service_supervision_gate_receipt_value(input: &GateReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision, "service supervision gate receipt decision")?;
    let gate_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("service-supervision-gate-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::SERVICE_SUPERVISION_GATE_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("report", vec![crate::preserves_rail::string(input.report_ref)]),
        crate::preserves_rail::record("suite", vec![crate::preserves_rail::string(input.suite_ref)]),
        crate::preserves_rail::record("restart-decision", vec![optional_string_value(input.restart_decision)]),
        crate::preserves_rail::record("status-count", vec![crate::preserves_rail::u64_value(count_as_u64(
            input.status_count,
            "service status count",
        )?)]),
        crate::preserves_rail::record("monitor-count", vec![crate::preserves_rail::u64_value(count_as_u64(
            input.monitor_count,
            "service monitor count",
        )?)]),
        crate::preserves_rail::record("cleanup-count", vec![crate::preserves_rail::u64_value(count_as_u64(
            input.cleanup_count,
            "service cleanup count",
        )?)]),
        crate::preserves_rail::record("diagnostics", vec![strings_sequence(input.diagnostics)]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("supervision-report-replay"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("failure-status-lifecycle-bound"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("restart-decision-bound"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("monitor-notifications-bound"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("cleanup-evidence-bound"),
                crate::preserves_rail::string(gate_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("service-supervision-gate-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

#[derive(Debug, Clone, Copy)]
pub struct ReportValueInput<'a> {
    pub suite_value: &'a IoValue,
    pub failure_markers: &'a [IoValue],
    pub statuses: &'a [IoValue],
    pub lifecycle_receipts: &'a [IoValue],
    pub monitor_notifications: &'a [IoValue],
    pub restart_decisions: &'a [IoValue],
    pub scheduled_demands: &'a [IoValue],
    pub cleanup_receipts: &'a [IoValue],
    pub retractions: &'a [IoValue],
    pub retention_inputs: &'a [IoValue],
}

pub fn supervision_fixture_suite_value() -> Result<IoValue> {
    let refs = fixture_refs()?;
    let restart_policy = fixture_restart_policy(&refs)?;
    let restart_policy_ref = crate::preserves_rail::canonical_hash(&restart_policy)?;
    let manifest = fixture_manifest(&refs, restart_policy_ref)?;
    let manifest_ref = crate::preserves_rail::canonical_hash(&manifest)?;
    let monitors = fixture_monitors(&refs.policy_ref)?;
    let link = fixture_link(&refs.policy_ref)?;
    let owned_state = fixture_owned_state(manifest_ref)?;
    service_supervision_suite_value(&ServiceSupervisionSuiteInput {
        manifest,
        links: vec![link],
        monitors,
        restart_policy,
        owned_state,
        restart_attempt: 0,
        logical_step: 0,
        evidence: ServiceSupervisionEvidenceInput {
            authority_refs: vec![refs.authority_ref],
            resource_refs: vec![refs.resource_ref],
            revocation_refs: Vec::new(),
            retention_policy_refs: vec![refs.retention_ref],
            prior_lifecycle_refs: vec![synthetic_ref("prior-lifecycle")?],
            effect_log_refs: vec![refs.effect_ref],
        },
    })
}

#[derive(Debug, Clone)]
struct FixtureRefs {
    authority_ref: String,
    resource_ref: String,
    retention_ref: String,
    effect_ref: String,
    policy_ref: String,
}

fn fixture_refs() -> Result<FixtureRefs> {
    Ok(FixtureRefs {
        authority_ref: synthetic_ref("authority")?,
        resource_ref: synthetic_ref("resource")?,
        retention_ref: synthetic_ref("retention")?,
        effect_ref: synthetic_ref("effect-log")?,
        policy_ref: synthetic_ref("policy")?,
    })
}

fn fixture_restart_policy(refs: &FixtureRefs) -> Result<IoValue> {
    crate::service_records::service_restart_policy_value(&crate::service_records::ServiceRestartPolicyInput {
        policy_id: "restart:web".to_string(),
        max_attempts: 2,
        window_steps: 8,
        backoff_steps: 0,
        resource_refs: vec![refs.resource_ref.clone()],
    })
}

fn fixture_manifest(refs: &FixtureRefs, restart_policy_ref: String) -> Result<IoValue> {
    crate::service_records::service_manifest_value(&crate::service_records::ServiceManifestInput {
        service_id: "svc:web".to_string(),
        owner_authority_ref: refs.authority_ref.clone(),
        target_ref: synthetic_ref("target")?,
        dependencies: Vec::new(),
        provided_assertion_refs: vec![synthetic_ref("ready-pattern")?],
        restart_policy_ref,
        policy_refs: vec![refs.policy_ref.clone()],
        resource_refs: vec![refs.resource_ref.clone()],
        effect_profile_refs: vec![refs.effect_ref.clone()],
    })
}

fn fixture_monitors(policy_ref: &str) -> Result<Vec<IoValue>> {
    let first_monitor = crate::service_records::service_monitor_value(&crate::service_records::ServiceMonitorInput {
        monitor_id: "monitor:web:b".to_string(),
        service_id: "svc:web".to_string(),
        observer_ref: synthetic_ref("observer-b")?,
        notification_policy: "failure".to_string(),
        policy_refs: vec![policy_ref.to_string()],
    })?;
    let second_monitor = crate::service_records::service_monitor_value(&crate::service_records::ServiceMonitorInput {
        monitor_id: "monitor:web:a".to_string(),
        service_id: "svc:web".to_string(),
        observer_ref: synthetic_ref("observer-a")?,
        notification_policy: "failure".to_string(),
        policy_refs: vec![policy_ref.to_string()],
    })?;
    Ok(vec![first_monitor, second_monitor])
}

fn fixture_link(policy_ref: &str) -> Result<IoValue> {
    crate::service_records::service_link_value(&crate::service_records::ServiceLinkInput {
        supervisor_id: "supervisor:web".to_string(),
        parent_service_id: "svc:web".to_string(),
        child_service_id: "svc:web".to_string(),
        propagation: "restart".to_string(),
        policy_refs: vec![policy_ref.to_string()],
    })
}

fn fixture_owned_state(manifest_ref: String) -> Result<IoValue> {
    service_owned_state_value(&ServiceOwnedStateInput {
        service_id: "svc:web".to_string(),
        manifest_ref: Some(manifest_ref),
        owned_assertion_refs: vec![synthetic_ref("readiness")?],
        observer_refs: vec![synthetic_ref("observer-registration")?],
        live_ref_refs: vec![synthetic_ref("live-ref")?],
        exposed_ref_refs: vec![synthetic_ref("exposed-ref")?],
        pending_effect_refs: vec![synthetic_ref("pending-effect")?],
        foreign_ref_claims: Vec::new(),
    })
}

fn failure_marker_value(suite: &ServiceSupervisionSuite) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("service-failure-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::SERVICE_FAILURE_MARKER_SCHEMA),
        crate::preserves_rail::record("service-id", vec![crate::preserves_rail::string(&suite.manifest.service_id)]),
        crate::preserves_rail::record("manifest", vec![crate::preserves_rail::string(&suite.manifest.manifest_ref)]),
        crate::preserves_rail::record("prior-lifecycle", vec![refs_sequence(&suite.evidence.prior_lifecycle_refs)]),
        crate::preserves_rail::record("effect-log", vec![refs_sequence(&suite.evidence.effect_log_refs)]),
        checks_value(&["canonical-service-failure", "logical-supervision", "replay-bound"]),
    ]))
}

fn parse_failure_marker_ref(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("service-failure-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-failure-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SERVICE_FAILURE_MARKER_SCHEMA, "service failure marker schema")?;
    let checks = parse_checks(&fields[5])?;
    require_check(&checks, "logical-supervision", "service failure marker")?;
    crate::preserves_rail::canonical_hash(value)
}

fn parse_monitor_notification_ref(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("service-monitor-notification-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-monitor-notification-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::SERVICE_MONITOR_NOTIFICATION_SCHEMA,
        "service monitor notification schema",
    )?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "failure-bound", "service monitor notification")?;
    crate::preserves_rail::canonical_hash(value)
}

fn parse_retraction_ref(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("service-retraction-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-retraction-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SERVICE_RETRACTION_SCHEMA, "service retraction schema")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "service-owned-retraction", "service retraction")?;
    crate::preserves_rail::canonical_hash(value)
}

fn failure_status_value(
    suite: &ServiceSupervisionSuite,
    failure_ref: &str,
    monitor_refs: &[String],
) -> Result<IoValue> {
    crate::service_records::service_status_value(&crate::service_records::ServiceStatusInput {
        service_id: suite.manifest.service_id.clone(),
        state: "failed".to_string(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        demand_refs: Vec::new(),
        dependency_status_refs: Vec::new(),
        readiness_assertion_refs: Vec::new(),
        failure_refs: vec![failure_ref.to_string()],
        restart_count: suite.restart_attempt,
        monitor_refs: monitor_refs.to_vec(),
        replay_refs: suite.evidence.prior_lifecycle_refs.clone(),
    })
}

fn final_status_values(
    suite: &ServiceSupervisionSuite,
    failure_ref: &str,
    restart: &RestartEvaluation,
    restart_decision_ref: &str,
    monitor_refs: &[String],
) -> Result<Vec<IoValue>> {
    if restart.decision != "deny" {
        return Ok(Vec::new());
    }
    let state = if suite.evidence.revocation_refs.is_empty() {
        "failed"
    } else {
        "stopped"
    };
    let status = crate::service_records::service_status_value(&crate::service_records::ServiceStatusInput {
        service_id: suite.manifest.service_id.clone(),
        state: state.to_string(),
        manifest_ref: Some(suite.manifest.manifest_ref.clone()),
        demand_refs: Vec::new(),
        dependency_status_refs: Vec::new(),
        readiness_assertion_refs: Vec::new(),
        failure_refs: vec![failure_ref.to_string(), restart_decision_ref.to_string()],
        restart_count: restart.attempt,
        monitor_refs: monitor_refs.to_vec(),
        replay_refs: suite.evidence.prior_lifecycle_refs.clone(),
    })?;
    Ok(vec![status])
}

fn monitor_notification_values(
    monitors: &[crate::service_records::ServiceMonitor],
    suite: &ServiceSupervisionSuite,
    failure_ref: &str,
    failure_status_ref: &str,
) -> Result<Vec<IoValue>> {
    let mut notifications = Vec::with_capacity(monitors.len());
    for monitor in monitors {
        let notification = crate::preserves_rail::record("service-monitor-notification-v1", vec![
            crate::preserves_rail::string(crate::preserves_rail::SERVICE_MONITOR_NOTIFICATION_SCHEMA),
            crate::preserves_rail::record("service-id", vec![crate::preserves_rail::string(
                &suite.manifest.service_id,
            )]),
            crate::preserves_rail::record("monitor", vec![crate::preserves_rail::string(&monitor.monitor_ref)]),
            crate::preserves_rail::record("observer", vec![crate::preserves_rail::string(&monitor.observer_ref)]),
            crate::preserves_rail::record("failure", vec![crate::preserves_rail::string(failure_ref)]),
            crate::preserves_rail::record("status", vec![crate::preserves_rail::string(failure_status_ref)]),
            checks_value(&["monitor-order-bound", "logical-notification", "failure-bound"]),
        ]);
        notifications.push(notification);
    }
    Ok(notifications)
}
