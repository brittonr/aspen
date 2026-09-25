
pub fn minimal_plugin_fixture(root: &std::path::Path) -> Result<PluginFixtureRun> {
    let registry = root.join("registry");
    let ledger_root = root.join("ledger");
    let seed = seed_refs()?;
    let manifest_value = executor_manifest(&registry, &seed, "minimal")?;
    let manifest = parse_plugin_manifest(&manifest_value)?;
    let trace = run_fixture_trace(&registry, &manifest_value, &seed)?;
    let lifecycle_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
        evaluation_kind: PluginLifecycleEvaluationKind::CompleteTrace,
        manifest: &manifest,
        install: Some(&trace.install),
        permission: Some(&trace.permission),
        activation: Some(&trace.lifecycle.start),
        hostcall: Some(&trace.call),
        health: Some(&trace.health),
        removal: Some(&trace.removal),
        upgrade: Some(&trace.upgrade),
        negotiation: None,
        compatibility: None,
        recovery_receipt_ref: None,
    })?;
    let evidence_values = vec![
        manifest_value.clone(),
        trace.install.value.clone(),
        trace.permission.value.clone(),
        trace.lifecycle.init.value.clone(),
        trace.lifecycle.start.value.clone(),
        trace.call.value.clone(),
        trace.health.value.clone(),
        trace.lifecycle.stop.value.clone(),
        trace.lifecycle.remove.value.clone(),
        trace.removal.value.clone(),
        trace.upgraded_manifest_value.clone(),
        trace.upgrade.value.clone(),
    ];
    for value in &evidence_values {
        let _ = crate::ledger::import_artifact(&ledger_root, value)?;
    }
    let report_value = plugin_fixture_report_value(&PluginFixtureReportInput {
        manifest_ref: &manifest.manifest_ref,
        install_receipt_ref: &trace.install.receipt_ref,
        permission_receipt_ref: &trace.permission.receipt_ref,
        start_receipt_ref: &trace.lifecycle.start.receipt_ref,
        hostcall_receipt_ref: &trace.call.receipt_ref,
        health_receipt_ref: &trace.health.receipt_ref,
        stop_receipt_ref: &trace.lifecycle.stop.receipt_ref,
        removal_receipt_ref: &trace.removal.receipt_ref,
        upgrade_receipt_ref: &trace.upgrade.receipt_ref,
    })?;
    let decision = trace_decision(&trace, &lifecycle_decision.decision);
    Ok(PluginFixtureRun {
        decision,
        manifest_ref: manifest.manifest_ref,
        install_receipt_ref: trace.install.receipt_ref,
        permission_receipt_ref: trace.permission.receipt_ref,
        start_receipt_ref: trace.lifecycle.start.receipt_ref,
        hostcall_receipt_ref: trace.call.receipt_ref,
        health_receipt_ref: trace.health.receipt_ref,
        stop_receipt_ref: trace.lifecycle.stop.receipt_ref,
        removal_receipt_ref: trace.removal.receipt_ref,
        upgrade_receipt_ref: trace.upgrade.receipt_ref,
        report_value,
        evidence_values,
    })
}

/// The receipts of one install, permission, lifecycle, hostcall, health, removal, and upgrade trace, and the
/// upgraded manifest installed between removal and upgrade.
struct FixtureTrace {
    install: PluginInstallReceipt,
    permission: PluginPermissionReceipt,
    lifecycle: LifeSteps,
    call: PluginHostcallReceipt,
    health: PluginHealthReceipt,
    removal: PluginRemovalReceipt,
    upgrade: PluginUpgradeReceipt,
    upgraded_manifest_value: IoValue,
}

fn run_fixture_trace(
    registry: &std::path::Path,
    manifest_value: &IoValue,
    seed: &SeedRefs,
) -> Result<FixtureTrace> {
    let install = install_plugin(registry, manifest_value)?;
    let permission = permission_step(manifest_value, seed)?;
    let lifecycle = life_steps(manifest_value, &permission.receipt_ref, seed)?;
    let call = call_step(manifest_value, seed)?;
    let service_ref = plugin_ref("service-supervision")?;
    let health = health_step(manifest_value, &lifecycle.start.receipt_ref, &service_ref)?;
    let removal = removal_step(manifest_value, &lifecycle.remove.receipt_ref, &service_ref)?;
    let upgraded_manifest_value = executor_manifest(registry, seed, "minimal-v2")?;
    let upgrade = upgrade_step(manifest_value, &upgraded_manifest_value, &removal.receipt_ref)?;
    Ok(FixtureTrace {
        install,
        permission,
        lifecycle,
        call,
        health,
        removal,
        upgrade,
        upgraded_manifest_value,
    })
}

