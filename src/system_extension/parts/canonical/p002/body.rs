
pub(crate) fn canonical_operator_status(status: OperatorStatus) -> crate::error::Result<CanonicalOperatorStatus> {
    if status.port_binding_refs.len() > MAX_CANONICAL_EXTENSION_ITEMS {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "system-extension status port binding count {} exceeds {}",
            status.port_binding_refs.len(),
            MAX_CANONICAL_EXTENSION_ITEMS
        )));
    }
    let value = crate::preserves_rail::record("system-extension-status-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_STATUS_SCHEMA),
        field("extension-id", crate::preserves_rail::string(&status.extension_id)),
        field("service-id", crate::preserves_rail::string(&status.service_id)),
        field("manifest-ref", crate::preserves_rail::string(&status.manifest_ref)),
        field("generation", crate::preserves_rail::u64_value(status.generation)),
        field("phase", crate::preserves_rail::string(status.phase.as_str())),
        field("execution-profile", crate::preserves_rail::string(status.execution_profile.as_str())),
        field("port-binding-refs", strings_value(status.port_binding_refs.iter().map(String::as_str))),
        field("resource-envelope", resource_envelope_value(&status.resources)),
        field("resource-usage", resource_usage_value(status.usage)),
        field("health", crate::preserves_rail::string(status.health.as_str())),
        field("restart-attempts", crate::preserves_rail::u64_value(status.restart_attempts)),
        field("checkpoint-ref", optional_string(status.checkpoint_ref.as_deref())),
        field("last-lifecycle-ref", optional_string(status.last_lifecycle_ref.as_deref())),
        field("invocation-count", crate::preserves_rail::u64_value(status.invocation_count)),
        checks_value(&[
            "bounded-operator-readback",
            "active-generation-visible",
            "profile-and-ports-visible",
            "secret-material-excluded",
            "status-is-not-behavioral-proof",
        ]),
    ]);
    let status_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalOperatorStatus {
        status_ref,
        status,
        value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorStatusReadback {
    pub extension_id: String,
    pub service_id: String,
    pub manifest_ref: String,
    pub generation: u64,
    pub phase: String,
    pub execution_profile: String,
    pub health: String,
    pub restart_attempts: u64,
    pub checkpoint_ref: Option<String>,
    pub invocation_count: u64,
    pub status_ref: String,
}

const READBACK_PHASES: [&str; 19] = [
    "absent",
    "installed",
    "admitted",
    "initializing",
    "initialized",
    "starting",
    "running",
    "checkpointing",
    "recovering",
    "draining",
    "drained",
    "failed",
    "restarting",
    "upgrading",
    "rolling-back",
    "shutting-down",
    "quarantined",
    "stopped",
    "removed",
];

const READBACK_EXECUTION_PROFILES: [&str; 3] = ["in-process-native", "native-process", "sandboxed-component"];

const READBACK_HEALTH_STATES: [&str; 7] = [
    "unknown",
    "starting",
    "healthy",
    "degraded",
    "failed",
    "quarantined",
    "stopped",
];

// r[impl molten.system_extension.operator_readback]
pub fn parse_operator_status_readback(value: &preserves::IOValue) -> crate::error::Result<OperatorStatusReadback> {
    const STATUS_FIELD_COUNT: usize = 16;
    let fields = value
        .collect_simple_record("system-extension-status-v1", Some(STATUS_FIELD_COUNT))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness("expected canonical system-extension status"))?;
    let schema = required_string(&fields[0], "status schema")?;
    if schema != super::SYSTEM_EXTENSION_STATUS_SCHEMA {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "system-extension status schema mismatch: {schema}"
        )));
    }
    let extension_id = record_string_field(&fields[1], "extension-id")?;
    let service_id = record_string_field(&fields[2], "service-id")?;
    let manifest_ref = record_string_field(&fields[3], "manifest-ref")?;
    let phase = record_string_field(&fields[5], "phase")?;
    let execution_profile = record_string_field(&fields[6], "execution-profile")?;
    let health = record_string_field(&fields[10], "health")?;
    let checkpoint_ref = record_optional_string_field(&fields[12], "checkpoint-ref")?;
    validate_readback_identifier(&extension_id, "extension-id")?;
    validate_readback_identifier(&service_id, "service-id")?;
    crate::preserves_rail::validate_content_ref(&manifest_ref)?;
    if let Some(checkpoint_ref) = &checkpoint_ref {
        crate::preserves_rail::validate_content_ref(checkpoint_ref)?;
    }
    validate_readback_enum(&phase, "phase", &READBACK_PHASES)?;
    validate_readback_enum(&execution_profile, "execution-profile", &READBACK_EXECUTION_PROFILES)?;
    validate_readback_enum(&health, "health", &READBACK_HEALTH_STATES)?;
    Ok(OperatorStatusReadback {
        extension_id,
        service_id,
        manifest_ref,
        generation: record_u64_field(&fields[4], "generation")?,
        phase,
        execution_profile,
        health,
        restart_attempts: record_u64_field(&fields[11], "restart-attempts")?,
        checkpoint_ref,
        invocation_count: record_u64_field(&fields[14], "invocation-count")?,
        status_ref: crate::preserves_rail::canonical_hash(value)?,
    })
}

fn validate_readback_identifier(value: &str, label: &str) -> crate::error::Result<()> {
    if value.len() > MAX_READBACK_IDENTIFIER_BYTES {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "system-extension {label} exceeds {MAX_READBACK_IDENTIFIER_BYTES} bytes"
        )));
    }
    crate::preserves_rail::validate_stable_id(value, label)
}

