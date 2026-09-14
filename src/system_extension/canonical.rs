const MAX_CANONICAL_EXTENSION_ITEMS: usize = 128;
const MAX_READBACK_IDENTIFIER_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAdmittedSystemExtensionManifest {
    manifest: super::AdmittedSystemExtensionManifest,
    manifest_ref: String,
    value: preserves::IOValue,
    tier_admission_ref: String,
    required_port_bindings: Vec<crate::fabric::CanonicalFabricPortBinding>,
    optional_port_bindings: Vec<crate::fabric::CanonicalFabricPortBinding>,
}

impl CanonicalAdmittedSystemExtensionManifest {
    pub fn manifest(&self) -> &super::AdmittedSystemExtensionManifest {
        &self.manifest
    }

    pub fn manifest_ref(&self) -> &str {
        &self.manifest_ref
    }

    pub fn value(&self) -> &preserves::IOValue {
        &self.value
    }

    pub fn tier_admission_ref(&self) -> &str {
        &self.tier_admission_ref
    }

    pub fn required_port_bindings(&self) -> &[crate::fabric::CanonicalFabricPortBinding] {
        &self.required_port_bindings
    }

    pub fn optional_port_bindings(&self) -> &[crate::fabric::CanonicalFabricPortBinding] {
        &self.optional_port_bindings
    }

    pub fn all_binding_refs(&self) -> impl Iterator<Item = &str> {
        self.required_port_bindings
            .iter()
            .chain(self.optional_port_bindings.iter())
            .map(|binding| binding.binding_ref.as_str())
    }

    pub fn binding_for(
        &self,
        key: &crate::fabric::FabricPortKey,
    ) -> Option<&crate::fabric::CanonicalFabricPortBinding> {
        self.required_port_bindings
            .iter()
            .chain(self.optional_port_bindings.iter())
            .find(|binding| binding.binding.key == *key)
    }
}

// r[impl molten.system_extension.manifest]
// r[impl molten.system_extension.typed_effects]
pub fn canonical_admit_system_extension_manifest(
    input: &super::SystemExtensionManifestInput,
    descriptors: &[crate::fabric::FabricPortDescriptor],
    tier: &crate::fabric::CanonicalExtensionTierAdmission,
    admitted_execution_profiles: &[super::ExecutionProfile],
) -> crate::error::Result<CanonicalAdmittedSystemExtensionManifest> {
    let registry = crate::fabric::build_fabric_port_registry(descriptors)
        .map_err(|issues| validation_error("system-extension fabric registry", &issues))?;
    let manifest = super::admit_system_extension_manifest(input, super::SystemExtensionAdmissionContext {
        registry: &registry,
        tier_admission: &tier.admission,
        admitted_execution_profiles,
    })
    .map_err(|issues| validation_error("system-extension manifest", &issues))?;

    let mut required_port_bindings = Vec::with_capacity(manifest.required_port_requirements.len());
    for requirement in &manifest.required_port_requirements {
        required_port_bindings.push(crate::fabric::resolve_canonical_fabric_port_binding(descriptors, requirement)?);
    }
    let mut optional_port_bindings = Vec::with_capacity(manifest.optional_port_bindings.len());
    for binding in &manifest.optional_port_bindings {
        let requirement = manifest
            .optional_port_requirements
            .iter()
            .find(|requirement| {
                requirement.port_id == binding.key.port_id && requirement.version == binding.key.version
            })
            .ok_or_else(|| {
                crate::error::MoltenError::invalid_harness(format!(
                    "optional binding {}@{} has no admitted requirement",
                    binding.key.port_id, binding.key.version
                ))
            })?;
        optional_port_bindings.push(crate::fabric::resolve_canonical_fabric_port_binding(descriptors, requirement)?);
    }
    required_port_bindings.sort_by(|left, right| left.binding.key.cmp(&right.binding.key));
    optional_port_bindings.sort_by(|left, right| left.binding.key.cmp(&right.binding.key));

    let value = system_extension_manifest_value(
        &manifest,
        &tier.admission_ref,
        &required_port_bindings,
        &optional_port_bindings,
    );
    let manifest_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalAdmittedSystemExtensionManifest {
        manifest,
        manifest_ref,
        value,
        tier_admission_ref: tier.admission_ref.clone(),
        required_port_bindings,
        optional_port_bindings,
    })
}

