
struct HostcallAdmission {
    diagnostics: Vec<String>,
    is_declared_hostcall: bool,
    operation_ref_bound: bool,
    has_authority: bool,
    has_resources: bool,
    has_descriptor_requirements: bool,
    has_ambient_request: bool,
}

pub fn plugin_hostcall_receipt_value(input: &HostcallReceiptInput<'_>) -> Result<IoValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_non_empty(input.operation, "plugin hostcall operation")?;
    validate_ref(input.hostcall_ref, "plugin hostcall ref")?;
    validate_ref(input.executor_receipt_ref, "plugin hostcall executor ref")?;
    validate_ref(input.effect_receipt_ref, "plugin hostcall effect ref")?;
    validate_refs(input.authority_refs, "plugin hostcall authority ref")?;
    validate_refs(input.resource_refs, "plugin hostcall resource ref")?;
    validate_optional_ref(input.input_schema_ref, "plugin hostcall input schema ref")?;
    validate_optional_ref(input.output_schema_ref, "plugin hostcall output schema ref")?;
    let admission = hostcall_admission(&manifest, input)?;
    let decision = if admission.diagnostics.is_empty() {
        PLUGIN_DECISION_PASS
    } else {
        PLUGIN_DECISION_DENY
    };
    Ok(record("plugin-hostcall-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("operation", vec![string(input.operation)]),
        record("hostcall", vec![string(input.hostcall_ref)]),
        record("executor", vec![string(input.executor_receipt_ref)]),
        record("effect", vec![string(input.effect_receipt_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(&admission.diagnostics)]),
        checks_value(&[
            ("declared-hostcall", status(admission.is_declared_hostcall)),
            ("operation-ref-bound", status(admission.operation_ref_bound)),
            ("executor-boundary", PLUGIN_DECISION_PASS),
            ("effect-handle-boundary", PLUGIN_DECISION_PASS),
            ("authority-present", status(admission.has_authority)),
            ("resource-bound", status(admission.has_resources)),
            ("descriptor-specific-requirements", status(admission.has_descriptor_requirements)),
            (
                "deny-ambient-side-effect",
                status(!admission.has_ambient_request || admission.is_declared_hostcall),
            ),
        ]),
    ]))
}

