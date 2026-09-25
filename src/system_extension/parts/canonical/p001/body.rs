
pub(crate) fn canonical_callback_receipt(
    input: CallbackReceiptInput<'_>,
) -> crate::error::Result<CanonicalCallbackReceipt> {
    let approved_effects = if input.decision == CallbackExecutionDecision::Succeeded {
        input.outcome.map_or_else(Vec::new, |outcome| outcome.effects.clone())
    } else {
        Vec::new()
    };
    let effect_values = approved_effects.iter().map(effect_value).collect::<Vec<_>>();
    let mut effect_refs = Vec::with_capacity(effect_values.len());
    for value in &effect_values {
        effect_refs.push(crate::preserves_rail::canonical_hash(value)?);
    }
    let output_refs = input.outcome.map_or(&[][..], |outcome| outcome.output_refs.as_slice());
    let execution_binding_value = crate::preserves_rail::record("system-extension-execution-binding-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_EXECUTION_BINDING_SCHEMA),
        field("manifest-ref", crate::preserves_rail::string(input.manifest_ref)),
        field("execution-profile", crate::preserves_rail::string(input.execution_profile.as_str())),
        field("callback", crate::preserves_rail::string(input.invocation.callback.as_str())),
        field("generation", crate::preserves_rail::u64_value(input.invocation.generation)),
        field("sequence", crate::preserves_rail::u64_value(input.invocation.sequence)),
        field("event-ref", crate::preserves_rail::string(&input.invocation.event_ref)),
        field("decision", crate::preserves_rail::string(input.decision.as_str())),
        field("output-refs", strings_value(output_refs.iter().map(String::as_str))),
        field("effect-refs", strings_value(effect_refs.iter().map(String::as_str))),
        checks_value(&[
            "executor-invoked",
            "profile-matched",
            "generation-matched",
            "outcome-validated-before-effect-release",
        ]),
    ]);
    let execution_binding_ref = crate::preserves_rail::canonical_hash(&execution_binding_value)?;
    let value = crate::preserves_rail::record("system-extension-callback-receipt-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_CALLBACK_SCHEMA),
        field("manifest-ref", crate::preserves_rail::string(input.manifest_ref)),
        field("extension-id", crate::preserves_rail::string(input.extension_id)),
        field("service-id", crate::preserves_rail::string(input.service_id)),
        field("callback", crate::preserves_rail::string(input.invocation.callback.as_str())),
        field("generation", crate::preserves_rail::u64_value(input.invocation.generation)),
        field("sequence", crate::preserves_rail::u64_value(input.invocation.sequence)),
        field("event-ref", crate::preserves_rail::string(&input.invocation.event_ref)),
        field("execution-binding-ref", crate::preserves_rail::string(&execution_binding_ref)),
        field("decision", crate::preserves_rail::string(input.decision.as_str())),
        field("output-refs", strings_value(output_refs.iter().map(String::as_str))),
        field("effect-refs", strings_value(effect_refs.iter().map(String::as_str))),
        field("state-ref", optional_string(input.outcome.and_then(|outcome| outcome.state_ref.as_deref()))),
        field(
            "checkpoint-ref",
            optional_string(input.outcome.and_then(|outcome| outcome.checkpoint_ref.as_deref())),
        ),
        field("diagnostic", optional_string(input.diagnostic)),
        checks_value(&[
            "real-callback-execution-bound",
            "typed-effects-only",
            "per-event-evidence-profile",
            "success-is-not-semantic-correctness-proof",
        ]),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalCallbackReceipt {
        receipt_ref,
        execution_binding_ref,
        invocation: input.invocation.clone(),
        decision: input.decision,
        approved_effects,
        value,
    })
}

// r[impl molten.system_extension.native_host.effect_completion_value]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortEffectOutput {
    pub output_schema_ref: String,
    pub output_ref: String,
    pub materialized_output: Option<super::NativeCallbackValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalEffectCompletion {
    pub completion_ref: String,
    pub callback_receipt_ref: String,
    pub binding_ref: String,
    pub request_ref: String,
    pub generation: u64,
    pub output_ref: String,
    pub materialized_output: Option<super::NativeCallbackValue>,
    pub value: preserves::IOValue,
}

