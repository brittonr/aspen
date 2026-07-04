
fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "plugin checks")?;
    ensure_count_at_most(items.len(), MAX_PLUGIN_CHECKS, "plugin checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "plugin check name")?;
        let status = required_string(&check[1], "plugin check status")?;
        match status.as_str() {
            PLUGIN_DECISION_PASS | PLUGIN_CHECK_FAIL | "diagnostic" => {
                parsed.push_limited((name, status), MAX_PLUGIN_CHECKS, "plugin checks")?
            }
            _ => return Err(MoltenError::invalid_harness("plugin check status must be pass/fail/diagnostic")),
        }
    }
    Ok(parsed)
}

fn require_check(checks: &[(String, String)], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(name, _)| name == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    validate_ref(&reference, label)?;
    Ok(reference)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_PLUGIN_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited(required_string(item, label)?, MAX_PLUGIN_REFS, label)?;
    }
    Ok(values)
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = record_string_sequence(value, label)?;
    validate_refs(&values, label)?;
    Ok(values)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

pub fn plugin_extension_contract_value(input: &PluginExtensionContractInput<'_>) -> Result<IoValue> {
    validate_extension_id(input.extension_id)?;
    validate_extension_version(input.version)?;
    validate_abi(input.compatible_host_abi)?;
    validate_lifecycle_callbacks(input.lifecycle_callbacks)?;
    ensure_count_at_most(
        input.hostcall_descriptors.len(),
        MAX_PLUGIN_HOSTCALL_DESCRIPTORS,
        "plugin extension hostcall descriptors",
    )?;
    if input.hostcall_descriptors.is_empty() {
        return Err(MoltenError::invalid_harness("plugin extension contract requires hostcall descriptors"));
    }
    require_non_empty_refs(input.policy_refs, "plugin extension policy refs")?;
    require_non_empty_refs(input.supply_chain_refs, "plugin extension supply-chain refs")?;
    let hostcalls = input
        .hostcall_descriptors
        .iter()
        .map(hostcall_descriptor_value)
        .collect::<Result<Vec<_>>>()?;
    Ok(record("plugin-extension-contract-v1", vec![
        string(crate::preserves_rail::PLUGIN_EXTENSION_CONTRACT_SCHEMA),
        record("extension-id", vec![string(input.extension_id)]),
        record("version", vec![string(input.version)]),
        record("host-abi", vec![string(input.compatible_host_abi)]),
        record("lifecycle", vec![strings_sequence(input.lifecycle_callbacks)]),
        record("hostcalls", vec![sequence(hostcalls)]),
        conformance_value(&input.conformance)?,
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("supply-chain", vec![refs_sequence(input.supply_chain_refs)]),
        record("profile", vec![string(profile_name(input.production_profile))]),
        checks_value(&[
            ("canonical-contract", PLUGIN_DECISION_PASS),
            ("compatible-host-abi", PLUGIN_DECISION_PASS),
            ("descriptor-refs-bound", PLUGIN_DECISION_PASS),
            ("conformance-bound", PLUGIN_DECISION_PASS),
            ("no-ambient-authority", PLUGIN_DECISION_PASS),
        ]),
    ]))
}

