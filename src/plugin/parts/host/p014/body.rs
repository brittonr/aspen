
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
