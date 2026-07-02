
pub fn summary(value: &preserves::IOValue) -> Result<String> {
    if let Ok(report) = parse_report(value) {
        let pass_count = report
            .lifecycle_receipts
            .iter()
            .filter_map(|receipt| crate::service_records::parse_service_lifecycle_receipt(receipt).ok())
            .filter(|receipt| receipt.decision == "pass")
            .count();
        return Ok(format!(
            "service runtime report ref={} suite={} lifecycle={} pass={} statuses={} readiness={}",
            report.report_ref,
            report.suite_ref,
            report.lifecycle_receipts.len(),
            pass_count,
            report.statuses.len(),
            report.readiness_assertions.len()
        ));
    }
    if let Ok(suite) = parse_suite(value) {
        return Ok(format!(
            "service runtime suite ref={} manifests={} demands={} statuses={}",
            suite.suite_ref,
            suite.manifests.len(),
            suite.demands.len(),
            suite.statuses.len()
        ));
    }
    crate::service_records::service_summary(value)
}

pub fn two_service_suite_value() -> Result<preserves::IOValue> {
    let evidence = EvidenceInput {
        authority_refs: vec![synthetic_ref("service-authority")?],
        policy_refs: vec![synthetic_ref("service-policy")?],
        resource_refs: vec![synthetic_ref("service-resource")?],
        effect_profile_refs: vec![synthetic_ref("service-effect")?],
        source_gate_refs: vec![synthetic_ref("service-source-gate")?],
        scheduler_ref: Some(synthetic_ref("service-scheduler")?),
        effect_log_refs: vec![synthetic_ref("service-effect-log")?],
    };
    let backend_manifest =
        crate::service_records::service_manifest_value(&crate::service_records::ServiceManifestInput {
            service_id: "svc:backend".to_string(),
            owner_authority_ref: evidence.authority_refs[0].clone(),
            target_ref: synthetic_ref("backend-target")?,
            dependencies: Vec::new(),
            provided_assertion_refs: vec![synthetic_ref("backend-ready-pattern")?],
            restart_policy_ref: synthetic_ref("backend-restart")?,
            policy_refs: evidence.policy_refs.clone(),
            resource_refs: evidence.resource_refs.clone(),
            effect_profile_refs: evidence.effect_profile_refs.clone(),
        })?;
    let frontend_manifest =
        crate::service_records::service_manifest_value(&crate::service_records::ServiceManifestInput {
            service_id: "svc:frontend".to_string(),
            owner_authority_ref: evidence.authority_refs[0].clone(),
            target_ref: synthetic_ref("frontend-target")?,
            dependencies: vec!["svc:backend".to_string()],
            provided_assertion_refs: vec![synthetic_ref("frontend-ready-pattern")?],
            restart_policy_ref: synthetic_ref("frontend-restart")?,
            policy_refs: evidence.policy_refs.clone(),
            resource_refs: evidence.resource_refs.clone(),
            effect_profile_refs: evidence.effect_profile_refs.clone(),
        })?;
    let backend_demand = crate::service_records::service_demand_value(&crate::service_records::ServiceDemandInput {
        demand_id: "demand:backend".to_string(),
        service_id: "svc:backend".to_string(),
        requester_ref: synthetic_ref("operator")?,
        manifest_ref: Some(canonical_hash(&backend_manifest)?),
        policy_refs: evidence.policy_refs.clone(),
    })?;
    let frontend_demand = crate::service_records::service_demand_value(&crate::service_records::ServiceDemandInput {
        demand_id: "demand:frontend".to_string(),
        service_id: "svc:frontend".to_string(),
        requester_ref: synthetic_ref("operator")?,
        manifest_ref: Some(canonical_hash(&frontend_manifest)?),
        policy_refs: evidence.policy_refs.clone(),
    })?;
    suite_value(&SuiteInput {
        manifests: vec![backend_manifest, frontend_manifest],
        demands: vec![backend_demand, frontend_demand],
        statuses: Vec::new(),
        evidence,
    })
}

