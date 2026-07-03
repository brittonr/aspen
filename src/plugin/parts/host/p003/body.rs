
pub fn minimal_plugin_fixture(root: &std::path::Path) -> Result<PluginFixtureRun> {
    let registry = root.join("registry");
    let ledger_root = root.join("ledger");
    let seed = seed_refs()?;
    let manifest_value = executor_manifest(&registry, &seed, "minimal")?;
    let manifest = parse_plugin_manifest(&manifest_value)?;
    let install = install_plugin(&registry, &manifest_value)?;
    let permission = permission_step(&manifest_value, &seed)?;
    let lifecycle = life_steps(&manifest_value, &permission.receipt_ref, &seed)?;
    let call = call_step(&manifest_value, &seed)?;
    let service_ref = plugin_ref("service-supervision")?;
    let health = health_step(&manifest_value, &lifecycle.start.receipt_ref, &service_ref)?;
    let removal = removal_step(&manifest_value, &lifecycle.remove.receipt_ref, &service_ref)?;
    let upgraded_manifest_value = executor_manifest(&registry, &seed, "minimal-v2")?;
    let upgrade = upgrade_step(&manifest_value, &upgraded_manifest_value, &removal.receipt_ref)?;
    let lifecycle_decision = evaluate_plugin_lifecycle_state(&PluginLifecycleStateInput {
        evaluation_kind: PluginLifecycleEvaluationKind::CompleteTrace,
        manifest: &manifest,
        install: Some(&install),
        permission: Some(&permission),
        activation: Some(&lifecycle.start),
        hostcall: Some(&call),
        health: Some(&health),
        removal: Some(&removal),
        upgrade: Some(&upgrade),
        recovery_receipt_ref: None,
    })?;
    let evidence_values = vec![
        manifest_value.clone(),
        install.value.clone(),
        permission.value.clone(),
        lifecycle.init.value.clone(),
        lifecycle.start.value.clone(),
        call.value.clone(),
        health.value.clone(),
        lifecycle.stop.value.clone(),
        lifecycle.remove.value.clone(),
        removal.value.clone(),
        upgraded_manifest_value.clone(),
        upgrade.value.clone(),
    ];
    for value in &evidence_values {
        let _ = crate::ledger::import_artifact(&ledger_root, value)?;
    }
    let report_value = plugin_fixture_report_value(&PluginFixtureReportInput {
        manifest_ref: &manifest.manifest_ref,
        install_receipt_ref: &install.receipt_ref,
        permission_receipt_ref: &permission.receipt_ref,
        start_receipt_ref: &lifecycle.start.receipt_ref,
        hostcall_receipt_ref: &call.receipt_ref,
        health_receipt_ref: &health.receipt_ref,
        stop_receipt_ref: &lifecycle.stop.receipt_ref,
        removal_receipt_ref: &removal.receipt_ref,
        upgrade_receipt_ref: &upgrade.receipt_ref,
    })?;
    let decision = run_decision(&[
        install.decision.as_str(),
        permission.decision.as_str(),
        lifecycle.init.decision.as_str(),
        lifecycle.start.decision.as_str(),
        call.decision.as_str(),
        health.decision.as_str(),
        lifecycle.stop.decision.as_str(),
        removal.decision.as_str(),
        upgrade.decision.as_str(),
        lifecycle_decision.decision.as_str(),
    ]);
    Ok(PluginFixtureRun {
        decision,
        manifest_ref: manifest.manifest_ref,
        install_receipt_ref: install.receipt_ref,
        permission_receipt_ref: permission.receipt_ref,
        start_receipt_ref: lifecycle.start.receipt_ref,
        hostcall_receipt_ref: call.receipt_ref,
        health_receipt_ref: health.receipt_ref,
        stop_receipt_ref: lifecycle.stop.receipt_ref,
        removal_receipt_ref: removal.receipt_ref,
        upgrade_receipt_ref: upgrade.receipt_ref,
        report_value,
        evidence_values,
    })
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
        resource_refs: std::slice::from_ref(&seed.resource_ref),
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
    if decisions.iter().all(|decision| *decision == "pass") {
        "pass"
    } else {
        "deny"
    }
    .to_string()
}

pub fn plugin_summary(value: &IoValue) -> Result<String> {
    if let Some(summary) = core_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = receipt_summary(value) {
        return Ok(summary);
    }
    if value.collect_simple_record("plugin-fixture-report-v1", Some(11)).is_some() {
        return Ok(format!("plugin fixture report ref={} (summary is non-normative)", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported plugin host artifact for summary"))
}

fn core_summary(value: &IoValue) -> Option<String> {
    if let Ok(manifest) = parse_plugin_manifest(value) {
        return Some(format!(
            "plugin manifest ref={} id={} artifact={} hostcalls={} lifecycle={} (summary is non-normative)",
            manifest.manifest_ref,
            manifest.plugin_id,
            manifest.artifact_ref,
            manifest.hostcall_refs.len(),
            manifest.lifecycle_callbacks.len()
        ));
    }
    if let Ok(install) = parse_plugin_install_receipt(value) {
        return Some(format!(
            "plugin install receipt ref={} decision={} manifest={} artifact={} diagnostics={} (summary is non-normative)",
            install.receipt_ref,
            install.decision,
            install.manifest_ref,
            install.artifact_ref,
            install.diagnostics.len()
        ));
    }
    if let Ok(permission) = parse_plugin_permission_receipt(value) {
        return Some(format!(
            "plugin permission receipt ref={} decision={} manifest={} diagnostics={} (summary is non-normative)",
            permission.receipt_ref,
            permission.decision,
            permission.manifest_ref,
            permission.diagnostics.len()
        ));
    }
    if let Ok(lifecycle) = parse_plugin_lifecycle_receipt(value) {
        return Some(format!(
            "plugin lifecycle receipt ref={} operation={} decision={} diagnostics={} (summary is non-normative)",
            lifecycle.receipt_ref,
            lifecycle.operation,
            lifecycle.decision,
            lifecycle.diagnostics.len()
        ));
    }
    None
}