pub(crate) fn canonical_effect_completion(
    callback_receipt_ref: &str,
    binding: &crate::fabric::CanonicalFabricPortBinding,
    effect: &super::TypedEffectRequest,
    output: &PortEffectOutput,
) -> crate::error::Result<CanonicalEffectCompletion> {
    crate::preserves_rail::validate_content_ref(&output.output_ref)?;
    if output.output_schema_ref != effect.output_schema_ref {
        return Err(crate::error::MoltenError::invalid_harness(format!(
            "system-extension effect completion schema mismatch: actual={} expected={}",
            output.output_schema_ref, effect.output_schema_ref
        )));
    }
    if let Some(materialized) = &output.materialized_output {
        crate::preserves_rail::validate_content_ref(&materialized.value_ref)?;
        let observed_ref = crate::preserves_rail::content_ref_from_bytes(&materialized.bytes);
        if materialized.value_ref != output.output_ref || observed_ref != materialized.value_ref {
            return Err(crate::error::MoltenError::invalid_harness(
                "system-extension materialized effect output identity mismatch",
            ));
        }
    }
    let value = crate::preserves_rail::record("system-extension-effect-completion-v2", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_EFFECT_COMPLETION_SCHEMA),
        field("callback-receipt-ref", crate::preserves_rail::string(callback_receipt_ref)),
        field("binding-ref", crate::preserves_rail::string(&binding.binding_ref)),
        field("port-id", crate::preserves_rail::string(&binding.binding.key.port_id)),
        field("port-version", crate::preserves_rail::string(&binding.binding.key.version)),
        field("request-ref", crate::preserves_rail::string(&effect.request_ref)),
        field("generation", crate::preserves_rail::u64_value(effect.generation)),
        field("output-schema-ref", crate::preserves_rail::string(&output.output_schema_ref)),
        field("output-ref", crate::preserves_rail::string(&output.output_ref)),
        field("output-value", optional_native_callback_value(output.materialized_output.as_ref())),
        checks_value(&[
            "exact-bound-port-routed",
            "generation-correlated",
            "output-schema-matched",
            "materialized-output-identity-checked",
            "completion-is-not-durability-proof",
        ]),
    ]);
    let completion_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalEffectCompletion {
        completion_ref,
        callback_receipt_ref: callback_receipt_ref.to_string(),
        binding_ref: binding.binding_ref.clone(),
        request_ref: effect.request_ref.clone(),
        generation: effect.generation,
        output_ref: output.output_ref.clone(),
        materialized_output: output.materialized_output.clone(),
        value,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationOperation {
    Upgrade,
    Rollback,
}

impl MigrationOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Upgrade => "upgrade",
            Self::Rollback => "rollback",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalStateMigrationReceipt {
    pub receipt_ref: String,
    pub operation: MigrationOperation,
    pub source_schema: String,
    pub target_schema: String,
    pub value: preserves::IOValue,
}

pub(crate) struct StateMigrationReceiptInput<'a> {
    pub operation: MigrationOperation,
    pub extension_id: &'a str,
    pub service_id: &'a str,
    pub previous_manifest_ref: &'a str,
    pub next_manifest_ref: &'a str,
    pub source_schema: &'a str,
    pub target_schema: &'a str,
    pub checkpoint_ref: &'a str,
    pub generation: u64,
}

pub(crate) fn canonical_state_migration_receipt(
    input: StateMigrationReceiptInput<'_>,
) -> crate::error::Result<CanonicalStateMigrationReceipt> {
    let value = crate::preserves_rail::record("system-extension-state-migration-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_STATE_MIGRATION_SCHEMA),
        field("operation", crate::preserves_rail::string(input.operation.as_str())),
        field("extension-id", crate::preserves_rail::string(input.extension_id)),
        field("service-id", crate::preserves_rail::string(input.service_id)),
        field("previous-manifest-ref", crate::preserves_rail::string(input.previous_manifest_ref)),
        field("next-manifest-ref", crate::preserves_rail::string(input.next_manifest_ref)),
        field("source-schema", crate::preserves_rail::string(input.source_schema)),
        field("target-schema", crate::preserves_rail::string(input.target_schema)),
        field("checkpoint-ref", crate::preserves_rail::string(input.checkpoint_ref)),
        field("generation", crate::preserves_rail::u64_value(input.generation)),
        checks_value(&[
            "state-schema-compatible",
            "generation-created",
            "checkpoint-explicit",
            "migration-receipt-is-not-durability-proof",
        ]),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalStateMigrationReceipt {
        receipt_ref,
        operation: input.operation,
        source_schema: input.source_schema.to_string(),
        target_schema: input.target_schema.to_string(),
        value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalServiceReadiness {
    pub readiness_ref: String,
    pub service_id: String,
    pub generation: u64,
    pub ready: bool,
    pub health: super::HealthState,
    pub value: preserves::IOValue,
}

pub(crate) fn canonical_service_readiness(
    manifest_ref: &str,
    extension_id: &str,
    service_id: &str,
    state: &super::LifecycleState,
    boundary_ref: &str,
) -> crate::error::Result<CanonicalServiceReadiness> {
    let is_ready = state.phase == super::LifecyclePhase::Running && state.health == super::HealthState::Healthy;
    let action = if is_ready { "publish" } else { "withdraw" };
    let value = crate::preserves_rail::record("system-extension-readiness-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_READINESS_SCHEMA),
        field("manifest-ref", crate::preserves_rail::string(manifest_ref)),
        field("extension-id", crate::preserves_rail::string(extension_id)),
        field("service-id", crate::preserves_rail::string(service_id)),
        field("generation", crate::preserves_rail::u64_value(state.generation)),
        field("phase", crate::preserves_rail::string(state.phase.as_str())),
        field("health", crate::preserves_rail::string(state.health.as_str())),
        field("ready", crate::preserves_rail::bool_value(is_ready)),
        field("action", crate::preserves_rail::string(action)),
        field("boundary-ref", crate::preserves_rail::string(boundary_ref)),
        checks_value(&[
            "generation-fenced-readiness",
            "failed-or-stopped-withdrawn",
            "readiness-is-not-authority",
        ]),
    ]);
    let readiness_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalServiceReadiness {
        readiness_ref,
        service_id: service_id.to_string(),
        generation: state.generation,
        ready: is_ready,
        health: state.health,
        value,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorStatus {
    pub extension_id: String,
    pub service_id: String,
    pub manifest_ref: String,
    pub generation: u64,
    pub phase: super::LifecyclePhase,
    pub execution_profile: super::ExecutionProfile,
    pub port_binding_refs: Vec<String>,
    pub resources: super::ResourceEnvelope,
    pub usage: super::ResourceUsage,
    pub health: super::HealthState,
    pub restart_attempts: u64,
    pub checkpoint_ref: Option<String>,
    pub last_lifecycle_ref: Option<String>,
    pub invocation_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalOperatorStatus {
    pub status_ref: String,
    pub status: OperatorStatus,
    pub value: preserves::IOValue,
}