fn trace_decision(trace: &FixtureTrace, lifecycle_decision: &str) -> String {
    run_decision(&[
        trace.install.decision.as_str(),
        trace.permission.decision.as_str(),
        trace.lifecycle.init.decision.as_str(),
        trace.lifecycle.start.decision.as_str(),
        trace.call.decision.as_str(),
        trace.health.decision.as_str(),
        trace.lifecycle.stop.decision.as_str(),
        trace.removal.decision.as_str(),
        trace.upgrade.decision.as_str(),
        lifecycle_decision,
    ])
}

struct SeedRefs {
    policy_ref: String,
    resource_ref: String,
    schema_ref: String,
    effect_manifest_ref: String,
    supply_chain_ref: String,
    authority_ref: String,
    executor_ref: String,
    effect_receipt_ref: String,
    call_ref: String,
}

struct LifeSteps {
    init: PluginLifecycleReceipt,
    start: PluginLifecycleReceipt,
    stop: PluginLifecycleReceipt,
    remove: PluginLifecycleReceipt,
}

fn seed_refs() -> Result<SeedRefs> {
    Ok(SeedRefs {
        policy_ref: plugin_ref("policy")?,
        resource_ref: plugin_ref("resource")?,
        schema_ref: plugin_ref("schema")?,
        effect_manifest_ref: plugin_ref("effect-manifest")?,
        supply_chain_ref: plugin_ref("supply-chain")?,
        authority_ref: plugin_ref("authority")?,
        executor_ref: plugin_ref("executor-preflight")?,
        effect_receipt_ref: plugin_ref("effect-receipt")?,
        call_ref: storage_read_hostcall_ref()?,
    })
}

fn executor_manifest(registry: &std::path::Path, seed: &SeedRefs, payload: &str) -> Result<IoValue> {
    let installed = crate::artifacts::install_artifact(registry, &crate::artifacts::ArtifactInstallInput {
        kind: "plugin-executor".to_string(),
        payload: record("reviewed-plugin-executor", vec![string(payload)]),
        schema_refs: vec![seed.schema_ref.clone()],
        dependency_refs: Vec::new(),
        effect_manifest_ref: Some(seed.effect_manifest_ref.clone()),
        policy_refs: vec![seed.policy_ref.clone()],
        evidence_refs: vec![seed.supply_chain_ref.clone()],
        installer_ref: seed.authority_ref.clone(),
        capability_refs: vec![seed.authority_ref.clone()],
    })?;
    plugin_manifest_value(&PluginManifestInput {
        plugin_id: "plugin:minimal",
        artifact_ref: &installed.artifact_ref,
        abi: PLUGIN_HOST_ABI_VERSION,
        lifecycle_callbacks: &string_vec(&["init", "start", "health", "stop", "remove"]),
        effect_manifest_refs: std::slice::from_ref(&seed.effect_manifest_ref),
        hostcall_refs: std::slice::from_ref(&seed.call_ref),
        schema_refs: std::slice::from_ref(&seed.schema_ref),
        policy_refs: std::slice::from_ref(&seed.policy_ref),
        resource_refs: std::slice::from_ref(&seed.resource_ref),
        supply_chain_refs: std::slice::from_ref(&seed.supply_chain_ref),
        extension_contract_refs: &[],
    })
}

fn permission_step(manifest_value: &IoValue, seed: &SeedRefs) -> Result<PluginPermissionReceipt> {
    let value = plugin_permission_receipt_value(&PermissionReviewInput {
        manifest_value,
        authority_refs: std::slice::from_ref(&seed.authority_ref),
        policy_refs: std::slice::from_ref(&seed.policy_ref),
        resource_refs: std::slice::from_ref(&seed.resource_ref),
        effect_receipt_refs: std::slice::from_ref(&seed.effect_receipt_ref),
        supply_chain_refs: std::slice::from_ref(&seed.supply_chain_ref),
    })?;
    parse_plugin_permission_receipt(&value)
}

