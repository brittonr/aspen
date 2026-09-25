
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
    let is_required_present = input.required_contract_refs.iter().all(|reference| selected_refs.contains(reference));
    let is_optional_policy_ok = input.allow_optional_omission
        || input.optional_contract_refs.iter().all(|reference| selected_refs.contains(reference));
    let is_conformance_bound = selected_refs.iter().all(|reference| {
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
            ("required-extensions-present", status(is_required_present)),
            ("optional-omission-policy", status(is_optional_policy_ok)),
            ("conformance-bound", status(is_conformance_bound)),
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
        let is_in_manifest = input.manifest.extension_contract_refs.contains(reference);
        let is_host_supports = input.host_supported_contract_refs.contains(reference);
        let contract = contract_for_ref(input.extension_contracts, reference);
        if is_in_manifest && is_host_supports && production_profile_ok(contract, input.production_profile) {
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
        let is_selectable = input.manifest.extension_contract_refs.contains(reference)
            && input.host_supported_contract_refs.contains(reference)
            && production_profile_ok(contract_for_ref(input.extension_contracts, reference), input.production_profile);
        if is_selectable {
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
    let is_retained_required = retained_required_contracts(input, &mut diagnostics)?;
    let is_compatible_versions = compatible_extension_versions(input, &mut diagnostics)?;
    let is_retained_hostcalls = retained_hostcall_descriptors(input, &mut diagnostics)?;
    let is_schema_compatible = schema_compatible(input, &mut diagnostics)?;
    let is_requirements_compatible = requirements_compatible(input, &mut diagnostics)?;
    let is_conformance_bound = compatibility_conformance_bound(input, &mut diagnostics)?;
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
            ("required-extensions-retained", status(is_retained_required)),
            ("version-compatible", status(is_compatible_versions)),
            ("hostcall-descriptors-retained", status(is_retained_hostcalls)),
            ("schema-compatible", status(is_schema_compatible)),
            ("authority-resource-effect-compatible", status(is_requirements_compatible)),
            ("rollback-cleanup-bound", status(!input.cleanup_refs.is_empty())),
            ("conformance-bound", status(is_conformance_bound)),
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
    let mut is_retained = true;
    for old_ref in &input.old_manifest.extension_contract_refs {
        let Some(old_contract) = contract_for_ref(input.old_contracts, old_ref) else {
            is_retained = false;
            diagnostics.push_limited(
                format!("old plugin extension contract {old_ref} is unavailable for compatibility"),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
            continue;
        };
        if matching_new_contract(input, old_contract).is_none() {
            is_retained = false;
            diagnostics.push_limited(
                format!("plugin extension upgrade removes required extension {}", old_contract.extension_id),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
        }
    }
    Ok(is_retained)
}

fn compatible_extension_versions(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let mut is_compatible = true;
    for old_ref in &input.old_manifest.extension_contract_refs {
        let Some(old_contract) = contract_for_ref(input.old_contracts, old_ref) else {
            is_compatible = false;
            continue;
        };
        let Some(new_contract) = matching_new_contract(input, old_contract) else {
            is_compatible = false;
            continue;
        };
        if !version_not_downgrade(&new_contract.version, &old_contract.version)? {
            is_compatible = false;
            diagnostics.push_limited(
                format!("plugin extension {} downgrades from {} to {}", old_contract.extension_id, old_contract.version, new_contract.version),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
        }
    }
    Ok(is_compatible)
}
