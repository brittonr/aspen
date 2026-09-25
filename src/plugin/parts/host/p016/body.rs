
fn plugin_health_passes(
    health: Option<&PluginHealthReceipt>,
    manifest: &PluginManifest,
    recovery_receipt_ref: Option<&str>,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let Some(health) = health else {
        return Ok(true);
    };
    if health.manifest_ref != manifest.manifest_ref {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_HEALTH_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    if health.decision == PLUGIN_DECISION_PASS || recovery_receipt_ref.is_some() {
        Ok(true)
    } else {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_HEALTH_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        Ok(false)
    }
}

fn plugin_removal_passes(
    removal: Option<&PluginRemovalReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let Some(removal) = removal else {
        return Ok(false);
    };
    if removal.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_REMOVAL_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    if removal.plugin_ref != manifest.plugin_ref || removal.manifest_ref != manifest.manifest_ref {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_REMOVAL_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    Ok(true)
}

fn plugin_upgrade_passes(
    upgrade: Option<&PluginUpgradeReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    let Some(upgrade) = upgrade else {
        return Ok(false);
    };
    if upgrade.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_UPGRADE_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    if upgrade.old_manifest_ref != manifest.manifest_ref {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_UPGRADE_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    Ok(true)
}

fn plugin_negotiation_passes(
    negotiation: Option<&PluginExtensionNegotiationReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    if manifest.extension_contract_refs.is_empty() {
        return Ok(true);
    }
    let Some(negotiation) = negotiation else {
        return Ok(false);
    };
    if negotiation.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_NEGOTIATION_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    let is_binding_matches = negotiation.manifest_ref == manifest.manifest_ref
        && contains_all(&negotiation.selected_contract_refs, &manifest.extension_contract_refs);
    if !is_binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_NEGOTIATION_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(is_binding_matches)
}

fn plugin_compatibility_passes(
    compatibility: Option<&PluginExtensionCompatibilityReceipt>,
    manifest: &PluginManifest,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<bool> {
    if manifest.extension_contract_refs.is_empty() {
        return Ok(true);
    }
    let Some(compatibility) = compatibility else {
        return Ok(false);
    };
    if compatibility.decision != PLUGIN_DECISION_PASS {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_COMPATIBILITY_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    if compatibility.old_manifest_ref != manifest.manifest_ref {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_COMPATIBILITY_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    Ok(true)
}

fn requires_permission(evaluation_kind: PluginLifecycleEvaluationKind) -> bool {
    matches!(
        evaluation_kind,
        PluginLifecycleEvaluationKind::ActivationRequest
            | PluginLifecycleEvaluationKind::HostcallRequest
            | PluginLifecycleEvaluationKind::UpgradeRequest
            | PluginLifecycleEvaluationKind::RemovalRequest
            | PluginLifecycleEvaluationKind::CompleteTrace
    )
}

fn requires_activation(evaluation_kind: PluginLifecycleEvaluationKind) -> bool {
    matches!(
        evaluation_kind,
        PluginLifecycleEvaluationKind::HostcallRequest
            | PluginLifecycleEvaluationKind::UpgradeRequest
            | PluginLifecycleEvaluationKind::RemovalRequest
            | PluginLifecycleEvaluationKind::CompleteTrace
    )
}

fn requires_healthy_use(evaluation_kind: PluginLifecycleEvaluationKind) -> bool {
    matches!(
        evaluation_kind,
        PluginLifecycleEvaluationKind::ActivationRequest
            | PluginLifecycleEvaluationKind::HostcallRequest
            | PluginLifecycleEvaluationKind::UpgradeRequest
    )
}

fn requires_negotiation(evaluation_kind: PluginLifecycleEvaluationKind, manifest: &PluginManifest) -> bool {
    !manifest.extension_contract_refs.is_empty()
        && matches!(
            evaluation_kind,
            PluginLifecycleEvaluationKind::ActivationRequest
                | PluginLifecycleEvaluationKind::HostcallRequest
                | PluginLifecycleEvaluationKind::CompleteTrace
        )
}

fn requires_extension_compatibility(evaluation_kind: PluginLifecycleEvaluationKind, manifest: &PluginManifest) -> bool {
    !manifest.extension_contract_refs.is_empty() && matches!(evaluation_kind, PluginLifecycleEvaluationKind::UpgradeRequest)
}

fn collect_missing_refs(
    required_refs: &[String],
    supplied_refs: &[String],
    label: &str,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    let mut sink = crate::bounded::DiagnosticSink::new(diagnostics, MAX_PLUGIN_DIAGNOSTICS, "plugin permission diagnostics");
    for value in required_refs {
        if !supplied_refs.contains(value) {
            sink.push(format!("plugin missing current {label} ref {value}"))?;
        }
    }
    Ok(())
}

fn contains_all(supplied_refs: &[String], required_refs: &[String]) -> bool {
    required_refs.iter().all(|required| supplied_refs.contains(required))
}

fn is_lifecycle_declared(callbacks: &[String], operation: &str) -> bool {
    callbacks.iter().any(|callback| callback == operation)
}

fn is_ambient_operation(operation: &str) -> bool {
    ["network", "filesystem", "env", "clock", "process", "node-control"]
        .iter()
        .any(|prefix| operation == *prefix || operation.starts_with(&format!("{prefix}.")))
}

fn validate_plugin_id(value: &str) -> Result<()> {
    validate_non_empty(value, "plugin id")?;
    if !value.starts_with("plugin:") {
        return Err(MoltenError::invalid_harness(format!("plugin id {value} must start with plugin:")));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_' | '.'))
    {
        return Err(MoltenError::invalid_harness(format!("unsupported plugin id {value}")));
    }
    Ok(())
}

fn validate_abi(value: &str) -> Result<()> {
    if value == PLUGIN_HOST_ABI_VERSION {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported plugin ABI {value}; expected {PLUGIN_HOST_ABI_VERSION}"
        )))
    }
}

fn validate_lifecycle_operation(value: &str) -> Result<()> {
    match value {
        "init" | "start" | "health" | "stop" | "remove" | "upgrade" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported plugin lifecycle operation {value}"))),
    }
}

fn validate_lifecycle_callbacks(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_CALLBACKS, "plugin lifecycle callbacks")?;
    if values.is_empty() {
        return Err(MoltenError::invalid_harness("plugin lifecycle callbacks must not be empty"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        validate_lifecycle_operation(value)?;
        if !seen.insert(value.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate plugin lifecycle callback {value}")));
        }
    }
    Ok(())
}

fn validate_health_status(value: &str) -> Result<()> {
    match value {
        "healthy" | "degraded" | "failed" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported plugin health status {value}"))),
    }
}

fn validate_host_abi_status(value: &str) -> Result<()> {
    match value {
        "ok" | "error" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported plugin ABI result status {value}"))),
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{field} must be a canonical content ref: {error}")))
}
