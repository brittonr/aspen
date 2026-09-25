
pub fn plugin_summary(value: &IoValue) -> Result<String> {
    if let Some(summary) = core_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = receipt_summary(value) {
        return Ok(summary);
    }
    if value.collect_simple_record("plugin-fixture-report-v1", Some(11)).is_some() {
        return Ok(format!("plugin fixture report ref={} (summary is non-normative)", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported plugin host artifact for summary"))
}

fn core_summary(value: &IoValue) -> Option<String> {
    if let Ok(grant) = parse_plugin_capability_grant(value) {
        return Some(format!(
            "plugin capability grant ref={} plugin={} operation={} revoked={} (summary is non-normative)",
            grant.grant_ref,
            grant.plugin_id,
            grant.operation,
            grant.revoked
        ));
    }
    if let Ok(manifest) = parse_plugin_manifest(value) {
        return Some(format!(
            "plugin manifest ref={} id={} artifact={} hostcalls={} lifecycle={} (summary is non-normative)",
            manifest.manifest_ref,
            manifest.plugin_id,
            manifest.artifact_ref,
            manifest.hostcall_refs.len(),
            manifest.lifecycle_callbacks.len()
        ));
    }
    if let Ok(install) = parse_plugin_install_receipt(value) {
        return Some(format!(
            "plugin install receipt ref={} decision={} manifest={} artifact={} diagnostics={} (summary is non-normative)",
            install.receipt_ref,
            install.decision,
            install.manifest_ref,
            install.artifact_ref,
            install.diagnostics.len()
        ));
    }
    if let Ok(permission) = parse_plugin_permission_receipt(value) {
        return Some(format!(
            "plugin permission receipt ref={} decision={} manifest={} diagnostics={} (summary is non-normative)",
            permission.receipt_ref,
            permission.decision,
            permission.manifest_ref,
            permission.diagnostics.len()
        ));
    }
    if let Ok(lifecycle) = parse_plugin_lifecycle_receipt(value) {
        return Some(format!(
            "plugin lifecycle receipt ref={} operation={} decision={} diagnostics={} (summary is non-normative)",
            lifecycle.receipt_ref,
            lifecycle.operation,
            lifecycle.decision,
            lifecycle.diagnostics.len()
        ));
    }
    None
}