fn hostcall_descriptor_value(input: &PluginHostcallDescriptorInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "plugin extension hostcall operation")?;
    validate_ref(input.descriptor_ref, "plugin extension hostcall descriptor ref")?;
    validate_ref(input.input_schema_ref, "plugin extension input schema ref")?;
    validate_ref(input.output_schema_ref, "plugin extension output schema ref")?;
    require_non_empty_refs(input.authority_refs, "plugin extension authority refs")?;
    require_non_empty_refs(input.resource_refs, "plugin extension resource refs")?;
    require_non_empty_refs(input.effect_manifest_refs, "plugin extension effect manifest refs")?;
    validate_replay_class(input.replay_class)?;
    validate_refs(input.error_class_refs, "plugin extension error class refs")?;
    Ok(record("hostcall-descriptor", vec![
        record("operation", vec![string(input.operation)]),
        record("descriptor", vec![string(input.descriptor_ref)]),
        record("input-schema", vec![string(input.input_schema_ref)]),
        record("output-schema", vec![string(input.output_schema_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("effects", vec![refs_sequence(input.effect_manifest_refs)]),
        record("replay", vec![string(input.replay_class)]),
        record("errors", vec![refs_sequence(input.error_class_refs)]),
    ]))
}

fn conformance_value(input: &PluginExtensionConformanceInput<'_>) -> Result<IoValue> {
    validate_ref(input.positive_suite_ref, "plugin extension positive conformance ref")?;
    validate_ref(input.negative_suite_ref, "plugin extension negative conformance ref")?;
    validate_ref(input.property_suite_ref, "plugin extension property conformance ref")?;
    Ok(record("conformance", vec![
        record("positive", vec![string(input.positive_suite_ref)]),
        record("negative", vec![string(input.negative_suite_ref)]),
        record("property", vec![string(input.property_suite_ref)]),
    ]))
}

pub fn parse_plugin_extension_contract(value: &IoValue) -> Result<PluginExtensionContract> {
    let fields = simple_record(value, "plugin-extension-contract-v1", PLUGIN_EXTENSION_CONTRACT_ARITY)?;
    require_schema(
        &fields[0],
        crate::preserves_rail::PLUGIN_EXTENSION_CONTRACT_SCHEMA,
        "plugin extension contract",
    )?;
    let extension_id = record_string(&fields[1], "extension-id")?;
    let version = record_string(&fields[2], "version")?;
    let compatible_host_abi = record_string(&fields[3], "host-abi")?;
    let lifecycle_callbacks = record_string_sequence(&fields[4], "lifecycle")?;
    let hostcall_descriptors = parse_hostcall_descriptors(&fields[5])?;
    let conformance = parse_conformance(&fields[6])?;
    let policy_refs = record_ref_sequence(&fields[7], "policy")?;
    let supply_chain_refs = record_ref_sequence(&fields[8], "supply-chain")?;
    let production_profile = parse_profile(&fields[9])?;
    let checks = parse_checks(&fields[10])?;
    require_check_status(&checks, "canonical-contract", PLUGIN_DECISION_PASS, "plugin extension contract")?;
    require_check_status(&checks, "no-ambient-authority", PLUGIN_DECISION_PASS, "plugin extension contract")?;
    validate_extension_id(&extension_id)?;
    validate_extension_version(&version)?;
    validate_abi(&compatible_host_abi)?;
    validate_lifecycle_callbacks(&lifecycle_callbacks)?;
    if hostcall_descriptors.is_empty() {
        return Err(MoltenError::invalid_harness("plugin extension contract requires hostcall descriptors"));
    }
    require_non_empty_refs(&policy_refs, "plugin extension policy refs")?;
    require_non_empty_refs(&supply_chain_refs, "plugin extension supply-chain refs")?;
    Ok(PluginExtensionContract {
        contract_ref: canonical_hash(value)?,
        extension_id,
        version,
        compatible_host_abi,
        lifecycle_callbacks,
        hostcall_descriptors,
        conformance,
        policy_refs,
        supply_chain_refs,
        production_profile,
        value: value.clone(),
    })
}

fn parse_hostcall_descriptors(value: &Value<IoValue>) -> Result<Vec<PluginHostcallDescriptor>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "hostcalls", 1)?;
    let items = required_sequence(&fields[0], "plugin extension hostcalls")?;
    ensure_count_at_most(
        items.len(),
        MAX_PLUGIN_HOSTCALL_DESCRIPTORS,
        "plugin extension hostcall descriptors",
    )?;
    let mut descriptors = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        descriptors.push_limited(
            parse_hostcall_descriptor(&item)?,
            MAX_PLUGIN_HOSTCALL_DESCRIPTORS,
            "plugin extension hostcall descriptors",
        )?;
    }
    ensure_unique_descriptors(&descriptors)?;
    Ok(descriptors)
}

