
pub fn plugin_manifest_value(input: &PluginManifestInput<'_>) -> Result<IoValue> {
    validate_plugin_id(input.plugin_id)?;
    validate_ref(input.artifact_ref, "plugin artifact ref")?;
    validate_abi(input.abi)?;
    validate_lifecycle_callbacks(input.lifecycle_callbacks)?;
    require_non_empty_refs(input.effect_manifest_refs, "plugin effect manifest refs")?;
    require_non_empty_refs(input.hostcall_refs, "plugin hostcall refs")?;
    require_non_empty_refs(input.schema_refs, "plugin schema refs")?;
    require_non_empty_refs(input.policy_refs, "plugin policy refs")?;
    require_non_empty_refs(input.resource_refs, "plugin resource refs")?;
    require_non_empty_refs(input.supply_chain_refs, "plugin supply-chain refs")?;
    validate_refs(input.extension_contract_refs, "plugin extension contract refs")?;
    Ok(record("plugin-manifest-v1", vec![
        string(crate::preserves_rail::PLUGIN_MANIFEST_SCHEMA),
        record("plugin-id", vec![string(input.plugin_id)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("abi", vec![string(input.abi)]),
        record("lifecycle", vec![strings_sequence(input.lifecycle_callbacks)]),
        record("effects", vec![refs_sequence(input.effect_manifest_refs)]),
        record("hostcalls", vec![refs_sequence(input.hostcall_refs)]),
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("supply-chain", vec![refs_sequence(input.supply_chain_refs)]),
        record("extension-contracts", vec![refs_sequence(input.extension_contract_refs)]),
        checks_value(&[
            ("artifact-backed", PLUGIN_DECISION_PASS),
            ("host-abi-version", PLUGIN_DECISION_PASS),
            ("declared-lifecycle", PLUGIN_DECISION_PASS),
            ("declared-effects", PLUGIN_DECISION_PASS),
            ("declared-hostcalls", PLUGIN_DECISION_PASS),
            ("extension-contracts-bound", PLUGIN_DECISION_PASS),
            ("explicit-policy", PLUGIN_DECISION_PASS),
            ("explicit-resource", PLUGIN_DECISION_PASS),
            ("supply-chain-bound", PLUGIN_DECISION_PASS),
            ("no-ambient-authority", PLUGIN_DECISION_PASS),
        ]),
    ]))
}
