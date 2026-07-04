
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

pub fn plugin_capability_grant_value(input: &PluginCapabilityGrantInput<'_>) -> Result<IoValue> {
    validate_ref(input.plugin_ref, "plugin capability grant subject plugin ref")?;
    validate_plugin_id(input.plugin_id)?;
    validate_ref(input.manifest_ref, "plugin capability grant manifest ref")?;
    validate_optional_ref(input.extension_contract_ref, "plugin capability grant extension contract ref")?;
    validate_ref(input.hostcall_descriptor_ref, "plugin capability grant hostcall descriptor ref")?;
    validate_non_empty(input.operation, "plugin capability grant operation")?;
    validate_ref(input.input_schema_ref, "plugin capability grant input schema ref")?;
    validate_ref(input.output_schema_ref, "plugin capability grant output schema ref")?;
    require_non_empty_refs(input.resource_refs, "plugin capability grant resource refs")?;
    validate_non_empty(input.resource_scope, "plugin capability grant resource scope")?;
    require_non_empty_refs(input.effect_manifest_refs, "plugin capability grant effect manifest refs")?;
    require_non_empty_refs(input.effect_receipt_refs, "plugin capability grant effect receipt refs")?;
    require_non_empty_refs(input.policy_refs, "plugin capability grant policy refs")?;
    validate_ref(input.issuer_ref, "plugin capability grant issuer ref")?;
    require_non_empty_refs(input.proof_refs, "plugin capability grant proof refs")?;
    validate_grant_attenuation_input(&input.attenuation)?;
    validate_refs(input.revocation_refs, "plugin capability grant revocation refs")?;
    validate_replay_class(input.replay_class)?;
    Ok(record("plugin-capability-grant-v1", vec![
        string(crate::preserves_rail::PLUGIN_CAPABILITY_GRANT_SCHEMA),
        record("subject", vec![
            record("plugin", vec![string(input.plugin_ref)]),
            record("plugin-id", vec![string(input.plugin_id)]),
            record("manifest", vec![string(input.manifest_ref)]),
        ]),
        record("extension-contract", vec![optional_ref_value(input.extension_contract_ref)]),
        record("hostcall", vec![
            record("descriptor", vec![string(input.hostcall_descriptor_ref)]),
            record("operation", vec![string(input.operation)]),
            record("input-schema", vec![string(input.input_schema_ref)]),
            record("output-schema", vec![string(input.output_schema_ref)]),
        ]),
        record("resource", vec![refs_sequence(input.resource_refs), string(input.resource_scope)]),
        record("effects", vec![refs_sequence(input.effect_manifest_refs), refs_sequence(input.effect_receipt_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("issuer", vec![string(input.issuer_ref)]),
        record("proofs", vec![refs_sequence(input.proof_refs)]),
        attenuation_value(&input.attenuation),
        record("revocation", vec![refs_sequence(input.revocation_refs), bool_value(input.revoked)]),
        record("replay", vec![string(input.replay_class)]),
        checks_value(&[
            ("canonical-capability-grant", PLUGIN_DECISION_PASS),
            ("typed-capability-ref", PLUGIN_DECISION_PASS),
            ("subject-bound", PLUGIN_DECISION_PASS),
            ("descriptor-bound", PLUGIN_DECISION_PASS),
            ("policy-proof-bound", PLUGIN_DECISION_PASS),
            ("attenuation-deterministic", PLUGIN_DECISION_PASS),
            ("revocation-explicit", PLUGIN_DECISION_PASS),
            ("no-ambient-authority", PLUGIN_DECISION_PASS),
        ]),
    ]))
}

pub fn parse_plugin_capability_grant(value: &IoValue) -> Result<PluginCapabilityGrant> {
    let fields = simple_record(value, "plugin-capability-grant-v1", PLUGIN_CAPABILITY_GRANT_ARITY)?;
    require_schema(
        &fields[0],
        crate::preserves_rail::PLUGIN_CAPABILITY_GRANT_SCHEMA,
        "plugin capability grant",
    )?;
    let (plugin_ref, plugin_id, manifest_ref) = parse_grant_subject(&fields[1])?;
    let extension_contract_ref = parse_optional_ref_field(&fields[2], "extension-contract")?;
    let (hostcall_descriptor_ref, operation, input_schema_ref, output_schema_ref) = parse_grant_hostcall(&fields[3])?;
    let (resource_refs, resource_scope) = parse_grant_resource(&fields[4])?;
    let (effect_manifest_refs, effect_receipt_refs) = parse_grant_effects(&fields[5])?;
    let policy_refs = record_ref_sequence(&fields[6], "policy")?;
    let issuer_ref = record_ref(&fields[7], "issuer")?;
    let proof_refs = record_ref_sequence(&fields[8], "proofs")?;
    let attenuation = parse_grant_attenuation(&fields[9])?;
    let (revocation_refs, revoked) = parse_grant_revocation(&fields[10])?;
    let replay_class = record_string(&fields[11], "replay")?;
    let checks = parse_checks(&fields[12])?;
    require_check_status(&checks, "canonical-capability-grant", PLUGIN_DECISION_PASS, "plugin capability grant")?;
    require_check_status(&checks, "typed-capability-ref", PLUGIN_DECISION_PASS, "plugin capability grant")?;
    require_check_status(&checks, "no-ambient-authority", PLUGIN_DECISION_PASS, "plugin capability grant")?;
    validate_ref(&plugin_ref, "plugin capability grant subject plugin ref")?;
    validate_plugin_id(&plugin_id)?;
    validate_ref(&manifest_ref, "plugin capability grant manifest ref")?;
    validate_optional_ref(extension_contract_ref.as_deref(), "plugin capability grant extension contract ref")?;
    validate_ref(&hostcall_descriptor_ref, "plugin capability grant hostcall descriptor ref")?;
    validate_non_empty(&operation, "plugin capability grant operation")?;
    validate_ref(&input_schema_ref, "plugin capability grant input schema ref")?;
    validate_ref(&output_schema_ref, "plugin capability grant output schema ref")?;
    require_non_empty_refs(&resource_refs, "plugin capability grant resource refs")?;
    validate_non_empty(&resource_scope, "plugin capability grant resource scope")?;
    require_non_empty_refs(&effect_manifest_refs, "plugin capability grant effect manifest refs")?;
    require_non_empty_refs(&effect_receipt_refs, "plugin capability grant effect receipt refs")?;
    require_non_empty_refs(&policy_refs, "plugin capability grant policy refs")?;
    require_non_empty_refs(&proof_refs, "plugin capability grant proof refs")?;
    validate_grant_attenuation(&attenuation)?;
    validate_refs(&revocation_refs, "plugin capability grant revocation refs")?;
    validate_replay_class(&replay_class)?;
    let grant_ref = canonical_hash(value)?;
    Ok(PluginCapabilityGrant {
        typed_ref: CapabilityGrantRef { value: grant_ref.clone() },
        grant_ref,
        plugin_ref,
        plugin_id,
        manifest_ref,
        extension_contract_ref,
        hostcall_descriptor_ref,
        operation,
        input_schema_ref,
        output_schema_ref,
        resource_refs,
        resource_scope,
        effect_manifest_refs,
        effect_receipt_refs,
        policy_refs,
        issuer_ref,
        proof_refs,
        attenuation,
        revocation_refs,
        revoked,
        replay_class,
        value: value.clone(),
    })
}

fn attenuation_value(input: &PluginCapabilityGrantAttenuationInput<'_>) -> IoValue {
    record("attenuation", vec![
        record("scope", vec![string(input.delegated_scope)]),
        record("delegation-depth", vec![u64_value(input.current_delegation_depth)]),
        record("max-delegation-depth", vec![u64_value(input.max_delegation_depth)]),
        record("budgets", vec![refs_sequence(input.budget_refs)]),
        record("validity", vec![
            record("from", vec![u64_value(input.valid_from_turn)]),
            record("until", vec![u64_value(input.valid_until_turn)]),
        ]),
    ])
}

fn parse_grant_subject(value: &Value<IoValue>) -> Result<(String, String, String)> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "subject", PLUGIN_CAPABILITY_GRANT_SUBJECT_ARITY)?;
    Ok((
        record_ref(&fields[0], "plugin")?,
        record_string(&fields[1], "plugin-id")?,
        record_ref(&fields[2], "manifest")?,
    ))
}

fn parse_grant_hostcall(value: &Value<IoValue>) -> Result<(String, String, String, String)> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "hostcall", PLUGIN_CAPABILITY_GRANT_HOSTCALL_ARITY)?;
    Ok((
        record_ref(&fields[0], "descriptor")?,
        record_string(&fields[1], "operation")?,
        record_ref(&fields[2], "input-schema")?,
        record_ref(&fields[3], "output-schema")?,
    ))
}

