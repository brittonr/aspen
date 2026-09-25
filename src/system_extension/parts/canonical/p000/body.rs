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

pub(crate) struct LifecycleReceiptInput<'a> {
    pub(crate) manifest_ref: &'a str,
    pub(crate) extension_id: &'a str,
    pub(crate) service_id: &'a str,
    pub(crate) previous: &'a super::LifecycleState,
    pub(crate) next: &'a super::LifecycleState,
    pub(crate) event: &'a super::LifecycleEvent,
    pub(crate) usage: super::ResourceUsage,
}

pub(crate) fn canonical_lifecycle_receipt(
    input: LifecycleReceiptInput<'_>,
) -> crate::error::Result<CanonicalLifecycleReceipt> {
    let LifecycleReceiptInput {
        manifest_ref,
        extension_id,
        service_id,
        previous,
        next,
        event,
        usage,
    } = input;
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
