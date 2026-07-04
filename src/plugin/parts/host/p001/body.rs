
pub fn parse_plugin_manifest(value: &IoValue) -> Result<PluginManifest> {
    let fields = simple_record_any(value, "plugin-manifest-v1")?;
    let field_count = record_arity(&fields);
    if field_count != PLUGIN_MANIFEST_BASE_ARITY && field_count != PLUGIN_MANIFEST_EXTENSION_ARITY {
        return Err(MoltenError::invalid_harness(format!(
            "expected <plugin-manifest-v1 ...> with arity {PLUGIN_MANIFEST_BASE_ARITY} or {PLUGIN_MANIFEST_EXTENSION_ARITY}, got {field_count}"
        )));
    }
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_MANIFEST_SCHEMA, "plugin manifest")?;
    let plugin_id = record_string(&fields[1], "plugin-id")?;
    let artifact_ref = record_ref(&fields[2], "artifact")?;
    let abi = record_string(&fields[3], "abi")?;
    let lifecycle_callbacks = record_string_sequence(&fields[4], "lifecycle")?;
    let effect_manifest_refs = record_ref_sequence(&fields[5], "effects")?;
    let hostcall_refs = record_ref_sequence(&fields[6], "hostcalls")?;
    let schema_refs = record_ref_sequence(&fields[7], "schemas")?;
    let policy_refs = record_ref_sequence(&fields[8], "policy")?;
    let resource_refs = record_ref_sequence(&fields[9], "resource")?;
    let supply_chain_refs = record_ref_sequence(&fields[10], "supply-chain")?;
    let extension_contract_refs = if field_count == PLUGIN_MANIFEST_EXTENSION_ARITY {
        record_ref_sequence(&fields[11], "extension-contracts")?
    } else {
        Vec::new()
    };
    let checks_index = field_count.saturating_sub(1);
    let checks = parse_checks(&fields[checks_index])?;
    require_check_status(&checks, "artifact-backed", PLUGIN_DECISION_PASS, "plugin manifest")?;
    require_check_status(&checks, "no-ambient-authority", PLUGIN_DECISION_PASS, "plugin manifest")?;
    validate_plugin_id(&plugin_id)?;
    validate_abi(&abi)?;
    validate_lifecycle_callbacks(&lifecycle_callbacks)?;
    require_non_empty_refs(&effect_manifest_refs, "plugin effect manifest refs")?;
    require_non_empty_refs(&hostcall_refs, "plugin hostcall refs")?;
    require_non_empty_refs(&schema_refs, "plugin schema refs")?;
    require_non_empty_refs(&policy_refs, "plugin policy refs")?;
    require_non_empty_refs(&resource_refs, "plugin resource refs")?;
    require_non_empty_refs(&supply_chain_refs, "plugin supply-chain refs")?;
    validate_refs(&extension_contract_refs, "plugin extension contract refs")?;
    Ok(PluginManifest {
        manifest_ref: canonical_hash(value)?,
        plugin_ref: plugin_identity_ref(&plugin_id, &artifact_ref)?,
        plugin_id,
        artifact_ref,
        abi,
        lifecycle_callbacks,
        effect_manifest_refs,
        hostcall_refs,
        schema_refs,
        policy_refs,
        resource_refs,
        supply_chain_refs,
        extension_contract_refs,
        checks,
        value: value.clone(),
    })
}

pub fn install_plugin(registry_root: &std::path::Path, manifest_value: &IoValue) -> Result<PluginInstallReceipt> {
    let manifest = parse_plugin_manifest(manifest_value)?;
    let mut diagnostics = Vec::new();
    let has_artifact = crate::artifacts::read_artifact(registry_root, &manifest.artifact_ref).is_ok();
    if !has_artifact {
        diagnostics.push_limited(
            format!("plugin artifact {} is not present in registry", manifest.artifact_ref),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin install diagnostics",
        )?;
    }
    let decision = if has_artifact { PLUGIN_DECISION_PASS } else { PLUGIN_DECISION_DENY };
    let value = record("plugin-install-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_INSTALL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("artifact", vec![string(&manifest.artifact_ref)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-install", PLUGIN_DECISION_PASS),
            ("artifact-backed", PLUGIN_DECISION_PASS),
            ("artifact-present", status(has_artifact)),
            ("activation-separate", PLUGIN_DECISION_PASS),
            ("no-code-loaded", PLUGIN_DECISION_PASS),
        ]),
    ]);
    parse_plugin_install_receipt(&value)
}

pub fn parse_plugin_install_receipt(value: &IoValue) -> Result<PluginInstallReceipt> {
    let fields = simple_record(value, "plugin-install-receipt-v1", 7)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_INSTALL_RECEIPT_SCHEMA, "plugin install receipt")?;
    let diagnostics = record_string_sequence(&fields[5], "diagnostics")?;
    let checks = parse_checks(&fields[6])?;
    require_check_status(&checks, "canonical-install", PLUGIN_DECISION_PASS, "plugin install receipt")?;
    require_check_status(&checks, "activation-separate", PLUGIN_DECISION_PASS, "plugin install receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin install receipt")?;
    Ok(PluginInstallReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        artifact_ref: record_ref(&fields[4], "artifact")?,
        diagnostics,
        value: value.clone(),
    })
}

