
fn collect_capability_grant_mismatch_diagnostics(
    manifest: &PluginManifest,
    bound: &BoundHostcallDescriptor<'_>,
    input: &HostcallReceiptInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    for grant in input.capability_grants {
        if grant.plugin_ref != manifest.plugin_ref
            || grant.plugin_id != manifest.plugin_id
            || grant.manifest_ref != manifest.manifest_ref
        {
            push_grant_diagnostic(diagnostics, input.operation, "wrong-manifest")?;
            continue;
        }
        if grant.extension_contract_ref.as_deref() != Some(bound.contract_ref) {
            push_grant_diagnostic(diagnostics, input.operation, "wrong-extension")?;
            continue;
        }
        if grant.operation != bound.descriptor.operation {
            push_grant_diagnostic(diagnostics, input.operation, "wrong-operation")?;
            continue;
        }
        if grant.hostcall_descriptor_ref != bound.descriptor.descriptor_ref {
            push_grant_diagnostic(diagnostics, input.operation, "wrong-descriptor")?;
            continue;
        }
        if grant.input_schema_ref != bound.descriptor.input_schema_ref
            || grant.output_schema_ref != bound.descriptor.output_schema_ref
            || input.input_schema_ref != Some(grant.input_schema_ref.as_str())
            || input.output_schema_ref != Some(grant.output_schema_ref.as_str())
        {
            push_grant_diagnostic(diagnostics, input.operation, "wrong-schema")?;
        }
        if !contains_all(&grant.resource_refs, &bound.descriptor.resource_refs)
            || !contains_all(&grant.resource_refs, input.resource_refs)
            || !resource_scope_matches(&grant.resource_scope, input.resource_refs)
        {
            push_grant_diagnostic(diagnostics, input.operation, "wrong-resource")?;
        }
        if grant.attenuation.current_delegation_depth > grant.attenuation.max_delegation_depth {
            push_grant_diagnostic(diagnostics, input.operation, "over-delegated")?;
        }
        if input.evaluation_turn < grant.attenuation.valid_from_turn
            || input.evaluation_turn > grant.attenuation.valid_until_turn
        {
            push_grant_diagnostic(diagnostics, input.operation, "expired")?;
        }
        if grant.revoked {
            push_grant_diagnostic(diagnostics, input.operation, "revoked")?;
        }
    }
    Ok(())
}

fn push_grant_diagnostic(diagnostics: &mut impl PushLimited<String>, operation: &str, mismatch: &str) -> Result<()> {
    diagnostics.push_limited(
        format!("plugin hostcall {operation} {mismatch} capability grant"),
        MAX_PLUGIN_DIAGNOSTICS,
        "plugin hostcall diagnostics",
    )
}

pub fn parse_plugin_hostcall_receipt(value: &IoValue) -> Result<PluginHostcallReceipt> {
    crate::preserves_rail::validate_boundary_schema(
        value,
        &crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA,
    )?;
    let fields = simple_record(value, "plugin-hostcall-receipt-v1", PLUGIN_HOSTCALL_RECEIPT_ARITY)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_SCHEMA, "plugin hostcall receipt")?;
    let checks = parse_checks(&fields[PLUGIN_HOSTCALL_RECEIPT_ARITY - 1])?;
    require_check(&checks, "declared-hostcall", "plugin hostcall receipt")?;
    require_check(&checks, "operation-ref-bound", "plugin hostcall receipt")?;
    require_check(&checks, "capability-grant-match", "plugin hostcall receipt")?;
    require_check_status(&checks, "effect-handle-boundary", PLUGIN_DECISION_PASS, "plugin hostcall receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let capability_grant_refs = record_ref_sequence(&fields[9], "capability-grants")?;
    let _evaluation_turn = record_u64(&fields[11], "evaluation-turn")?;
    let diagnostics = record_string_sequence(&fields[12], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin hostcall receipt")?;
    Ok(PluginHostcallReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        operation: record_string(&fields[4], "operation")?,
        hostcall_ref: record_ref(&fields[5], "hostcall")?,
        capability_grant_refs,
        diagnostics,
        value: value.clone(),
    })
}

pub fn plugin_health_receipt_value(input: &HealthReceiptInput<'_>) -> Result<IoValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_ref(input.lifecycle_receipt_ref, "plugin health lifecycle receipt ref")?;
    validate_refs(input.service_refs, "plugin health service ref")?;
    validate_health_status(input.health_status)?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let is_healthy = input.health_status == "healthy";
    if !is_healthy && diagnostics.is_empty() {
        diagnostics.push_limited(
            "plugin health check failed".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin health diagnostics",
        )?;
    }
    let decision = if is_healthy && diagnostics.is_empty() {
        PLUGIN_DECISION_PASS
    } else {
        PLUGIN_DECISION_DENY
    };
    Ok(record("plugin-health-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_HEALTH_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("lifecycle", vec![string(input.lifecycle_receipt_ref)]),
        record("status", vec![string(input.health_status)]),
        record("services", vec![refs_sequence(input.service_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-health", PLUGIN_DECISION_PASS),
            ("service-supervision-bound", status(!input.service_refs.is_empty())),
            ("failed-health-isolated", PLUGIN_DECISION_PASS),
            ("cleanup-required-on-failure", status(is_healthy)),
        ]),
    ]))
}