fn parse_hostcall_descriptor(value: &IoValue) -> Result<PluginHostcallDescriptor> {
    let fields = simple_record(value, "hostcall-descriptor", PLUGIN_HOSTCALL_DESCRIPTOR_ARITY)?;
    let operation = record_string(&fields[0], "operation")?;
    let descriptor_ref = record_ref(&fields[1], "descriptor")?;
    let input_schema_ref = record_ref(&fields[2], "input-schema")?;
    let output_schema_ref = record_ref(&fields[3], "output-schema")?;
    let authority_refs = record_ref_sequence(&fields[4], "authority")?;
    let resource_refs = record_ref_sequence(&fields[5], "resource")?;
    let effect_manifest_refs = record_ref_sequence(&fields[6], "effects")?;
    let replay_class = record_string(&fields[7], "replay")?;
    let error_class_refs = record_ref_sequence(&fields[8], "errors")?;
    validate_non_empty(&operation, "plugin extension hostcall operation")?;
    require_non_empty_refs(&authority_refs, "plugin extension authority refs")?;
    require_non_empty_refs(&resource_refs, "plugin extension resource refs")?;
    require_non_empty_refs(&effect_manifest_refs, "plugin extension effect manifest refs")?;
    validate_replay_class(&replay_class)?;
    Ok(PluginHostcallDescriptor {
        operation,
        descriptor_ref,
        input_schema_ref,
        output_schema_ref,
        authority_refs,
        resource_refs,
        effect_manifest_refs,
        replay_class,
        error_class_refs,
    })
}

fn parse_conformance(value: &Value<IoValue>) -> Result<PluginExtensionConformance> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, "conformance", PLUGIN_CONFORMANCE_ARITY)?;
    Ok(PluginExtensionConformance {
        positive_suite_ref: record_ref(&fields[0], "positive")?,
        negative_suite_ref: record_ref(&fields[1], "negative")?,
        property_suite_ref: record_ref(&fields[2], "property")?,
    })
}

fn parse_profile(value: &Value<IoValue>) -> Result<bool> {
    let profile = record_string(value, "profile")?;
    match profile.as_str() {
        PLUGIN_PROFILE_PRODUCTION => Ok(true),
        PLUGIN_PROFILE_DEVELOPMENT => Ok(false),
        _ => Err(MoltenError::invalid_harness(format!(
            "plugin extension profile {profile} must be production or development"
        ))),
    }
}

pub fn plugin_extension_negotiation_receipt_value(input: &PluginExtensionNegotiationInput<'_>) -> Result<IoValue> {
    validate_refs(input.required_contract_refs, "plugin extension required contract refs")?;
    validate_refs(input.optional_contract_refs, "plugin extension optional contract refs")?;
    validate_refs(input.host_supported_contract_refs, "plugin extension host supported refs")?;
    validate_ref(input.host_feature_snapshot_ref, "plugin extension host feature snapshot ref")?;
    let mut diagnostics = Vec::new();
    let mut selected_refs = Vec::new();
    negotiate_required(input, &mut selected_refs, &mut diagnostics)?;
    negotiate_optional(input, &mut selected_refs, &mut diagnostics)?;
    let required_present = input.required_contract_refs.iter().all(|reference| selected_refs.contains(reference));
    let optional_policy_ok = input.allow_optional_omission
        || input.optional_contract_refs.iter().all(|reference| selected_refs.contains(reference));
    let conformance_bound = selected_refs.iter().all(|reference| {
        contract_for_ref(input.extension_contracts, reference)
            .is_some_and(|contract| !input.production_profile || contract.production_profile)
    });
    let decision = if diagnostics.is_empty() {
        PLUGIN_DECISION_PASS
    } else {
        PLUGIN_DECISION_DENY
    };
    Ok(record("plugin-extension-negotiation-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_EXTENSION_NEGOTIATION_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("manifest", vec![string(&input.manifest.manifest_ref)]),
        record("required", vec![refs_sequence(input.required_contract_refs)]),
        record("optional", vec![refs_sequence(input.optional_contract_refs)]),
        record("host-supported", vec![refs_sequence(input.host_supported_contract_refs)]),
        record("selected", vec![refs_sequence(&selected_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("required-extensions-present", status(required_present)),
            ("optional-omission-policy", status(optional_policy_ok)),
            ("conformance-bound", status(conformance_bound)),
            ("fail-closed-negotiation", PLUGIN_DECISION_PASS),
            ("no-implicit-fallback", status(diagnostics.is_empty())),
        ]),
    ]))
}

