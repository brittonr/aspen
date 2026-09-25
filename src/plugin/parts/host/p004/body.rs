
fn receipt_summary(value: &IoValue) -> Option<String> {
    if let Ok(hostcall) = parse_plugin_hostcall_receipt(value) {
        return Some(format!(
            "plugin hostcall receipt ref={} manifest={} operation={} decision={} diagnostics={} (summary is non-normative)",
            hostcall.receipt_ref,
            hostcall.manifest_ref,
            hostcall.operation,
            hostcall.decision,
            hostcall.diagnostics.len()
        ));
    }
    if let Ok(health) = parse_plugin_health_receipt(value) {
        return Some(format!(
            "plugin health receipt ref={} manifest={} decision={} diagnostics={} (summary is non-normative)",
            health.receipt_ref,
            health.manifest_ref,
            health.decision,
            health.diagnostics.len()
        ));
    }
    if let Ok(removal) = parse_plugin_removal_receipt(value) {
        return Some(format!(
            "plugin removal receipt ref={} manifest={} decision={} diagnostics={} (summary is non-normative)",
            removal.receipt_ref,
            removal.manifest_ref,
            removal.decision,
            removal.diagnostics.len()
        ));
    }
    if let Ok(upgrade) = parse_plugin_upgrade_receipt(value) {
        return Some(format!(
            "plugin upgrade receipt ref={} decision={} old={} new={} diagnostics={} (summary is non-normative)",
            upgrade.receipt_ref,
            upgrade.decision,
            upgrade.old_manifest_ref,
            upgrade.new_manifest_ref,
            upgrade.diagnostics.len()
        ));
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PluginFixtureReportInput<'a> {
    manifest_ref: &'a str,
    install_receipt_ref: &'a str,
    permission_receipt_ref: &'a str,
    start_receipt_ref: &'a str,
    hostcall_receipt_ref: &'a str,
    health_receipt_ref: &'a str,
    stop_receipt_ref: &'a str,
    removal_receipt_ref: &'a str,
    upgrade_receipt_ref: &'a str,
}

fn plugin_fixture_report_value(input: &PluginFixtureReportInput<'_>) -> Result<IoValue> {
    let refs = [
        input.manifest_ref,
        input.install_receipt_ref,
        input.permission_receipt_ref,
        input.start_receipt_ref,
        input.hostcall_receipt_ref,
        input.health_receipt_ref,
        input.stop_receipt_ref,
        input.removal_receipt_ref,
        input.upgrade_receipt_ref,
    ];
    for value in refs {
        validate_ref(value, "plugin fixture report ref")?;
    }
    Ok(record("plugin-fixture-report-v1", vec![
        string("molten.plugin.fixture-report.v1"),
        record("decision", vec![string("pass")]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("install", vec![string(input.install_receipt_ref)]),
        record("permission", vec![string(input.permission_receipt_ref)]),
        record("start", vec![string(input.start_receipt_ref)]),
        record("hostcall", vec![string(input.hostcall_receipt_ref)]),
        record("health", vec![string(input.health_receipt_ref)]),
        record("stop", vec![string(input.stop_receipt_ref)]),
        record("removal", vec![string(input.removal_receipt_ref)]),
        record("upgrade", vec![string(input.upgrade_receipt_ref)]),
    ]))
}

pub fn storage_read_hostcall_ref() -> Result<String> {
    primitive_hostcall_ref("storage.read")
}

pub fn network_open_hostcall_ref() -> Result<String> {
    primitive_hostcall_ref("network.open")
}

fn primitive_hostcall_ref(operation: &str) -> Result<String> {
    canonical_hash(&record("plugin-hostcall", vec![string(operation)]))
}

fn plugin_ref(label: &str) -> Result<String> {
    canonical_hash(&record("plugin-ref", vec![string(label)]))
}

fn plugin_identity_ref(plugin_id: &str, artifact_ref: &str) -> Result<String> {
    canonical_hash(&record("plugin-identity-v1", vec![string(plugin_id), string(artifact_ref)]))
}

fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub fn evaluate_plugin_lifecycle_state(input: &PluginLifecycleStateInput<'_>) -> Result<PluginLifecycleStateDecision> {
    validate_optional_ref(input.recovery_receipt_ref, "plugin lifecycle recovery receipt ref")?;
    let mut diagnostics = Vec::new();
    let is_install_passes = plugin_install_passes(input.install, input.manifest, &mut diagnostics)?;
    let is_permission_passes = plugin_permission_passes(input.permission, input.manifest, &mut diagnostics)?;
    let is_activation_passes = plugin_activation_passes(input.activation, input.manifest, &mut diagnostics)?;
    let is_hostcall_passes = plugin_hostcall_passes(input.hostcall, input.manifest, &mut diagnostics)?;
    let is_health_passes = plugin_health_passes(input.health, input.manifest, input.recovery_receipt_ref, &mut diagnostics)?;
    let is_removal_passes = plugin_removal_passes(input.removal, input.manifest, &mut diagnostics)?;
    let is_upgrade_passes = plugin_upgrade_passes(input.upgrade, input.manifest, &mut diagnostics)?;
    let is_negotiation_passes = plugin_negotiation_passes(input.negotiation, input.manifest, &mut diagnostics)?;
    let is_compatibility_passes = plugin_compatibility_passes(input.compatibility, input.manifest, &mut diagnostics)?;

    let lifecycle_gaps = [
        (requires_permission(input.evaluation_kind) && !is_permission_passes, PLUGIN_LIFECYCLE_PERMISSION_MISSING),
        (requires_activation(input.evaluation_kind) && !is_activation_passes, PLUGIN_LIFECYCLE_ACTIVATION_MISSING),
        (requires_healthy_use(input.evaluation_kind) && !is_health_passes, PLUGIN_LIFECYCLE_HEALTH_FAILED),
        (requires_negotiation(input.evaluation_kind, input.manifest) && !is_negotiation_passes, PLUGIN_LIFECYCLE_NEGOTIATION_MISSING),
        (requires_extension_compatibility(input.evaluation_kind, input.manifest) && !is_compatibility_passes, PLUGIN_LIFECYCLE_COMPATIBILITY_MISSING),
        (matches!(input.evaluation_kind, PluginLifecycleEvaluationKind::HostcallRequest) && is_removal_passes, PLUGIN_LIFECYCLE_AUTHORITY_CLOSED),
        (matches!(input.evaluation_kind, PluginLifecycleEvaluationKind::UpgradeRequest) && is_removal_passes, PLUGIN_LIFECYCLE_AUTHORITY_CLOSED),
    ];
    for (is_gap, diagnostic) in lifecycle_gaps {
        if is_gap {
            diagnostics.push_limited(diagnostic.to_string(), MAX_PLUGIN_DIAGNOSTICS, "plugin lifecycle diagnostics")?;
        }
    }

    let guards = plugin_lifecycle_guard_snapshot(input, PluginLifecycleGuardBooleans {
        install_passes: is_install_passes,
        permission_passes: is_permission_passes,
        activation_passes: is_activation_passes,
        hostcall_passes: is_hostcall_passes,
        health_passes: is_health_passes,
        removal_passes: is_removal_passes,
        upgrade_passes: is_upgrade_passes,
        negotiation_passes: is_negotiation_passes,
        compatibility_passes: is_compatibility_passes,
    });
    plugin_lifecycle_transition_decision(input.evaluation_kind, &input.manifest.manifest_ref, guards, diagnostics)
}

fn plugin_install_passes(
    install: Option<&PluginInstallReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let Some(install) = install else {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_INSTALL_MISSING.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    };
    if install.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_INSTALL_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    let is_binding_matches = install.plugin_ref == manifest.plugin_ref
        && install.manifest_ref == manifest.manifest_ref
        && install.artifact_ref == manifest.artifact_ref;
    if !is_binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_INSTALL_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(is_binding_matches)
}

fn plugin_permission_passes(
    permission: Option<&PluginPermissionReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let Some(permission) = permission else {
        return Ok(false);
    };
    if permission.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_PERMISSION_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    let is_binding_matches = permission.plugin_ref == manifest.plugin_ref && permission.manifest_ref == manifest.manifest_ref;
    if !is_binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_PERMISSION_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(is_binding_matches)
}

fn plugin_activation_passes(
    activation: Option<&PluginLifecycleReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let Some(activation) = activation else {
        return Ok(false);
    };
    if activation.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_ACTIVATION_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    let is_binding_matches = activation.plugin_ref == manifest.plugin_ref
        && activation.manifest_ref == manifest.manifest_ref
        && activation.operation == PLUGIN_LIFECYCLE_ACTIVATION_OPERATION;
    if !is_binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_ACTIVATION_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(is_binding_matches)
}

fn plugin_hostcall_passes(
    hostcall: Option<&PluginHostcallReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let Some(hostcall) = hostcall else {
        return Ok(false);
    };
    if hostcall.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_HOSTCALL_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    if hostcall.plugin_ref != manifest.plugin_ref || hostcall.manifest_ref != manifest.manifest_ref {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_HOSTCALL_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    let is_manifest_declares_hostcall = manifest.hostcall_refs.iter().any(|reference| reference == &hostcall.hostcall_ref)
        || !manifest.extension_contract_refs.is_empty();
    if !is_manifest_declares_hostcall {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_HOSTCALL_UNDECLARED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    Ok(true)
}
