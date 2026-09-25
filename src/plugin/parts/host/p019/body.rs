
fn retained_hostcall_descriptors(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let mut is_retained = true;
    for old_ref in &input.old_manifest.extension_contract_refs {
        let Some(old_contract) = contract_for_ref(input.old_contracts, old_ref) else {
            is_retained = false;
            continue;
        };
        let Some(new_contract) = matching_new_contract(input, old_contract) else {
            is_retained = false;
            continue;
        };
        for old_descriptor in &old_contract.hostcall_descriptors {
            if find_descriptor(new_contract, &old_descriptor.operation, &old_descriptor.descriptor_ref).is_none()
                && input.migration_refs.is_empty()
            {
                is_retained = false;
                diagnostics.push_limited(
                    format!("plugin extension upgrade removes required hostcall {}", old_descriptor.operation),
                    MAX_PLUGIN_DIAGNOSTICS,
                    "plugin extension compatibility diagnostics",
                )?;
            }
        }
    }
    Ok(is_retained)
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
        for old_descriptor in &old_contract.hostcall_descriptors {
            if let Some(new_descriptor) = find_descriptor(new_contract, &old_descriptor.operation, &old_descriptor.descriptor_ref)
                && !predicate(old_descriptor, new_descriptor)
                && input.migration_refs.is_empty()
            {
                is_compatible = false;
                diagnostics.push_limited(
                    format!("{message}: {}", old_descriptor.operation),
                    MAX_PLUGIN_DIAGNOSTICS,
                    "plugin extension compatibility diagnostics",
                )?;
            }
        }
    }
    Ok(is_compatible)
}

fn compatibility_conformance_bound(
    input: &PluginExtensionCompatibilityInput<'_>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let mut is_bound = true;
    for new_ref in &input.new_manifest.extension_contract_refs {
        let Some(contract) = contract_for_ref(input.new_contracts, new_ref) else {
            is_bound = false;
            diagnostics.push_limited(
                format!("new plugin extension contract {new_ref} is unavailable for conformance"),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
            continue;
        };
        if input.production_profile && !contract.production_profile {
            is_bound = false;
            diagnostics.push_limited(
                format!("plugin extension contract {} lacks production conformance evidence", contract.extension_id),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin extension compatibility diagnostics",
            )?;
        }
    }
    Ok(is_bound)
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