struct ReportValueInput<'a> {
    suite_value: &'a preserves::IOValue,
    lifecycle_receipts: &'a [preserves::IOValue],
    statuses: &'a [preserves::IOValue],
    readiness_assertions: &'a [preserves::IOValue],
    replay_identities: &'a [preserves::IOValue],
    turn_contexts: &'a [preserves::IOValue],
}

fn report_value(input: ReportValueInput<'_>) -> Result<preserves::IOValue> {
    Ok(record("service-runtime-report-v1", vec![
        string(RUNTIME_REPORT_SCHEMA),
        record("suite", vec![input.suite_value.clone()]),
        record("lifecycle", vec![sequence(input.lifecycle_receipts.to_vec())]),
        record("statuses", vec![sequence(input.statuses.to_vec())]),
        record("readiness", vec![sequence(input.readiness_assertions.to_vec())]),
        record("replay-identities", vec![sequence(input.replay_identities.to_vec())]),
        record("turn-contexts", vec![sequence(input.turn_contexts.to_vec())]),
        checks_value(&[
            "canonical-service-runtime-report",
            "replayable-suite-embedded",
            "no-text-evidence",
            "side-effects-recorded",
        ]),
    ]))
}

fn start_outcome(
    runtime: &mut crate::runtime::RuntimeState,
    evidence: &EvidenceInput,
    demand: &crate::service_records::ServiceDemand,
    manifest: &crate::service_records::ServiceManifest,
    dependency_status_refs: Vec<String>,
) -> Result<DemandOutcome> {
    let replay_identity = replay_identity_value(evidence, demand, manifest, &dependency_status_refs)?;
    let replay_identity_ref = canonical_hash(&replay_identity)?;
    let readiness = readiness_assertion_value(demand, manifest, &dependency_status_refs)?;
    let readiness_ref = canonical_hash(&readiness)?;
    let runtime_value = crate::runtime::RuntimeValue::new(readiness.clone())?;
    let step = crate::runtime::RuntimeStep::Assert {
        actor: manifest.service_id.clone(),
        value: runtime_value,
    };
    let events = runtime.apply_step(&step);
    let turn_context = turn_context_value(demand, manifest, &readiness_ref, &events)?;
    let status = crate::service_records::service_status_value(&crate::service_records::ServiceStatusInput {
        service_id: manifest.service_id.clone(),
        state: "ready".to_string(),
        manifest_ref: Some(manifest.manifest_ref.clone()),
        demand_refs: vec![demand.demand_ref.clone()],
        dependency_status_refs,
        readiness_assertion_refs: vec![readiness_ref],
        failure_refs: Vec::new(),
        restart_count: 0,
        monitor_refs: Vec::new(),
        replay_refs: vec![replay_identity_ref],
    })?;
    let status_ref = canonical_hash(&status)?;
    let lifecycle_receipt = crate::service_records::service_lifecycle_receipt_value(
        &crate::service_records::ServiceLifecycleReceiptInput {
            operation: "start".to_string(),
            decision: "pass".to_string(),
            service_id: manifest.service_id.clone(),
            manifest_ref: Some(manifest.manifest_ref.clone()),
            status_ref: Some(status_ref),
            authority_refs: evidence.authority_refs.clone(),
            resource_refs: evidence.resource_refs.clone(),
            effect_profile_refs: evidence.effect_profile_refs.clone(),
            supervision_refs: Vec::new(),
            diagnostics: Vec::new(),
        },
    )?;
    Ok(DemandOutcome {
        lifecycle_receipt,
        status: Some(status),
        readiness: Some(readiness),
        replay_identity: Some(replay_identity),
        turn_context: Some(turn_context),
    })
}

fn missing_manifest_outcome(demand: &crate::service_records::ServiceDemand) -> Result<DemandOutcome> {
    deny_outcome(demand, None, "service demand has no matching manifest")
}

