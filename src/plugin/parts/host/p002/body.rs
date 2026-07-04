
struct HostcallAdmission {
    diagnostics: Vec<String>,
    is_declared_hostcall: bool,
    operation_ref_bound: bool,
    has_authority: bool,
    has_typed_capability_grant: bool,
    has_matching_capability_grant: bool,
    attenuation_valid: bool,
    revocation_valid: bool,
    has_resources: bool,
    has_descriptor_requirements: bool,
    has_ambient_request: bool,
    capability_grant_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct BoundHostcallDescriptor<'a> {
    contract_ref: &'a str,
    descriptor: &'a PluginHostcallDescriptor,
}

#[derive(Debug, Clone, Copy)]
struct GrantMatchResult<'a> {
    grant: Option<&'a PluginCapabilityGrant>,
    attenuation_valid: bool,
    revocation_valid: bool,
}

pub fn plugin_hostcall_receipt_value(input: &HostcallReceiptInput<'_>) -> Result<IoValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_non_empty(input.operation, "plugin hostcall operation")?;
    validate_ref(input.hostcall_ref, "plugin hostcall ref")?;
    validate_ref(input.executor_receipt_ref, "plugin hostcall executor ref")?;
    validate_ref(input.effect_receipt_ref, "plugin hostcall effect ref")?;
    validate_refs(input.authority_refs, "plugin hostcall authority ref")?;
    validate_capability_grants(input.capability_grants)?;
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
        record("capability-grants", vec![refs_sequence(&admission.capability_grant_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("evaluation-turn", vec![u64_value(input.evaluation_turn)]),
        record("diagnostics", vec![strings_sequence(&admission.diagnostics)]),
        checks_value(&[
            ("declared-hostcall", status(admission.is_declared_hostcall)),
            ("operation-ref-bound", status(admission.operation_ref_bound)),
            ("executor-boundary", PLUGIN_DECISION_PASS),
            ("effect-handle-boundary", PLUGIN_DECISION_PASS),
            ("authority-present", status(admission.has_authority)),
            ("typed-capability-grant-present", status(admission.has_typed_capability_grant)),
            ("capability-grant-match", status(admission.has_matching_capability_grant)),
            ("capability-attenuation-valid", status(admission.attenuation_valid)),
            ("capability-revocation-valid", status(admission.revocation_valid)),
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
    let grant_match = extension_descriptor
        .as_ref()
        .map(|bound| matching_capability_grant(manifest, bound, input))
        .transpose()?
        .unwrap_or(GrantMatchResult {
            grant: None,
            attenuation_valid: true,
            revocation_valid: true,
        });
    let descriptor_requirements = descriptor_requirements_pass(
        manifest,
        extension_descriptor.as_ref(),
        &grant_match,
        input,
    );
    let is_declared_hostcall = primitive_declared || extension_descriptor.is_some();
    let operation_ref_bound = primitive_ref_matches || extension_descriptor.is_some();
    let has_authority = !input.authority_refs.is_empty();
    let has_resources = !input.resource_refs.is_empty();
    let has_ambient_request = is_ambient_operation(input.operation);
    let has_matching_capability_grant = extension_descriptor.is_none() || grant_match.grant.is_some();
    let has_typed_capability_grant = extension_descriptor.is_none() || !input.capability_grants.is_empty();
    let attenuation_valid = extension_descriptor.is_none() || grant_match.attenuation_valid;
    let revocation_valid = extension_descriptor.is_none() || grant_match.revocation_valid;
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
    if extension_descriptor.is_some() && input.capability_grants.is_empty() {
        diagnostics.push_limited(
            format!("plugin hostcall {} missing typed capability grant", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if let Some(bound) = extension_descriptor.as_ref()
        && !has_matching_capability_grant
        && !input.capability_grants.is_empty()
    {
        collect_capability_grant_mismatch_diagnostics(manifest, bound, input, &mut diagnostics)?;
        diagnostics.push_limited(
            format!("plugin hostcall {} has no matching capability grant", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if extension_descriptor.is_some() && !attenuation_valid {
        diagnostics.push_limited(
            format!("plugin hostcall {} capability grant attenuation is invalid", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if extension_descriptor.is_some() && !revocation_valid {
        diagnostics.push_limited(
            format!("plugin hostcall {} capability grant is revoked", input.operation),
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
        has_typed_capability_grant,
        has_matching_capability_grant,
        attenuation_valid,
        revocation_valid,
        has_resources,
        has_descriptor_requirements,
        has_ambient_request,
        capability_grant_refs: capability_grant_refs(input.capability_grants),
    })
}

fn matching_bound_descriptor<'a>(
    manifest: &PluginManifest,
    contracts: &'a [PluginExtensionContract],
    operation: &str,
    descriptor_ref: &str,
) -> Option<BoundHostcallDescriptor<'a>> {
    contracts
        .iter()
        .filter(|contract| manifest.extension_contract_refs.contains(&contract.contract_ref))
        .find_map(|contract| {
            contract
                .hostcall_descriptors
                .iter()
                .find(|descriptor| descriptor.operation == operation && descriptor.descriptor_ref == descriptor_ref)
                .map(|descriptor| BoundHostcallDescriptor {
                    contract_ref: contract.contract_ref.as_str(),
                    descriptor,
                })
        })
}

fn descriptor_requirements_pass(
    manifest: &PluginManifest,
    descriptor: Option<&BoundHostcallDescriptor<'_>>,
    grant_match: &GrantMatchResult<'_>,
    input: &HostcallReceiptInput<'_>,
) -> bool {
    descriptor.is_some_and(|bound| {
        let descriptor = bound.descriptor;
        input.input_schema_ref == Some(descriptor.input_schema_ref.as_str())
            && input.output_schema_ref == Some(descriptor.output_schema_ref.as_str())
            && contains_all(input.authority_refs, &descriptor.authority_refs)
            && contains_all(input.resource_refs, &descriptor.resource_refs)
            && contains_all(&manifest.effect_manifest_refs, &descriptor.effect_manifest_refs)
            && grant_match.grant.is_some()
            && grant_match.attenuation_valid
            && grant_match.revocation_valid
    })
}

fn matching_capability_grant<'a>(
    manifest: &PluginManifest,
    bound: &BoundHostcallDescriptor<'_>,
    input: &'a HostcallReceiptInput<'_>,
) -> Result<GrantMatchResult<'a>> {
    let mut any_attenuation_invalid = false;
    let mut any_revocation_invalid = false;
    for grant in input.capability_grants {
        if !grant_identity_matches(manifest, bound, grant) {
            continue;
        }
        let attenuation_valid = grant_attenuation_matches(grant, input.evaluation_turn, input.resource_refs);
        let revocation_valid = !grant.revoked;
        if attenuation_valid && revocation_valid && grant_context_matches(manifest, bound.descriptor, input, grant) {
            return Ok(GrantMatchResult {
                grant: Some(grant),
                attenuation_valid,
                revocation_valid,
            });
        }
        any_attenuation_invalid |= !attenuation_valid;
        any_revocation_invalid |= !revocation_valid;
    }
    Ok(GrantMatchResult {
        grant: None,
        attenuation_valid: !any_attenuation_invalid,
        revocation_valid: !any_revocation_invalid,
    })
}

fn grant_identity_matches(
    manifest: &PluginManifest,
    bound: &BoundHostcallDescriptor<'_>,
    grant: &PluginCapabilityGrant,
) -> bool {
    grant.plugin_ref == manifest.plugin_ref
        && grant.plugin_id == manifest.plugin_id
        && grant.manifest_ref == manifest.manifest_ref
        && grant.extension_contract_ref.as_deref() == Some(bound.contract_ref)
        && grant.hostcall_descriptor_ref == bound.descriptor.descriptor_ref
        && grant.operation == bound.descriptor.operation
}

fn grant_context_matches(
    manifest: &PluginManifest,
    descriptor: &PluginHostcallDescriptor,
    input: &HostcallReceiptInput<'_>,
    grant: &PluginCapabilityGrant,
) -> bool {
    grant.input_schema_ref == descriptor.input_schema_ref
        && grant.output_schema_ref == descriptor.output_schema_ref
        && input.input_schema_ref == Some(grant.input_schema_ref.as_str())
        && input.output_schema_ref == Some(grant.output_schema_ref.as_str())
        && contains_all(&grant.resource_refs, &descriptor.resource_refs)
        && contains_all(&grant.resource_refs, input.resource_refs)
        && contains_all(&grant.effect_manifest_refs, &descriptor.effect_manifest_refs)
        && grant.effect_receipt_refs.iter().any(|reference| reference == input.effect_receipt_ref)
        && contains_all(&grant.policy_refs, &manifest.policy_refs)
        && !grant.proof_refs.is_empty()
}

fn grant_attenuation_matches(grant: &PluginCapabilityGrant, evaluation_turn: u64, resource_refs: &[String]) -> bool {
    grant.attenuation.current_delegation_depth <= grant.attenuation.max_delegation_depth
        && grant.attenuation.valid_from_turn <= evaluation_turn
        && evaluation_turn <= grant.attenuation.valid_until_turn
        && resource_scope_matches(&grant.resource_scope, resource_refs)
        && resource_scope_matches(&grant.attenuation.delegated_scope, resource_refs)
}

fn resource_scope_matches(scope: &str, resource_refs: &[String]) -> bool {
    scope == "*" || resource_refs.iter().any(|reference| reference == scope)
}

fn capability_grant_refs(grants: &[PluginCapabilityGrant]) -> Vec<String> {
    grants.iter().map(|grant| grant.typed_ref.as_str().to_string()).collect()
}

fn validate_capability_grants(grants: &[PluginCapabilityGrant]) -> Result<()> {
    ensure_count_at_most(grants.len(), MAX_PLUGIN_REFS, "plugin capability grant refs")?;
    for grant in grants {
        validate_ref(grant.typed_ref.as_str(), "plugin capability grant ref")?;
    }
    Ok(())
}

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
            diagnostics.push_limited(
                format!("plugin hostcall {} wrong-manifest capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
            continue;
        }
        if grant.extension_contract_ref.as_deref() != Some(bound.contract_ref) {
            diagnostics.push_limited(
                format!("plugin hostcall {} wrong-extension capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
            continue;
        }
        if grant.operation != bound.descriptor.operation {
            diagnostics.push_limited(
                format!("plugin hostcall {} wrong-operation capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
            continue;
        }
        if grant.hostcall_descriptor_ref != bound.descriptor.descriptor_ref {
            diagnostics.push_limited(
                format!("plugin hostcall {} wrong-descriptor capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
            continue;
        }
        if grant.input_schema_ref != bound.descriptor.input_schema_ref
            || grant.output_schema_ref != bound.descriptor.output_schema_ref
            || input.input_schema_ref != Some(grant.input_schema_ref.as_str())
            || input.output_schema_ref != Some(grant.output_schema_ref.as_str())
        {
            diagnostics.push_limited(
                format!("plugin hostcall {} wrong-schema capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
        }
        if !contains_all(&grant.resource_refs, &bound.descriptor.resource_refs)
            || !contains_all(&grant.resource_refs, input.resource_refs)
            || !resource_scope_matches(&grant.resource_scope, input.resource_refs)
        {
            diagnostics.push_limited(
                format!("plugin hostcall {} wrong-resource capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
        }
        if grant.attenuation.current_delegation_depth > grant.attenuation.max_delegation_depth {
            diagnostics.push_limited(
                format!("plugin hostcall {} over-delegated capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
        }
        if input.evaluation_turn < grant.attenuation.valid_from_turn
            || input.evaluation_turn > grant.attenuation.valid_until_turn
        {
            diagnostics.push_limited(
                format!("plugin hostcall {} expired capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
        }
        if grant.revoked {
            diagnostics.push_limited(
                format!("plugin hostcall {} revoked capability grant", input.operation),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin hostcall diagnostics",
            )?;
        }
    }
    Ok(())
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
