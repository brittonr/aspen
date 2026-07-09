
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
    let install_passes = plugin_install_passes(input.install, input.manifest, &mut diagnostics)?;
    let permission_passes = plugin_permission_passes(input.permission, input.manifest, &mut diagnostics)?;
    let activation_passes = plugin_activation_passes(input.activation, input.manifest, &mut diagnostics)?;
    let hostcall_passes = plugin_hostcall_passes(input.hostcall, input.manifest, &mut diagnostics)?;
    let health_passes = plugin_health_passes(input.health, input.manifest, input.recovery_receipt_ref, &mut diagnostics)?;
    let removal_passes = plugin_removal_passes(input.removal, input.manifest, &mut diagnostics)?;
    let upgrade_passes = plugin_upgrade_passes(input.upgrade, input.manifest, &mut diagnostics)?;
    let negotiation_passes = plugin_negotiation_passes(input.negotiation, input.manifest, &mut diagnostics)?;
    let compatibility_passes = plugin_compatibility_passes(input.compatibility, input.manifest, &mut diagnostics)?;

    if requires_permission(input.evaluation_kind) && !permission_passes {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_PERMISSION_MISSING.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if requires_activation(input.evaluation_kind) && !activation_passes {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_ACTIVATION_MISSING.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if requires_healthy_use(input.evaluation_kind) && !health_passes {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_HEALTH_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if requires_negotiation(input.evaluation_kind, input.manifest) && !negotiation_passes {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_NEGOTIATION_MISSING.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if requires_extension_compatibility(input.evaluation_kind, input.manifest) && !compatibility_passes {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_COMPATIBILITY_MISSING.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if matches!(input.evaluation_kind, PluginLifecycleEvaluationKind::HostcallRequest) && removal_passes {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_AUTHORITY_CLOSED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if matches!(input.evaluation_kind, PluginLifecycleEvaluationKind::UpgradeRequest) && removal_passes {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_AUTHORITY_CLOSED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }

    let guards = plugin_lifecycle_guard_snapshot(input, PluginLifecycleGuardBooleans {
        install_passes,
        permission_passes,
        activation_passes,
        hostcall_passes,
        health_passes,
        removal_passes,
        upgrade_passes,
        negotiation_passes,
        compatibility_passes,
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
    let binding_matches = install.plugin_ref == manifest.plugin_ref
        && install.manifest_ref == manifest.manifest_ref
        && install.artifact_ref == manifest.artifact_ref;
    if !binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_INSTALL_FAILED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(binding_matches)
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
    let binding_matches = permission.plugin_ref == manifest.plugin_ref && permission.manifest_ref == manifest.manifest_ref;
    if !binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_PERMISSION_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(binding_matches)
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
    let binding_matches = activation.plugin_ref == manifest.plugin_ref
        && activation.manifest_ref == manifest.manifest_ref
        && activation.operation == PLUGIN_LIFECYCLE_ACTIVATION_OPERATION;
    if !binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_ACTIVATION_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(binding_matches)
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
    let manifest_declares_hostcall = manifest.hostcall_refs.iter().any(|reference| reference == &hostcall.hostcall_ref)
        || !manifest.extension_contract_refs.is_empty();
    if !manifest_declares_hostcall {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_HOSTCALL_UNDECLARED.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
        return Ok(false);
    }
    Ok(true)
}

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
    let binding_matches = negotiation.manifest_ref == manifest.manifest_ref
        && contains_all(&negotiation.selected_contract_refs, &manifest.extension_contract_refs);
    if !binding_matches {
        diagnostics.push_limited(
            PLUGIN_LIFECYCLE_NEGOTIATION_BINDING_MISMATCH.to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    Ok(binding_matches)
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
    let mut sink = DiagnosticSink::new(diagnostics, MAX_PLUGIN_DIAGNOSTICS, "plugin permission diagnostics");
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

fn validate_optional_ref(value: Option<&str>, field: &str) -> Result<()> {
    if let Some(value) = value {
        validate_ref(value, field)
    } else {
        Ok(())
    }
}

fn validate_refs(values: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_REFS, field)?;
    for value in values {
        validate_ref(value, field)?;
    }
    Ok(())
}

fn require_non_empty_refs(values: &[String], field: &str) -> Result<()> {
    if values.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    validate_refs(values, field)
}

fn validate_diagnostics(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_DIAGNOSTICS, "plugin diagnostics")
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, maximum, label)
}

fn status(value: bool) -> &'static str {
    if value { PLUGIN_DECISION_PASS } else { PLUGIN_CHECK_FAIL }
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_text_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn simple_record_any<'a>(
    value: &'a IoValue,
    label: &str,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, None)
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))
}

fn record_arity(record: &preserves::Record<Value<IoValue>>) -> usize {
    record._vec().len().saturating_sub(1)
}

fn record_decision(value: &Value<IoValue>, label: &str) -> Result<String> {
    let decision = record_string(value, label)?;
    validate_decision(&decision)?;
    Ok(decision)
}

fn validate_decision(value: &str) -> Result<()> {
    match value {
        PLUGIN_DECISION_PASS | PLUGIN_DECISION_DENY => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("plugin receipt decision {value} must be pass or deny"))),
    }
}

fn require_check_status(checks: &[(String, String)], expected: &str, status: &str, context: &str) -> Result<()> {
    match checks.iter().find(|(name, _)| name == expected) {
        Some((_, actual)) if actual == status => Ok(()),
        Some((_, actual)) => Err(MoltenError::invalid_harness(format!(
            "{context} {expected} check has status {actual}, expected {status}"
        ))),
        None => Err(MoltenError::invalid_harness(format!("{context} missing {expected} check"))),
    }
}

fn validate_receipt_coherence(
    decision: &str,
    checks: &[(String, String)],
    diagnostics: &[String],
    context: &str,
) -> Result<()> {
    let has_failed_check = checks.iter().any(|(_, status)| status == PLUGIN_CHECK_FAIL);
    if decision == PLUGIN_DECISION_PASS && has_failed_check {
        return Err(MoltenError::invalid_harness(format!(
            "{context} pass decision carries failed required checks"
        )));
    }
    if decision == PLUGIN_DECISION_DENY && !has_failed_check && diagnostics.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "{context} deny decision requires failed checks or diagnostics"
        )));
    }
    Ok(())
}