fn negotiate_required(
    input: &PluginExtensionNegotiationInput<'_>,
    selected_refs: &mut impl PushLimited<String>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    for reference in input.required_contract_refs {
        let in_manifest = input.manifest.extension_contract_refs.contains(reference);
        let host_supports = input.host_supported_contract_refs.contains(reference);
        let contract = contract_for_ref(input.extension_contracts, reference);
        if in_manifest && host_supports && production_profile_ok(contract, input.production_profile) {
            selected_refs.push_limited(reference.clone(), MAX_PLUGIN_REFS, "plugin selected extension refs")?;
        } else {
            diagnostics.push_limited(
                format!("plugin required extension contract {reference} is missing, incompatible, or lacks production conformance"),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension negotiation diagnostics",
            )?;
        }
    }
    Ok(())
}

fn negotiate_optional(
    input: &PluginExtensionNegotiationInput<'_>,
    selected_refs: &mut impl PushLimited<String>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    for reference in input.optional_contract_refs {
        let selectable = input.manifest.extension_contract_refs.contains(reference)
            && input.host_supported_contract_refs.contains(reference)
            && production_profile_ok(contract_for_ref(input.extension_contracts, reference), input.production_profile);
        if selectable {
            selected_refs.push_limited(reference.clone(), MAX_PLUGIN_REFS, "plugin selected extension refs")?;
        } else if !input.allow_optional_omission {
            diagnostics.push_limited(
                format!("plugin optional extension contract {reference} cannot be omitted by policy"),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension negotiation diagnostics",
            )?;
        }
    }
    Ok(())
}

pub fn parse_plugin_extension_negotiation_receipt(value: &IoValue) -> Result<PluginExtensionNegotiationReceipt> {
    let fields = simple_record(
        value,
        "plugin-extension-negotiation-receipt-v1",
        PLUGIN_NEGOTIATION_RECEIPT_ARITY,
    )?;
    require_schema(
        &fields[0],
        crate::preserves_rail::PLUGIN_EXTENSION_NEGOTIATION_RECEIPT_SCHEMA,
        "plugin extension negotiation receipt",
    )?;
    let checks = parse_checks(&fields[8])?;
    require_check_status(&checks, "fail-closed-negotiation", PLUGIN_DECISION_PASS, "plugin extension negotiation receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let diagnostics = record_string_sequence(&fields[7], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin extension negotiation receipt")?;
    Ok(PluginExtensionNegotiationReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        manifest_ref: record_ref(&fields[2], "manifest")?,
        required_contract_refs: record_ref_sequence(&fields[3], "required")?,
        optional_contract_refs: record_ref_sequence(&fields[4], "optional")?,
        selected_contract_refs: record_ref_sequence(&fields[6], "selected")?,
        diagnostics,
        value: value.clone(),
    })
}

