
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

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_u64(&fields[0], label)
}

fn required_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    ensure_count_at_most(items.len(), MAX_PLUGIN_REFS, label)?;
    let mut refs = Vec::new();
    for item in items.iter() {
        let reference = required_string(item, label)?;
        validate_ref(&reference, label)?;
        refs.push_limited(reference, MAX_PLUGIN_REFS, label)?;
    }
    Ok(refs)
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
    crate::preserves_rail::validate_boundary_schema(
        value,
        &crate::preserves_rail::PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA,
    )?;
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
    let is_production_profile = parse_profile(&fields[9])?;
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
        production_profile: is_production_profile,
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