fn life_step(
    operation: &str,
    manifest_value: &IoValue,
    permission_ref: &str,
    seed: &SeedRefs,
) -> Result<PluginLifecycleReceipt> {
    let value = plugin_lifecycle_receipt_value(&LifecycleReceiptInput {
        operation,
        manifest_value,
        permission_receipt_ref: permission_ref,
        executor_receipt_ref: &seed.executor_ref,
        authority_refs: std::slice::from_ref(&seed.authority_ref),
        resource_refs: std::slice::from_ref(&seed.resource_ref),
        effect_receipt_refs: std::slice::from_ref(&seed.effect_receipt_ref),
        diagnostics: &[],
    })?;
    parse_plugin_lifecycle_receipt(&value)
}

fn life_steps(manifest_value: &IoValue, permission_ref: &str, seed: &SeedRefs) -> Result<LifeSteps> {
    Ok(LifeSteps {
        init: life_step("init", manifest_value, permission_ref, seed)?,
        start: life_step("start", manifest_value, permission_ref, seed)?,
        stop: life_step("stop", manifest_value, permission_ref, seed)?,
        remove: life_step("remove", manifest_value, permission_ref, seed)?,
    })
}

fn call_step(manifest_value: &IoValue, seed: &SeedRefs) -> Result<PluginHostcallReceipt> {
    let value = plugin_hostcall_receipt_value(&HostcallReceiptInput {
        manifest_value,
        operation: "storage.read",
        hostcall_ref: &seed.call_ref,
        executor_receipt_ref: &seed.executor_ref,
        effect_receipt_ref: &seed.effect_receipt_ref,
        authority_refs: std::slice::from_ref(&seed.authority_ref),
        capability_grants: &[],
        resource_refs: std::slice::from_ref(&seed.resource_ref),
        extension_contracts: &[],
        input_schema_ref: None,
        output_schema_ref: None,
        evaluation_turn: PLUGIN_INITIAL_TURN,
    })?;
    parse_plugin_hostcall_receipt(&value)
}

fn health_step(manifest_value: &IoValue, lifecycle_ref: &str, service_ref: &str) -> Result<PluginHealthReceipt> {
    let service_ref = service_ref.to_string();
    let value = plugin_health_receipt_value(&HealthReceiptInput {
        manifest_value,
        lifecycle_receipt_ref: lifecycle_ref,
        service_refs: std::slice::from_ref(&service_ref),
        health_status: "healthy",
        diagnostics: &[],
    })?;
    parse_plugin_health_receipt(&value)
}

fn removal_step(manifest_value: &IoValue, lifecycle_ref: &str, service_ref: &str) -> Result<PluginRemovalReceipt> {
    let service_ref = service_ref.to_string();
    let assertion_ref = plugin_ref("assertion-retraction")?;
    let handle_ref = plugin_ref("handle-revocation")?;
    let catalog_ref = plugin_ref("catalog-retraction")?;
    let value = plugin_removal_receipt_value(&RemovalReceiptInput {
        manifest_value,
        lifecycle_receipt_ref: lifecycle_ref,
        owned_service_refs: std::slice::from_ref(&service_ref),
        assertion_refs: std::slice::from_ref(&assertion_ref),
        handle_refs: std::slice::from_ref(&handle_ref),
        catalog_entry_refs: std::slice::from_ref(&catalog_ref),
        diagnostics: &[],
    })?;
    parse_plugin_removal_receipt(&value)
}

fn upgrade_step(old_value: &IoValue, new_value: &IoValue, cleanup_ref: &str) -> Result<PluginUpgradeReceipt> {
    let rollback_ref = plugin_ref("rollback")?;
    let cleanup_ref = cleanup_ref.to_string();
    let value = plugin_upgrade_receipt_value(&UpgradeReceiptInput {
        old_manifest_value: old_value,
        new_manifest_value: new_value,
        rollback_ref: &rollback_ref,
        cleanup_refs: std::slice::from_ref(&cleanup_ref),
        diagnostics: &[],
    })?;
    parse_plugin_upgrade_receipt(&value)
}

fn run_decision(decisions: &[&str]) -> String {
    if decisions.iter().all(|decision| *decision == PLUGIN_DECISION_PASS) {
        PLUGIN_DECISION_PASS
    } else {
        PLUGIN_DECISION_DENY
    }
    .to_string()
}
