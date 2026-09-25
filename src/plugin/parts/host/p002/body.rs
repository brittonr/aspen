
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
    let is_primitive_ref_matches = primitive_hostcall_ref(input.operation)? == input.hostcall_ref;
    let is_primitive_declared = is_primitive_ref_matches && manifest.hostcall_refs.iter().any(|value| value == input.hostcall_ref);
    let extension_descriptor =
        matching_bound_descriptor(manifest, input.extension_contracts, input.operation, input.hostcall_ref);
    let grant_match = extension_descriptor
        .as_ref()
        .map(|bound| matching_capability_grant(manifest, bound, input))
        .transpose()?
        .unwrap_or(GrantMatchResult {
            grant: None,
            attenuation_valid: true,
            revocation_valid: true,
        });
    let is_descriptor_requirements = descriptor_requirements_pass(
        manifest,
        extension_descriptor.as_ref(),
        &grant_match,
        input,
    );
    let is_declared_hostcall = is_primitive_declared || extension_descriptor.is_some();
    let is_operation_ref_bound = is_primitive_ref_matches || extension_descriptor.is_some();
    let has_authority = !input.authority_refs.is_empty();
    let has_resources = !input.resource_refs.is_empty();
    let has_ambient_request = is_ambient_operation(input.operation);
    let has_matching_capability_grant = extension_descriptor.is_none() || grant_match.grant.is_some();
    let has_typed_capability_grant = extension_descriptor.is_none() || !input.capability_grants.is_empty();
    let is_attenuation_valid = extension_descriptor.is_none() || grant_match.attenuation_valid;
    let is_revocation_valid = extension_descriptor.is_none() || grant_match.revocation_valid;
    let has_descriptor_requirements = if extension_descriptor.is_some() {
        is_descriptor_requirements
    } else {
        is_primitive_declared
    };
    push_hostcall_gap(&mut diagnostics, !is_declared_hostcall, || format!("plugin hostcall {} is not declared by active manifest or extension contracts", input.operation))?;
    push_hostcall_gap(&mut diagnostics, !is_operation_ref_bound, || format!("plugin hostcall operation/ref binding mismatch for {}", input.operation))?;
    push_hostcall_gap(&mut diagnostics, has_ambient_request && !is_declared_hostcall, || format!("ambient plugin hostcall {} denied before side effects", input.operation))?;
    push_hostcall_gap(&mut diagnostics, !has_authority, || "plugin hostcall requires authority evidence".to_string())?;
    push_hostcall_gap(&mut diagnostics, !has_resources, || "plugin hostcall requires resource evidence".to_string())?;
    push_hostcall_gap(&mut diagnostics, extension_descriptor.is_some() && input.capability_grants.is_empty(), || format!("plugin hostcall {} missing typed capability grant", input.operation))?;
    if let Some(bound) = extension_descriptor.as_ref()
        && !has_matching_capability_grant
        && !input.capability_grants.is_empty()
    {
        collect_capability_grant_mismatch_diagnostics(manifest, bound, input, &mut diagnostics)?;
        let diagnostic = format!("plugin hostcall {} has no matching capability grant", input.operation);
        diagnostics.push_limited(diagnostic, MAX_PLUGIN_DIAGNOSTICS, "plugin hostcall diagnostics")?;
    }
    push_hostcall_gap(&mut diagnostics, extension_descriptor.is_some() && !is_attenuation_valid, || format!("plugin hostcall {} capability grant attenuation is invalid", input.operation))?;
    push_hostcall_gap(&mut diagnostics, extension_descriptor.is_some() && !is_revocation_valid, || format!("plugin hostcall {} capability grant is revoked", input.operation))?;
    push_hostcall_gap(&mut diagnostics, !has_descriptor_requirements, || format!("plugin hostcall {} missing descriptor-specific requirements", input.operation))?;
    Ok(HostcallAdmission {
        diagnostics,
        is_declared_hostcall,
        operation_ref_bound: is_operation_ref_bound,
        has_authority,
        has_typed_capability_grant,
        has_matching_capability_grant,
        attenuation_valid: is_attenuation_valid,
        revocation_valid: is_revocation_valid,
        has_resources,
        has_descriptor_requirements,
        has_ambient_request,
        capability_grant_refs: capability_grant_refs(input.capability_grants),
    })
}

fn push_hostcall_gap(
    diagnostics: &mut impl PushLimited<String>,
    is_gap: bool,
    diagnostic: impl FnOnce() -> String,
) -> Result<()> {
    if is_gap {
        diagnostics.push_limited(diagnostic(), MAX_PLUGIN_DIAGNOSTICS, "plugin hostcall diagnostics")?;
    }
    Ok(())
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
    let mut is_any_attenuation_invalid = false;
    let mut is_any_revocation_invalid = false;
    for grant in input.capability_grants {
        if !grant_identity_matches(manifest, bound, grant) {
            continue;
        }
        let is_attenuation_valid = grant_attenuation_matches(grant, input.evaluation_turn, input.resource_refs);
        let is_revocation_valid = !grant.revoked;
        if is_attenuation_valid && is_revocation_valid && grant_context_matches(manifest, bound.descriptor, input, grant) {
            return Ok(GrantMatchResult {
                grant: Some(grant),
                attenuation_valid: is_attenuation_valid,
                revocation_valid: is_revocation_valid,
            });
        }
        is_any_attenuation_invalid |= !is_attenuation_valid;
        is_any_revocation_invalid |= !is_revocation_valid;
    }
    Ok(GrantMatchResult {
        grant: None,
        attenuation_valid: !is_any_attenuation_invalid,
        revocation_valid: !is_any_revocation_invalid,
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