pub fn plugin_permission_receipt_value(input: &PermissionReviewInput<'_>) -> Result<IoValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_refs(input.authority_refs, "plugin authority ref")?;
    validate_refs(input.policy_refs, "plugin policy review ref")?;
    validate_refs(input.resource_refs, "plugin resource review ref")?;
    validate_refs(input.effect_receipt_refs, "plugin effect receipt ref")?;
    validate_refs(input.supply_chain_refs, "plugin supply-chain review ref")?;
    let mut diagnostics = Vec::new();
    collect_missing_refs(&manifest.policy_refs, input.policy_refs, "policy", &mut diagnostics)?;
    collect_missing_refs(&manifest.resource_refs, input.resource_refs, "resource", &mut diagnostics)?;
    collect_missing_refs(&manifest.supply_chain_refs, input.supply_chain_refs, "supply-chain", &mut diagnostics)?;
    if input.authority_refs.is_empty() {
        diagnostics.push_limited(
            "plugin activation requires explicit authority evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin permission diagnostics",
        )?;
    }
    if input.effect_receipt_refs.is_empty() {
        diagnostics.push_limited(
            "plugin activation requires effect-handle boundary evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin permission diagnostics",
        )?;
    }
    let has_authority = !input.authority_refs.is_empty();
    let has_effect_boundary = !input.effect_receipt_refs.is_empty();
    let has_current_policy = contains_all(input.policy_refs, &manifest.policy_refs);
    let has_current_resources = contains_all(input.resource_refs, &manifest.resource_refs);
    let has_current_supply_chain = contains_all(input.supply_chain_refs, &manifest.supply_chain_refs);
    let decision = if diagnostics.is_empty() { PLUGIN_DECISION_PASS } else { PLUGIN_DECISION_DENY };
    Ok(record("plugin-permission-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_PERMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("effects", vec![refs_sequence(input.effect_receipt_refs)]),
        record("supply-chain", vec![refs_sequence(input.supply_chain_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("install-not-authority", PLUGIN_DECISION_PASS),
            ("authority-present", status(has_authority)),
            ("policy-current", status(has_current_policy)),
            ("resource-bound", status(has_current_resources)),
            ("supply-chain-current", status(has_current_supply_chain)),
            ("effect-handle-boundary", status(has_effect_boundary)),
            ("no-ambient-authority", PLUGIN_DECISION_PASS),
        ]),
    ]))
}

pub fn parse_plugin_permission_receipt(value: &IoValue) -> Result<PluginPermissionReceipt> {
    let fields = simple_record(value, "plugin-permission-receipt-v1", 11)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_PERMISSION_RECEIPT_SCHEMA, "plugin permission receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check_status(&checks, "install-not-authority", PLUGIN_DECISION_PASS, "plugin permission receipt")?;
    require_check(&checks, "effect-handle-boundary", "plugin permission receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let diagnostics = record_string_sequence(&fields[9], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin permission receipt")?;
    Ok(PluginPermissionReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        diagnostics,
        value: value.clone(),
    })
}

pub fn plugin_lifecycle_receipt_value(input: &LifecycleReceiptInput<'_>) -> Result<IoValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_lifecycle_operation(input.operation)?;
    validate_ref(input.permission_receipt_ref, "plugin permission receipt ref")?;
    validate_ref(input.executor_receipt_ref, "plugin executor receipt ref")?;
    validate_refs(input.authority_refs, "plugin lifecycle authority ref")?;
    validate_refs(input.resource_refs, "plugin lifecycle resource ref")?;
    validate_refs(input.effect_receipt_refs, "plugin lifecycle effect ref")?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let is_declared_callback = is_lifecycle_declared(&manifest.lifecycle_callbacks, input.operation);
    if !is_declared_callback {
        diagnostics.push_limited(
            format!("plugin lifecycle operation {} is not declared", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if input.authority_refs.is_empty() {
        diagnostics.push_limited(
            "plugin lifecycle requires authority evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if input.resource_refs.is_empty() {
        diagnostics.push_limited(
            "plugin lifecycle requires resource evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if input.effect_receipt_refs.is_empty() {
        diagnostics.push_limited(
            "plugin lifecycle requires effect receipt evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    let has_authority = !input.authority_refs.is_empty();
    let has_resources = !input.resource_refs.is_empty();
    let has_effects = !input.effect_receipt_refs.is_empty();
    let decision = if diagnostics.is_empty() { PLUGIN_DECISION_PASS } else { PLUGIN_DECISION_DENY };
    Ok(record("plugin-lifecycle-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_LIFECYCLE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("executor", vec![string(input.executor_receipt_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("effects", vec![refs_sequence(input.effect_receipt_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-lifecycle", PLUGIN_DECISION_PASS),
            ("declared-callback", status(is_declared_callback)),
            ("executor-boundary", PLUGIN_DECISION_PASS),
            ("authority-present", status(has_authority)),
            ("resource-bound", status(has_resources)),
            ("effect-boundary", status(has_effects)),
            ("failure-isolated", PLUGIN_DECISION_PASS),
        ]),
    ]))
}

pub fn parse_plugin_lifecycle_receipt(value: &IoValue) -> Result<PluginLifecycleReceipt> {
    let fields = simple_record(value, "plugin-lifecycle-receipt-v1", 11)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_LIFECYCLE_RECEIPT_SCHEMA, "plugin lifecycle receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check_status(&checks, "canonical-lifecycle", PLUGIN_DECISION_PASS, "plugin lifecycle receipt")?;
    require_check_status(&checks, "executor-boundary", PLUGIN_DECISION_PASS, "plugin lifecycle receipt")?;
    let decision = record_decision(&fields[2], "decision")?;
    let diagnostics = record_string_sequence(&fields[9], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin lifecycle receipt")?;
    Ok(PluginLifecycleReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision,
        plugin_ref: record_ref(&fields[3], "plugin")?,
        manifest_ref: record_ref(&fields[4], "manifest")?,
        diagnostics,
        value: value.clone(),
    })
}