pub fn plugin_extension_compatibility_receipt_value(input: &PluginExtensionCompatibilityInput<'_>) -> Result<IoValue> {
    validate_refs(input.migration_refs, "plugin extension migration refs")?;
    validate_ref(input.rollback_ref, "plugin extension rollback ref")?;
    validate_refs(input.cleanup_refs, "plugin extension cleanup refs")?;
    let mut diagnostics = Vec::new();
    if input.old_manifest.plugin_id != input.new_manifest.plugin_id {
        diagnostics.push_limited(
            "plugin extension compatibility cannot change plugin id".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin extension compatibility diagnostics",
        )?;
    }
    if input.old_manifest.abi != input.new_manifest.abi {
        diagnostics.push_limited(
            "plugin extension compatibility cannot change host ABI".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin extension compatibility diagnostics",
        )?;
    }
    let retained_required = retained_required_contracts(input, &mut diagnostics)?;
    let compatible_versions = compatible_extension_versions(input, &mut diagnostics)?;
    let retained_hostcalls = retained_hostcall_descriptors(input, &mut diagnostics)?;
    let schema_compatible = schema_compatible(input, &mut diagnostics)?;
    let requirements_compatible = requirements_compatible(input, &mut diagnostics)?;
    let conformance_bound = compatibility_conformance_bound(input, &mut diagnostics)?;
    if input.cleanup_refs.is_empty() {
        diagnostics.push_limited(
            "plugin extension compatibility requires cleanup refs".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin extension compatibility diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() {
        PLUGIN_DECISION_PASS
    } else {
        PLUGIN_DECISION_DENY
    };
    let old_contract_refs = input.old_manifest.extension_contract_refs.clone();
    let new_contract_refs = input.new_manifest.extension_contract_refs.clone();
    Ok(record("plugin-extension-compatibility-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_EXTENSION_COMPATIBILITY_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("old-manifest", vec![string(&input.old_manifest.manifest_ref)]),
        record("new-manifest", vec![string(&input.new_manifest.manifest_ref)]),
        record("old-contracts", vec![refs_sequence(&old_contract_refs)]),
        record("new-contracts", vec![refs_sequence(&new_contract_refs)]),
        record("migration", vec![refs_sequence(input.migration_refs)]),
        record("rollback", vec![string(input.rollback_ref)]),
        record("cleanup", vec![refs_sequence(input.cleanup_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("same-plugin", status(input.old_manifest.plugin_id == input.new_manifest.plugin_id)),
            ("host-abi-compatible", status(input.old_manifest.abi == input.new_manifest.abi)),
            ("required-extensions-retained", status(retained_required)),
            ("version-compatible", status(compatible_versions)),
            ("hostcall-descriptors-retained", status(retained_hostcalls)),
            ("schema-compatible", status(schema_compatible)),
            ("authority-resource-effect-compatible", status(requirements_compatible)),
            ("rollback-cleanup-bound", status(!input.cleanup_refs.is_empty())),
            ("conformance-bound", status(conformance_bound)),
        ]),
    ]))
}

pub fn parse_plugin_extension_compatibility_receipt(value: &IoValue) -> Result<PluginExtensionCompatibilityReceipt> {
    let fields = simple_record(
        value,
        "plugin-extension-compatibility-receipt-v1",
        PLUGIN_COMPATIBILITY_RECEIPT_ARITY,
    )?;
    require_schema(
        &fields[0],
        crate::preserves_rail::PLUGIN_EXTENSION_COMPATIBILITY_RECEIPT_SCHEMA,
        "plugin extension compatibility receipt",
    )?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "required-extensions-retained", "plugin extension compatibility receipt")?;
    require_check(&checks, "conformance-bound", "plugin extension compatibility receipt")?;
    let decision = record_decision(&fields[1], "decision")?;
    let diagnostics = record_string_sequence(&fields[9], "diagnostics")?;
    validate_receipt_coherence(&decision, &checks, &diagnostics, "plugin extension compatibility receipt")?;
    Ok(PluginExtensionCompatibilityReceipt {
        receipt_ref: canonical_hash(value)?,
        decision,
        old_manifest_ref: record_ref(&fields[2], "old-manifest")?,
        new_manifest_ref: record_ref(&fields[3], "new-manifest")?,
        diagnostics,
        value: value.clone(),
    })
}

fn retained_required_contracts(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let mut retained = true;
    for old_ref in &input.old_manifest.extension_contract_refs {
        let Some(old_contract) = contract_for_ref(input.old_contracts, old_ref) else {
            retained = false;
            diagnostics.push_limited(
                format!("old plugin extension contract {old_ref} is unavailable for compatibility"),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
            continue;
        };
        if matching_new_contract(input, old_contract).is_none() {
            retained = false;
            diagnostics.push_limited(
                format!("plugin extension upgrade removes required extension {}", old_contract.extension_id),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
        }
    }
    Ok(retained)
}

fn compatible_extension_versions(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let mut compatible = true;
    for old_ref in &input.old_manifest.extension_contract_refs {
        let Some(old_contract) = contract_for_ref(input.old_contracts, old_ref) else {
            compatible = false;
            continue;
        };
        let Some(new_contract) = matching_new_contract(input, old_contract) else {
            compatible = false;
            continue;
        };
        if !version_not_downgrade(&new_contract.version, &old_contract.version)? {
            compatible = false;
            diagnostics.push_limited(
                format!("plugin extension {} downgrades from {} to {}", old_contract.extension_id, old_contract.version, new_contract.version),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
        }
    }
    Ok(compatible)
}

fn retained_hostcall_descriptors(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let mut retained = true;
    for old_ref in &input.old_manifest.extension_contract_refs {
        let Some(old_contract) = contract_for_ref(input.old_contracts, old_ref) else {
            retained = false;
            continue;
        };
        let Some(new_contract) = matching_new_contract(input, old_contract) else {
            retained = false;
            continue;
        };
        for old_descriptor in &old_contract.hostcall_descriptors {
            if find_descriptor(new_contract, &old_descriptor.operation, &old_descriptor.descriptor_ref).is_none()
                && input.migration_refs.is_empty()
            {
                retained = false;
                diagnostics.push_limited(
                    format!("plugin extension upgrade removes required hostcall {}", old_descriptor.operation),
                    MAX_PLUGIN_DIAGNOSTICS,
                    "plugin extension compatibility diagnostics",
                )?;
            }
        }
    }
    Ok(retained)
}

fn schema_compatible(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    compare_descriptors(input, diagnostics, |old, new| {
        old.input_schema_ref == new.input_schema_ref && old.output_schema_ref == new.output_schema_ref
    }, "plugin extension upgrade changes hostcall schema without migration")
}

fn requirements_compatible(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    compare_descriptors(input, diagnostics, |old, new| {
        contains_all(&new.authority_refs, &old.authority_refs)
            && contains_all(&new.resource_refs, &old.resource_refs)
            && contains_all(&new.effect_manifest_refs, &old.effect_manifest_refs)
    }, "plugin extension upgrade weakens authority/resource/effect requirements")
}

fn compare_descriptors(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
    predicate: impl Fn(&PluginHostcallDescriptor, &PluginHostcallDescriptor) -> bool,
    message: &str,
) -> Result<bool> {
    let mut compatible = true;
    for old_ref in &input.old_manifest.extension_contract_refs {
        let Some(old_contract) = contract_for_ref(input.old_contracts, old_ref) else {
            compatible = false;
            continue;
        };
        let Some(new_contract) = matching_new_contract(input, old_contract) else {
            compatible = false;
            continue;
        };
        for old_descriptor in &old_contract.hostcall_descriptors {
            if let Some(new_descriptor) = find_descriptor(new_contract, &old_descriptor.operation, &old_descriptor.descriptor_ref)
                && !predicate(old_descriptor, new_descriptor)
                && input.migration_refs.is_empty()
            {
                compatible = false;
                diagnostics.push_limited(
                    format!("{message}: {}", old_descriptor.operation),
                    MAX_PLUGIN_DIAGNOSTICS,
                    "plugin extension compatibility diagnostics",
                )?;
            }
        }
    }
    Ok(compatible)
}

fn compatibility_conformance_bound(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let mut bound = true;
    for new_ref in &input.new_manifest.extension_contract_refs {
        let Some(contract) = contract_for_ref(input.new_contracts, new_ref) else {
            bound = false;
            diagnostics.push_limited(
                format!("new plugin extension contract {new_ref} is unavailable for conformance"),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
            continue;
        };
        if input.production_profile && !contract.production_profile {
            bound = false;
            diagnostics.push_limited(
                format!("plugin extension contract {} lacks production conformance evidence", contract.extension_id),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
        }
    }
    Ok(bound)
}

fn matching_new_contract<'a>(
    input: &'a PluginExtensionCompatibilityInput<'_>,
    old_contract: &PluginExtensionContract,
) -> Option<&'a PluginExtensionContract> {
    input
        .new_manifest
        .extension_contract_refs
        .iter()
        .filter_map(|reference| contract_for_ref(input.new_contracts, reference))
        .find(|candidate| candidate.extension_id == old_contract.extension_id)
}

fn find_descriptor<'a>(
    contract: &'a PluginExtensionContract,
    operation: &str,
    descriptor_ref: &str,
) -> Option<&'a PluginHostcallDescriptor> {
    contract
        .hostcall_descriptors
        .iter()
        .find(|descriptor| descriptor.operation == operation && descriptor.descriptor_ref == descriptor_ref)
}

fn contract_for_ref<'a>(contracts: &'a [PluginExtensionContract], reference: &str) -> Option<&'a PluginExtensionContract> {
    contracts.iter().find(|contract| contract.contract_ref == reference)
}