fn hostcall_admission(manifest: &PluginManifest, input: &HostcallReceiptInput<'_>) -> Result<HostcallAdmission> {
    let mut diagnostics = Vec::new();
    let primitive_ref_matches = primitive_hostcall_ref(input.operation)? == input.hostcall_ref;
    let primitive_declared = primitive_ref_matches && manifest.hostcall_refs.iter().any(|value| value == input.hostcall_ref);
    let extension_descriptor = matching_bound_descriptor(
        manifest,
        input.extension_contracts,
        input.operation,
        input.hostcall_ref,
    );
    let descriptor_requirements = descriptor_requirements_pass(
        &manifest.effect_manifest_refs,
        extension_descriptor,
        input,
    );
    let is_declared_hostcall = primitive_declared || extension_descriptor.is_some();
    let operation_ref_bound = primitive_ref_matches || extension_descriptor.is_some();
    let has_authority = !input.authority_refs.is_empty();
    let has_resources = !input.resource_refs.is_empty();
    let has_ambient_request = is_ambient_operation(input.operation);
    let has_descriptor_requirements = if extension_descriptor.is_some() {
        descriptor_requirements
    } else {
        primitive_declared
    };
    if !is_declared_hostcall {
        diagnostics.push_limited(
            format!("plugin hostcall {} is not declared by active manifest or extension contracts", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if !operation_ref_bound {
        diagnostics.push_limited(
            format!("plugin hostcall operation/ref binding mismatch for {}", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if has_ambient_request && !is_declared_hostcall {
        diagnostics.push_limited(
            format!("ambient plugin hostcall {} denied before side effects", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if !has_authority {
        diagnostics.push_limited(
            "plugin hostcall requires authority evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if !has_resources {
        diagnostics.push_limited(
            "plugin hostcall requires resource evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if !has_descriptor_requirements {
        diagnostics.push_limited(
            format!("plugin hostcall {} missing descriptor-specific requirements", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    Ok(HostcallAdmission {
        diagnostics,
        is_declared_hostcall,
        operation_ref_bound,
        has_authority,
        has_resources,
        has_descriptor_requirements,
        has_ambient_request,
    })
}

fn matching_bound_descriptor<'a>(
    manifest: &PluginManifest,
    contracts: &'a [PluginExtensionContract],
    operation: &str,
    descriptor_ref: &str,
) -> Option<&'a PluginHostcallDescriptor> {
    contracts
        .iter()
        .filter(|contract| manifest.extension_contract_refs.contains(&contract.contract_ref))
        .flat_map(|contract| contract.hostcall_descriptors.iter())
        .find(|descriptor| descriptor.operation == operation && descriptor.descriptor_ref == descriptor_ref)
}

fn descriptor_requirements_pass(
    manifest_effect_refs: &[String],
    descriptor: Option<&PluginHostcallDescriptor>,
    input: &HostcallReceiptInput<'_>,
) -> bool {
    descriptor.is_some_and(|descriptor| {
        input.input_schema_ref == Some(descriptor.input_schema_ref.as_str())
            && input.output_schema_ref == Some(descriptor.output_schema_ref.as_str())
            && contains_all(input.authority_refs, &descriptor.authority_refs)
            && contains_all(input.resource_refs, &descriptor.resource_refs)
            && contains_all(manifest_effect_refs, &descriptor.effect_manifest_refs)
    })
}

pub fn parse_plugin_hostcall_receipt(value: &IoValue) -> Result<PluginHostcallReceipt> {
    let fields = simple_record(value, "plugin-hostcall-receipt-v1", PLUGIN_HOSTCALL_RECEIPT_ARITY)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_SCHEMA, "plugin hostcall receipt")?;
    let checks = parse_checks(&fields[PLUGIN_HOSTCALL_RECEIPT_ARITY - 1])?;
    require_check(&checks, "declared-hostcall", "plugin hostcall receipt")?;
    require_check(&checks, "operation-ref-bound", "plugin hostcall receipt")?;
    require_check_status(&checks, "effect-handle-boundary", PLUGIN_DECISION_PASS, "plugin hostcall receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let diagnostics = record_string_sequence(&fields[10], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin hostcall receipt")?;
    Ok(PluginHostcallReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        operation: record_string(&fields[4], "operation")?,
        hostcall_ref: record_ref(&fields[5], "hostcall")?,
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

pub fn plugin_host_abi_result_value(input: &HostAbiResultInput<'_>) -> Result<IoValue> {
    validate_host_abi_status(input.status)?;
    validate_optional_ref(input.payload_ref, "plugin ABI payload ref")?;
    if input.status == "ok" && input.error.is_some() {
        return Err(MoltenError::invalid_harness("successful plugin ABI result must not carry an error"));
    }
    if input.status == "error" && input.error.is_none() {
        return Err(MoltenError::invalid_harness("error plugin ABI result requires an error message"));
    }
    Ok(record("plugin-host-abi-result-v1", vec![
        string(crate::preserves_rail::PLUGIN_HOST_ABI_RESULT_SCHEMA),
        record("abi", vec![string(crate::preserves_rail::PLUGIN_HOST_ABI_SCHEMA)]),
        record("status", vec![string(input.status)]),
        record("payload", vec![optional_ref_value(input.payload_ref)]),
        record("error", vec![optional_text_value(input.error)]),
        checks_value(&[
            ("canonical-preserves-result", PLUGIN_DECISION_PASS),
            ("error-is-explicit", status(input.status != "error" || input.error.is_some())),
        ]),
    ]))
}