fn system_extension_manifest_value(
    manifest: &super::AdmittedSystemExtensionManifest,
    tier_admission_ref: &str,
    required_bindings: &[crate::fabric::CanonicalFabricPortBinding],
    optional_bindings: &[crate::fabric::CanonicalFabricPortBinding],
) -> preserves::IOValue {
    crate::preserves_rail::record("system-extension-manifest-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_MANIFEST_SCHEMA),
        field("extension-id", crate::preserves_rail::string(&manifest.extension_id)),
        field("service-id", crate::preserves_rail::string(&manifest.service_id)),
        field("implementation-ref", crate::preserves_rail::string(&manifest.implementation_ref)),
        field("callbacks", strings_value(manifest.callbacks.iter().map(|callback| callback.as_str()))),
        field(
            "required-port-binding-refs",
            strings_value(required_bindings.iter().map(|binding| binding.binding_ref.as_str())),
        ),
        field(
            "optional-port-binding-refs",
            strings_value(optional_bindings.iter().map(|binding| binding.binding_ref.as_str())),
        ),
        field("capability-refs", strings_value(manifest.capability_refs.iter().map(String::as_str))),
        field("policy-refs", strings_value(manifest.policy_refs.iter().map(String::as_str))),
        field("provenance-refs", strings_value(manifest.provenance_refs.iter().map(String::as_str))),
        field("resource-envelope", resource_envelope_value(&manifest.resources)),
        field("execution-profile", crate::preserves_rail::string(manifest.execution_profile.as_str())),
        field("state-schema", crate::preserves_rail::string(&manifest.state_schema)),
        field(
            "compatible-state-schemas",
            strings_value(manifest.compatible_state_schemas.iter().map(String::as_str)),
        ),
        field("evidence-profile-ref", crate::preserves_rail::string(&manifest.evidence_profile_ref)),
        field("tier-admission-ref", crate::preserves_rail::string(tier_admission_ref)),
        field("initial-generation", crate::preserves_rail::u64_value(manifest.initial_generation)),
        field("non-claims", strings_value(manifest.non_claims.iter().map(|non_claim| non_claim.as_str()))),
        checks_value(&[
            "canonical-system-extension-manifest",
            "system-tier-admitted",
            "ports-exactly-bound",
            "capabilities-not-artifact-possession",
            "plugin-metadata-not-system-extension-admission",
            "execution-profile-no-fallback",
        ]),
    ])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalLifecycleReceipt {
    pub receipt_ref: String,
    pub previous: super::LifecycleState,
    pub next: super::LifecycleState,
    pub event: super::LifecycleEvent,
    pub value: preserves::IOValue,
}

pub(crate) fn canonical_lifecycle_receipt(
    manifest_ref: &str,
    extension_id: &str,
    service_id: &str,
    previous: &super::LifecycleState,
    next: &super::LifecycleState,
    event: &super::LifecycleEvent,
    usage: super::ResourceUsage,
) -> crate::error::Result<CanonicalLifecycleReceipt> {
    let value = crate::preserves_rail::record("system-extension-lifecycle-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_LIFECYCLE_SCHEMA),
        field("manifest-ref", crate::preserves_rail::string(manifest_ref)),
        field("extension-id", crate::preserves_rail::string(extension_id)),
        field("service-id", crate::preserves_rail::string(service_id)),
        field("event", crate::preserves_rail::string(event.kind.as_str())),
        field("event-generation", crate::preserves_rail::u64_value(event.generation)),
        field("previous-generation", crate::preserves_rail::u64_value(previous.generation)),
        field("next-generation", crate::preserves_rail::u64_value(next.generation)),
        field("previous-phase", crate::preserves_rail::string(previous.phase.as_str())),
        field("next-phase", crate::preserves_rail::string(next.phase.as_str())),
        field("restart-attempts", crate::preserves_rail::u64_value(next.restart_attempts)),
        field("health", crate::preserves_rail::string(next.health.as_str())),
        field("checkpoint-ref", optional_string(event.checkpoint_ref.as_deref())),
        field("failure-class", optional_failure(event.failure_class)),
        field("resource-usage", resource_usage_value(usage)),
        checks_value(&[
            "generation-fenced",
            "transition-law-validated",
            "logical-inputs-only",
            "receipt-is-not-consensus-or-durability-proof",
        ]),
    ]);
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalLifecycleReceipt {
        receipt_ref,
        previous: previous.clone(),
        next: next.clone(),
        event: event.clone(),
        value,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackExecutionDecision {
    Succeeded,
    ExecutorFailed,
    OutcomeDenied,
}

impl CallbackExecutionDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::ExecutorFailed => "executor-failed",
            Self::OutcomeDenied => "outcome-denied",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalCallbackReceipt {
    pub receipt_ref: String,
    pub execution_binding_ref: String,
    pub invocation: super::CallbackInvocation,
    pub decision: CallbackExecutionDecision,
    pub approved_effects: Vec<super::TypedEffectRequest>,
    pub value: preserves::IOValue,
}

pub(crate) struct CallbackReceiptInput<'a> {
    pub manifest_ref: &'a str,
    pub extension_id: &'a str,
    pub service_id: &'a str,
    pub execution_profile: super::ExecutionProfile,
    pub invocation: &'a super::CallbackInvocation,
    pub decision: CallbackExecutionDecision,
    pub outcome: Option<&'a super::CallbackOutcome>,
    pub diagnostic: Option<&'a str>,
}

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
    validate_readback_enum(&phase, "phase", &[
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
    ])?;
    validate_readback_enum(&execution_profile, "execution-profile", &[
        "in-process-native",
        "native-process",
        "sandboxed-component",
    ])?;
    validate_readback_enum(&health, "health", &[
        "unknown",
        "starting",
        "healthy",
        "degraded",
        "failed",
        "quarantined",
        "stopped",
    ])?;
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

pub(crate) fn callback_event_value(
    callback: super::CallbackKind,
    generation: u64,
    sequence: u64,
    payload_ref: Option<&str>,
    logical_tick: u64,
    deadline_tick: u64,
) -> preserves::IOValue {
    crate::preserves_rail::record("system-extension-callback-event-v1", vec![
        crate::preserves_rail::string(super::SYSTEM_EXTENSION_CALLBACK_SCHEMA),
        field("callback", crate::preserves_rail::string(callback.as_str())),
        field("generation", crate::preserves_rail::u64_value(generation)),
        field("sequence", crate::preserves_rail::u64_value(sequence)),
        field("payload-ref", optional_string(payload_ref)),
        field("logical-tick", crate::preserves_rail::u64_value(logical_tick)),
        field("deadline-tick", crate::preserves_rail::u64_value(deadline_tick)),
    ])
}