pub fn parse_plugin_health_receipt(value: &IoValue) -> Result<PluginHealthReceipt> {
    let fields = simple_record(value, "plugin-health-receipt-v1", 9)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_HEALTH_RECEIPT_SCHEMA, "plugin health receipt")?;
    let checks = parse_checks(&fields[8])?;
    require_check_status(&checks, "canonical-health", PLUGIN_DECISION_PASS, "plugin health receipt")?;
    require_check_status(&checks, "failed-health-isolated", PLUGIN_DECISION_PASS, "plugin health receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let diagnostics = record_string_sequence(&fields[7], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin health receipt")?;
    Ok(PluginHealthReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        diagnostics,
        value: value.clone(),
    })
}

pub fn plugin_removal_receipt_value(input: &RemovalReceiptInput<'_>) -> Result<IoValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_ref(input.lifecycle_receipt_ref, "plugin removal lifecycle receipt ref")?;
    validate_refs(input.owned_service_refs, "plugin removal service ref")?;
    validate_refs(input.assertion_refs, "plugin removal assertion ref")?;
    validate_refs(input.handle_refs, "plugin removal handle ref")?;
    validate_refs(input.catalog_entry_refs, "plugin removal catalog ref")?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let has_service_cleanup = !input.owned_service_refs.is_empty();
    let has_assertion_cleanup = !input.assertion_refs.is_empty();
    let has_handle_cleanup = !input.handle_refs.is_empty();
    let has_catalog_cleanup = !input.catalog_entry_refs.is_empty();
    if !(has_service_cleanup && has_assertion_cleanup && has_handle_cleanup && has_catalog_cleanup) {
        diagnostics.push_limited(
            "plugin removal requires service/assertion/handle/catalog cleanup refs".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin removal diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { PLUGIN_DECISION_PASS } else { PLUGIN_DECISION_DENY };
    Ok(record("plugin-removal-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_REMOVAL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("lifecycle", vec![string(input.lifecycle_receipt_ref)]),
        record("services", vec![refs_sequence(input.owned_service_refs)]),
        record("assertions", vec![refs_sequence(input.assertion_refs)]),
        record("handles", vec![refs_sequence(input.handle_refs)]),
        record("catalog", vec![refs_sequence(input.catalog_entry_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-removal", PLUGIN_DECISION_PASS),
            ("service-retractions", status(has_service_cleanup)),
            ("assertion-retractions", status(has_assertion_cleanup)),
            ("handle-revocations", status(has_handle_cleanup)),
            ("catalog-retractions", status(has_catalog_cleanup)),
            ("complete-cleanup", status(diagnostics.is_empty())),
        ]),
    ]))
}

pub fn parse_plugin_removal_receipt(value: &IoValue) -> Result<PluginRemovalReceipt> {
    let fields = simple_record(value, "plugin-removal-receipt-v1", 11)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_REMOVAL_RECEIPT_SCHEMA, "plugin removal receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check_status(&checks, "canonical-removal", PLUGIN_DECISION_PASS, "plugin removal receipt")?;
    require_check(&checks, "complete-cleanup", "plugin removal receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let diagnostics = record_string_sequence(&fields[9], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin removal receipt")?;
    Ok(PluginRemovalReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        diagnostics,
        value: value.clone(),
    })
}

pub fn plugin_upgrade_receipt_value(input: &UpgradeReceiptInput<'_>) -> Result<IoValue> {
    let old_manifest = parse_plugin_manifest(input.old_manifest_value)?;
    let new_manifest = parse_plugin_manifest(input.new_manifest_value)?;
    validate_ref(input.rollback_ref, "plugin upgrade rollback ref")?;
    validate_refs(input.cleanup_refs, "plugin upgrade cleanup ref")?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let has_same_plugin = old_manifest.plugin_id == new_manifest.plugin_id;
    let has_compatible_abi = old_manifest.abi == new_manifest.abi;
    let has_compatible_schemas = contains_all(&new_manifest.schema_refs, &old_manifest.schema_refs);
    if !has_same_plugin {
        diagnostics.push_limited(
            "plugin upgrade cannot change plugin id".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    if !has_compatible_abi {
        diagnostics.push_limited(
            "plugin upgrade ABI is incompatible".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    if !has_compatible_schemas {
        diagnostics.push_limited(
            "plugin upgrade drops required schema refs".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    if input.cleanup_refs.is_empty() {
        diagnostics.push_limited(
            "plugin upgrade requires rollback/cleanup evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { PLUGIN_DECISION_PASS } else { PLUGIN_DECISION_DENY };
    Ok(record("plugin-upgrade-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_UPGRADE_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("old-manifest", vec![string(&old_manifest.manifest_ref)]),
        record("new-manifest", vec![string(&new_manifest.manifest_ref)]),
        record("rollback", vec![string(input.rollback_ref)]),
        record("cleanup", vec![refs_sequence(input.cleanup_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-upgrade", PLUGIN_DECISION_PASS),
            ("same-plugin", status(has_same_plugin)),
            ("abi-compatible", status(has_compatible_abi)),
            ("schema-compatible", status(has_compatible_schemas)),
            ("rollback-bound", status(!input.cleanup_refs.is_empty())),
        ]),
    ]))
}

pub fn parse_plugin_upgrade_receipt(value: &IoValue) -> Result<PluginUpgradeReceipt> {
    let fields = simple_record(value, "plugin-upgrade-receipt-v1", 8)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_UPGRADE_RECEIPT_SCHEMA, "plugin upgrade receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check_status(&checks, "canonical-upgrade", PLUGIN_DECISION_PASS, "plugin upgrade receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let diagnostics = record_string_sequence(&fields[6], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin upgrade receipt")?;
    Ok(PluginUpgradeReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        old_manifest_ref: record_ref(&fields[2], "old-manifest")?,
        new_manifest_ref: record_ref(&fields[3], "new-manifest")?,
        diagnostics,
        value: value.clone(),
    })
}