fn deny_outcome(
    demand: &crate::service_records::ServiceDemand,
    manifest: Option<&crate::service_records::ServiceManifest>,
    diagnostic: &str,
) -> Result<DemandOutcome> {
    let lifecycle_receipt = crate::service_records::service_lifecycle_receipt_value(
        &crate::service_records::ServiceLifecycleReceiptInput {
            operation: "start".to_string(),
            decision: "deny".to_string(),
            service_id: demand.service_id.clone(),
            manifest_ref: manifest.map(|manifest| manifest.manifest_ref.clone()),
            status_ref: None,
            authority_refs: Vec::new(),
            resource_refs: Vec::new(),
            effect_profile_refs: Vec::new(),
            supervision_refs: Vec::new(),
            diagnostics: vec![diagnostic.to_string()],
        },
    )?;
    Ok(DemandOutcome {
        lifecycle_receipt,
        status: None,
        readiness: None,
        replay_identity: None,
        turn_context: None,
    })
}

fn dependency_wait_outcome(
    demand: &crate::service_records::ServiceDemand,
    manifest: Option<&crate::service_records::ServiceManifest>,
    diagnostic: &str,
) -> Result<DemandOutcome> {
    dependency_resolution_outcome(demand, manifest, "diagnostic", diagnostic)
}

fn dependency_deny_outcome(
    demand: &crate::service_records::ServiceDemand,
    manifest: Option<&crate::service_records::ServiceManifest>,
    diagnostic: &str,
) -> Result<DemandOutcome> {
    dependency_resolution_outcome(demand, manifest, "deny", diagnostic)
}

fn dependency_resolution_outcome(
    demand: &crate::service_records::ServiceDemand,
    manifest: Option<&crate::service_records::ServiceManifest>,
    decision: &str,
    diagnostic: &str,
) -> Result<DemandOutcome> {
    let lifecycle_receipt = crate::service_records::service_lifecycle_receipt_value(
        &crate::service_records::ServiceLifecycleReceiptInput {
            operation: "dependency-wait".to_string(),
            decision: decision.to_string(),
            service_id: demand.service_id.clone(),
            manifest_ref: manifest.map(|manifest| manifest.manifest_ref.clone()),
            status_ref: None,
            authority_refs: Vec::new(),
            resource_refs: Vec::new(),
            effect_profile_refs: Vec::new(),
            supervision_refs: Vec::new(),
            diagnostics: vec![diagnostic.to_string()],
        },
    )?;
    Ok(DemandOutcome {
        lifecycle_receipt,
        status: None,
        readiness: None,
        replay_identity: None,
        turn_context: None,
    })
}

fn readiness_assertion_value(
    demand: &crate::service_records::ServiceDemand,
    manifest: &crate::service_records::ServiceManifest,
    dependency_status_refs: &[String],
) -> Result<preserves::IOValue> {
    Ok(record("service-readiness-v1", vec![
        string(SERVICE_READINESS_ASSERTION_SCHEMA),
        record("service-id", vec![string(&manifest.service_id)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("demand", vec![string(&demand.demand_ref)]),
        record("dependencies", vec![refs_sequence(dependency_status_refs)]),
        checks_value(&[
            "service-owned-assertion",
            "dependency-readiness-bound",
            "cleanup-identifiable",
        ]),
    ]))
}

fn replay_identity_value(
    evidence: &EvidenceInput,
    demand: &crate::service_records::ServiceDemand,
    manifest: &crate::service_records::ServiceManifest,
    dependency_status_refs: &[String],
) -> Result<preserves::IOValue> {
    Ok(record("service-replay-identity-v1", vec![
        string(SERVICE_REPLAY_IDENTITY_SCHEMA),
        record("service-id", vec![string(&manifest.service_id)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("demand", vec![string(&demand.demand_ref)]),
        record("dependencies", vec![refs_sequence(dependency_status_refs)]),
        record("authority", vec![refs_sequence(&evidence.authority_refs)]),
        record("policy", vec![refs_sequence(&evidence.policy_refs)]),
        record("resource", vec![refs_sequence(&evidence.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&evidence.effect_profile_refs)]),
        record("source-gate", vec![refs_sequence(&evidence.source_gate_refs)]),
        record("scheduler", vec![optional_ref_value(evidence.scheduler_ref.as_deref())]),
        record("effect-log", vec![refs_sequence(&evidence.effect_log_refs)]),
        checks_value(&[
            "demand-bound",
            "dependency-bound",
            "authority-resource-effect-bound",
            "source-gate-bound",
        ]),
    ]))
}
