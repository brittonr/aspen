
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
    let is_revoked = fields[1]
        .as_boolean()
        .ok_or_else(|| MoltenError::invalid_harness("plugin capability grant revoked flag must be boolean"))?;
    Ok((refs, is_revoked))
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