fn parse_grant_resource(value: &Value<IoValue>) -> Result<(Vec<String>, String)> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "resource", PLUGIN_CAPABILITY_GRANT_RESOURCE_ARITY)?;
    let resource_refs = required_ref_sequence(&fields[0], "plugin capability grant resource refs")?;
    let scope = required_string(&fields[1], "plugin capability grant resource scope")?;
    Ok((resource_refs, scope))
}

fn parse_grant_effects(value: &Value<IoValue>) -> Result<(Vec<String>, Vec<String>)> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "effects", PLUGIN_CAPABILITY_GRANT_EFFECTS_ARITY)?;
    Ok((
        required_ref_sequence(&fields[0], "plugin capability grant effect manifest refs")?,
        required_ref_sequence(&fields[1], "plugin capability grant effect receipt refs")?,
    ))
}

fn parse_grant_revocation(value: &Value<IoValue>) -> Result<(Vec<String>, bool)> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "revocation", PLUGIN_CAPABILITY_GRANT_REVOCATION_ARITY)?;
    let refs = required_ref_sequence(&fields[0], "plugin capability grant revocation refs")?;
    let revoked = fields[1]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness("plugin capability grant revoked flag must be boolean"))?;
    Ok((refs, revoked))
}

fn parse_grant_attenuation(value: &Value<IoValue>) -> Result<PluginCapabilityGrantAttenuation> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "attenuation", PLUGIN_CAPABILITY_GRANT_ATTENUATION_ARITY)?;
    let validity = value_to_iovalue(&fields[4]);
    let validity = simple_record(&validity, "validity", PLUGIN_CAPABILITY_GRANT_VALIDITY_ARITY)?;
    Ok(PluginCapabilityGrantAttenuation {
        delegated_scope: record_string(&fields[0], "scope")?,
        current_delegation_depth: record_u64(&fields[1], "delegation-depth")?,
        max_delegation_depth: record_u64(&fields[2], "max-delegation-depth")?,
        budget_refs: record_ref_sequence(&fields[3], "budgets")?,
        valid_from_turn: record_u64(&validity[0], "from")?,
        valid_until_turn: record_u64(&validity[1], "until")?,
    })
}