fn production_profile_ok(contract: Option<&PluginExtensionContract>, production_profile: bool) -> bool {
    contract.is_some_and(|contract| !production_profile || contract.production_profile)
}

fn ensure_unique_descriptors(descriptors: &[PluginHostcallDescriptor]) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for descriptor in descriptors {
        let key = (descriptor.operation.clone(), descriptor.descriptor_ref.clone());
        if !seen.insert(key) {
            return Err(MoltenError::invalid_harness(format!(
                "duplicate plugin extension hostcall descriptor {}",
                descriptor.operation
            )));
        }
    }
    Ok(())
}

fn validate_extension_id(value: &str) -> Result<()> {
    validate_non_empty(value, "plugin extension id")?;
    if !value.starts_with("plugin-extension:") {
        return Err(MoltenError::invalid_harness(format!(
            "plugin extension id {value} must start with plugin-extension:"
        )));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_' | '.'))
    {
        return Err(MoltenError::invalid_harness(format!("unsupported plugin extension id {value}")));
    }
    Ok(())
}

fn validate_extension_version(value: &str) -> Result<()> {
    validate_non_empty(value, "plugin extension version")?;
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_'))
    {
        return Err(MoltenError::invalid_harness(format!("unsupported plugin extension version {value}")));
    }
    Ok(())
}

fn validate_replay_class(value: &str) -> Result<()> {
    validate_non_empty(value, "plugin extension replay class")
}

fn profile_name(production_profile: bool) -> &'static str {
    if production_profile {
        PLUGIN_PROFILE_PRODUCTION
    } else {
        PLUGIN_PROFILE_DEVELOPMENT
    }
}

fn version_not_downgrade(new_version: &str, old_version: &str) -> Result<bool> {
    let new_parts = semver_parts(new_version)?;
    let old_parts = semver_parts(old_version)?;
    Ok(new_parts >= old_parts)
}

fn semver_parts(version: &str) -> Result<Vec<u64>> {
    validate_extension_version(version)?;
    let mut parts = Vec::new();
    for raw in version.split('.').take(PLUGIN_SEMVER_PARTS) {
        let numeric = raw
            .chars()
            .take_while(|character| character.is_ascii_digit())
            .collect::<String>();
        if numeric.is_empty() {
            parts.push_limited(0, PLUGIN_SEMVER_PARTS, "plugin extension semver parts")?;
        } else {
            let parsed = numeric.parse::<u64>().map_err(|error| {
                MoltenError::invalid_harness(format!("plugin extension version {version} has unsupported numeric part: {error}"))
            })?;
            parts.push_limited(parsed, PLUGIN_SEMVER_PARTS, "plugin extension semver parts")?;
        }
    }
    while parts.len() < PLUGIN_SEMVER_PARTS {
        parts.push_limited(0, PLUGIN_SEMVER_PARTS, "plugin extension semver parts")?;
    }
    Ok(parts)
}