fn validate_readback_enum(value: &str, label: &str, allowed: &[&str]) -> crate::error::Result<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(crate::error::MoltenError::invalid_harness(format!("unsupported system-extension {label}: {value}")))
    }
}

fn record_string_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} STRING>")))?;
    required_string(&fields[0], label)
}

fn record_u64_field(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<u64> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} U64>")))?;
    fields[0]
        .as_u64()
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected u64 for {label}")))?
        .map_err(|error| crate::error::MoltenError::invalid_harness(format!("u64 out of range for {label}: {error}")))
}

fn record_optional_string_field(
    value: &preserves::Value<preserves::IOValue>,
    label: &str,
) -> crate::error::Result<Option<String>> {
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected <{label} OPTION>")))?;
    if fields[0].collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    let some = fields[0]
        .collect_simple_record("some", Some(1))
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected optional string for {label}")))?;
    required_string(&some[0], label).map(Some)
}

fn required_string(value: &preserves::Value<preserves::IOValue>, label: &str) -> crate::error::Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| crate::error::MoltenError::invalid_harness(format!("expected string for {label}")))
}

fn effect_value(effect: &super::TypedEffectRequest) -> preserves::IOValue {
    let target = match &effect.target {
        super::EffectTarget::FabricPort(key) => port_key_value(key),
        super::EffectTarget::Ambient(ambient) => {
            crate::preserves_rail::record("ambient-effect", vec![crate::preserves_rail::string(ambient.as_str())])
        }
    };
    crate::preserves_rail::record("system-extension-typed-effect-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_TYPED_EFFECT_SCHEMA),
        field("target", target),
        field("operation", crate::preserves_rail::string(&effect.operation)),
        field("input-schema-ref", crate::preserves_rail::string(&effect.input_schema_ref)),
        field("output-schema-ref", crate::preserves_rail::string(&effect.output_schema_ref)),
        field("request-ref", crate::preserves_rail::string(&effect.request_ref)),
        field("generation", crate::preserves_rail::u64_value(effect.generation)),
        field("accounted-bytes", crate::preserves_rail::u64_value(effect.accounted_bytes)),
    ])
}

fn port_key_value(key: &crate::fabric::FabricPortKey) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-port-key", vec![
        crate::preserves_rail::string(&key.port_id),
        crate::preserves_rail::string(&key.version),
    ])
}

fn resource_envelope_value(resources: &super::ResourceEnvelope) -> preserves::IOValue {
    crate::preserves_rail::record("resource-envelope-v1", vec![
        field("max-concurrent-callbacks", crate::preserves_rail::u64_value(resources.max_concurrent_callbacks)),
        field("max-queued-events", crate::preserves_rail::u64_value(resources.max_queued_events)),
        field("max-inflight-bytes", crate::preserves_rail::u64_value(resources.max_inflight_bytes)),
        field("max-open-streams", crate::preserves_rail::u64_value(resources.max_open_streams)),
        field("max-timers", crate::preserves_rail::u64_value(resources.max_timers)),
        field("max-effect-requests", crate::preserves_rail::u64_value(resources.max_effect_requests)),
        field("callback-deadline-ticks", crate::preserves_rail::u64_value(resources.callback_deadline_ticks)),
        field("shutdown-grace-ticks", crate::preserves_rail::u64_value(resources.shutdown_grace_ticks)),
        field("max-restart-attempts", crate::preserves_rail::u64_value(resources.max_restart_attempts)),
        field("overload-policy", crate::preserves_rail::string(resources.overload_policy.as_str())),
    ])
}

fn resource_usage_value(usage: super::ResourceUsage) -> preserves::IOValue {
    crate::preserves_rail::record("resource-usage-v1", vec![
        field("concurrent-callbacks", crate::preserves_rail::u64_value(usage.concurrent_callbacks)),
        field("queued-events", crate::preserves_rail::u64_value(usage.queued_events)),
        field("inflight-bytes", crate::preserves_rail::u64_value(usage.inflight_bytes)),
        field("open-streams", crate::preserves_rail::u64_value(usage.open_streams)),
        field("timers", crate::preserves_rail::u64_value(usage.timers)),
        field("effect-requests", crate::preserves_rail::u64_value(usage.effect_requests)),
    ])
}

fn optional_failure(failure: Option<super::FailureClass>) -> preserves::IOValue {
    optional_string(failure.map(|failure| failure.as_str()))
}

fn optional_string(value: Option<&str>) -> preserves::IOValue {
    match value {
        Some(value) => crate::preserves_rail::record("some", vec![crate::preserves_rail::string(value)]),
        None => crate::preserves_rail::record("none", Vec::new()),
    }
}

fn optional_native_callback_value(value: Option<&super::NativeCallbackValue>) -> preserves::IOValue {
    value.map_or_else(
        || crate::preserves_rail::record("none", Vec::new()),
        |value| {
            crate::preserves_rail::record("some", vec![crate::preserves_rail::record(
                "native-callback-value-v2",
                vec![
                    crate::preserves_rail::string(&value.value_ref),
                    crate::preserves_rail::sequence(
                        value.bytes.iter().map(|byte| crate::preserves_rail::u64_value(u64::from(*byte))).collect(),
                    ),
                ],
            )])
        },
    )
}

fn field(label: &'static str, value: preserves::IOValue) -> preserves::IOValue {
    crate::preserves_rail::record(label, vec![value])
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> preserves::IOValue {
    crate::preserves_rail::sequence(values.into_iter().map(crate::preserves_rail::string).collect())
}

fn checks_value(checks: &[&str]) -> preserves::IOValue {
    field("checks", strings_value(checks.iter().copied()))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> crate::error::MoltenError {
    crate::error::MoltenError::invalid_harness(format!("{label} validation denied: {issues:?}"))
}