fn parse_optional_ref_field(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&fields[0], label)
}

fn parse_optional_ref_value(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let optional = value_to_iovalue(value);
    if optional.collect_simple_record("none", Some(0)).is_some() {
        Ok(None)
    } else if let Some(some) = optional.collect_simple_record("some", Some(1)) {
        let reference = required_string(&some[0], label)?;
        validate_ref(&reference, label)?;
        Ok(Some(reference))
    } else {
        Err(MoltenError::invalid_harness(format!("expected optional ref for {label}")))
    }
}

fn validate_grant_attenuation_input(input: &PluginCapabilityGrantAttenuationInput<'_>) -> Result<()> {
    validate_non_empty(input.delegated_scope, "plugin capability grant delegated scope")?;
    require_non_empty_refs(input.budget_refs, "plugin capability grant budget refs")?;
    validate_turn_window(input.valid_from_turn, input.valid_until_turn)
}

fn validate_grant_attenuation(input: &PluginCapabilityGrantAttenuation) -> Result<()> {
    validate_non_empty(&input.delegated_scope, "plugin capability grant delegated scope")?;
    require_non_empty_refs(&input.budget_refs, "plugin capability grant budget refs")?;
    validate_turn_window(input.valid_from_turn, input.valid_until_turn)
}

fn validate_turn_window(valid_from_turn: u64, valid_until_turn: u64) -> Result<()> {
    if valid_from_turn > valid_until_turn {
        Err(MoltenError::invalid_harness("plugin capability grant validity window is inverted"))
    } else {
        Ok(())
    }
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
