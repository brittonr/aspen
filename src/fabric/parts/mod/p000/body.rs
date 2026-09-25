
pub use molten_core::fabric::*;
#[allow(
    tigerstyle::non_trait_imports,
    reason = "fabric application modules share one typed external capability error vocabulary"
)]
pub use port::*;

pub const FABRIC_BOUNDARY_SCHEMA: &str = "molten.fabric.boundary.v1";
pub const FABRIC_TIER_ADMISSION_SCHEMA: &str = "molten.fabric.tier-admission.v1";
pub const FABRIC_REFERENCE_MATRIX_SUITE_SCHEMA: &str = "molten.fabric.reference-matrix-suite.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFabricBoundary {
    pub report: FabricBoundaryReport,
    pub boundary_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalExtensionTierAdmission {
    pub admission: ExtensionTierAdmission,
    pub admission_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFabricPortBinding {
    pub binding: FabricPortBinding,
    pub descriptor_ref: String,
    pub registry_ref: String,
    pub binding_ref: String,
    pub descriptor_value: preserves::IOValue,
    pub registry_value: preserves::IOValue,
    pub binding_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFabricEvidenceProfile {
    pub summary: FabricEvidenceProfileSummary,
    pub profile_ref: String,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalReferenceMatrixSuite {
    pub summary: ReferenceMatrixSummary,
    pub suite_ref: String,
    pub value: preserves::IOValue,
}

// r[impl molten.fabric_boundary.fabric_identity]
// r[impl molten.fabric_boundary.mechanism_semantics_separation]
// r[impl molten.fabric_boundary.non_claims]
pub fn canonical_fabric_boundary(
    descriptor: &FabricBoundaryDescriptor,
) -> crate::error::Result<CanonicalFabricBoundary> {
    let report = validate_fabric_boundary(descriptor).map_err(|issues| validation_error("fabric boundary", &issues))?;
    let value = fabric_boundary_value(&report);
    let boundary_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalFabricBoundary {
        report,
        boundary_ref,
        value,
    })
}

// r[impl molten.fabric_boundary.extension_tiers]
pub fn canonical_extension_tier_admission(
    request: &ExtensionTierRequest,
) -> crate::error::Result<CanonicalExtensionTierAdmission> {
    let admission = validate_extension_tier(request).map_err(|issues| validation_error("extension tier", &issues))?;
    let value = extension_tier_admission_value(&admission);
    let admission_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalExtensionTierAdmission {
        admission,
        admission_ref,
        value,
    })
}

// r[impl molten.fabric_boundary.port_registry]
pub fn canonical_fabric_port_descriptor(
    descriptor: &FabricPortDescriptor,
) -> crate::error::Result<(String, preserves::IOValue)> {
    let registry = build_fabric_port_registry(std::slice::from_ref(descriptor))
        .map_err(|issues| validation_error("fabric port descriptor", &issues))?;
    let Some(normalized) = registry.descriptors().first() else {
        return Err(crate::error::MoltenError::invalid_harness(
            "fabric port descriptor validation produced an empty registry",
        ));
    };
    let value = fabric_port_descriptor_value(normalized);
    let descriptor_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok((descriptor_ref, value))
}

// r[impl molten.fabric_boundary.port_registry]
pub fn resolve_canonical_fabric_port_binding(
    descriptors: &[FabricPortDescriptor],
    requirement: &FabricPortRequirement,
) -> crate::error::Result<CanonicalFabricPortBinding> {
    let registry =
        build_fabric_port_registry(descriptors).map_err(|issues| validation_error("fabric port registry", &issues))?;
    let binding = resolve_fabric_port_binding(&registry, requirement)
        .map_err(|issues| validation_error("fabric port binding", &issues))?;
    let Some(descriptor) = registry.descriptors().iter().find(|descriptor| descriptor.key() == binding.key) else {
        return Err(crate::error::MoltenError::invalid_harness("fabric port binding resolved without its descriptor"));
    };

    let descriptor_value = fabric_port_descriptor_value(descriptor);
    let descriptor_ref = crate::preserves_rail::canonical_hash(&descriptor_value)?;
    let registry_value = fabric_port_registry_value(&registry)?;
    let registry_ref = crate::preserves_rail::canonical_hash(&registry_value)?;
    let binding_value = fabric_port_binding_value(&binding, &descriptor_ref, &registry_ref);
    let binding_ref = crate::preserves_rail::canonical_hash(&binding_value)?;
    Ok(CanonicalFabricPortBinding {
        binding,
        descriptor_ref,
        registry_ref,
        binding_ref,
        descriptor_value,
        registry_value,
        binding_value,
    })
}

// r[impl molten.fabric_boundary.evidence_granularity]
// r[impl molten.fabric_boundary.non_claims]
pub fn canonical_fabric_evidence_profile(
    profile: &FabricEvidenceProfile,
) -> crate::error::Result<CanonicalFabricEvidenceProfile> {
    let summary = validate_fabric_evidence_profile(profile)
        .map_err(|issues| validation_error("fabric evidence profile", &issues))?;
    let value = fabric_evidence_profile_value(&summary);
    let profile_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalFabricEvidenceProfile {
        summary,
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_boundary.reference_system_exit_criteria]
// r[impl molten.fabric_boundary.non_claims]
pub fn canonical_reference_matrix_suite(
    matrices: &[ReferenceSystemMatrix],
) -> crate::error::Result<CanonicalReferenceMatrixSuite> {
    let summary = validate_reference_system_matrices(matrices)
        .map_err(|issues| validation_error("fabric reference matrix", &issues))?;
    let value = reference_matrix_suite_value(&summary);
    let suite_ref = crate::preserves_rail::canonical_hash(&value)?;
    Ok(CanonicalReferenceMatrixSuite {
        summary,
        suite_ref,
        value,
    })
}

fn fabric_boundary_value(report: &FabricBoundaryReport) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-boundary-v1", vec![
        crate::preserves_rail::string(FABRIC_BOUNDARY_SCHEMA),
        field("identity", crate::preserves_rail::string(report.identity.as_str())),
        field("mechanisms", strings_value(report.mechanisms.iter().map(|mechanism| mechanism.as_str()))),
        field("core-owned-workload-semantics", strings_value(std::iter::empty::<&str>())),
        field("non-claims", strings_value(report.non_claims.iter().map(|non_claim| non_claim.as_str()))),
        checks_value(&[
            "workload-neutral-fabric",
            "mechanisms-only-in-core",
            "extension-semantics-excluded",
            "non-claims-explicit",
        ]),
    ])
}

fn extension_tier_admission_value(admission: &ExtensionTierAdmission) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-tier-admission-v1", vec![
        crate::preserves_rail::string(FABRIC_TIER_ADMISSION_SCHEMA),
        field("tier", crate::preserves_rail::string(admission.tier.as_str())),
        field(
            "authorities",
            strings_value(admission.admitted_authorities.iter().map(|authority| authority.as_str())),
        ),
        field(
            "supporting-evidence",
            strings_value(admission.supporting_evidence.iter().map(|evidence| evidence.as_str())),
        ),
        checks_value(&[
            "tier-explicit",
            "authority-declared",
            "artifact-possession-not-authority",
        ]),
    ])
}

fn fabric_port_descriptor_value(descriptor: &FabricPortDescriptor) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-port-descriptor-v1", vec![
        crate::preserves_rail::string(FABRIC_PORT_DESCRIPTOR_SCHEMA),
        field("port-id", crate::preserves_rail::string(&descriptor.port_id)),
        field("version", crate::preserves_rail::string(&descriptor.version)),
        field("class", crate::preserves_rail::string(descriptor.class.as_str())),
        field("operations", strings_value(descriptor.operation_classes.iter().map(String::as_str))),
        field("input-schemas", strings_value(descriptor.input_schema_refs.iter().map(String::as_str))),
        field("output-schemas", strings_value(descriptor.output_schema_refs.iter().map(String::as_str))),
        field(
            "authorities",
            strings_value(descriptor.authority_requirements.iter().map(|authority| authority.as_str())),
        ),
        field(
            "resources",
            strings_value(descriptor.resource_requirements.iter().map(|resource| resource.as_str())),
        ),
        field("determinism", crate::preserves_rail::string(descriptor.determinism.as_str())),
        field("replay", crate::preserves_rail::string(descriptor.replay.as_str())),
        field("implementation-profile", crate::preserves_rail::string(&descriptor.implementation_profile)),
        field("conformance-refs", strings_value(descriptor.conformance_refs.iter().map(String::as_str))),
        field("non-claims", strings_value(descriptor.non_claims.iter().map(|non_claim| non_claim.as_str()))),
        field("enabled", crate::preserves_rail::bool_value(descriptor.enabled)),
        checks_value(&[
            "canonical-port-key",
            "profile-explicit",
            "authority-is-requirement-not-grant",
            "adapter-types-excluded",
        ]),
    ])
}

fn fabric_port_registry_value(registry: &FabricPortRegistry) -> crate::error::Result<preserves::IOValue> {
    let mut descriptor_refs = Vec::with_capacity(registry.descriptors().len());
    for descriptor in registry.descriptors() {
        let descriptor_value = fabric_port_descriptor_value(descriptor);
        descriptor_refs.push(crate::preserves_rail::canonical_hash(&descriptor_value)?);
    }
    Ok(crate::preserves_rail::record("fabric-port-registry-v1", vec![
        crate::preserves_rail::string(FABRIC_PORT_REGISTRY_SCHEMA),
        field("descriptor-refs", strings_value(descriptor_refs.iter().map(String::as_str))),
        checks_value(&["keys-unique", "versions-exact", "silent-substitution-denied"]),
    ]))
}

fn fabric_port_binding_value(
    binding: &FabricPortBinding,
    descriptor_ref: &str,
    registry_ref: &str,
) -> preserves::IOValue {
    crate::preserves_rail::record("fabric-port-binding-v1", vec![
        crate::preserves_rail::string(FABRIC_PORT_BINDING_SCHEMA),
        field("port-id", crate::preserves_rail::string(&binding.key.port_id)),
        field("version", crate::preserves_rail::string(&binding.key.version)),
        field("class", crate::preserves_rail::string(binding.class.as_str())),
        field("implementation-profile", crate::preserves_rail::string(&binding.implementation_profile)),
        field("descriptor-ref", crate::preserves_rail::string(descriptor_ref)),
        field("registry-ref", crate::preserves_rail::string(registry_ref)),
        field("conformance-refs", strings_value(binding.conformance_refs.iter().map(String::as_str))),
        field("non-claims", strings_value(binding.non_claims.iter().map(|non_claim| non_claim.as_str()))),
        checks_value(&[
            "exact-port-version",
            "exact-profile",
            "schemas-compatible",
            "binding-is-not-behavioral-proof",
        ]),
    ])
}

fn fabric_evidence_profile_value(summary: &FabricEvidenceProfileSummary) -> preserves::IOValue {
    let rules = summary
        .rules
        .iter()
        .map(|rule| {
            crate::preserves_rail::record("evidence-rule", vec![
                crate::preserves_rail::string(rule.boundary.as_str()),
                crate::preserves_rail::string(rule.emission.as_str()),
            ])
        })
        .collect::<Vec<_>>();
    crate::preserves_rail::record("fabric-evidence-profile-v1", vec![
        crate::preserves_rail::string(FABRIC_EVIDENCE_PROFILE_SCHEMA),
        field("profile-id", crate::preserves_rail::string(&summary.profile_id)),
        field("class", crate::preserves_rail::string(summary.class.as_str())),
        field("rules", crate::preserves_rail::sequence(rules)),
        field("aggregate-limit-ref", optional_string_value(summary.aggregate_limit_ref.as_deref())),
        field("non-claims", strings_value(summary.non_claims.iter().map(|non_claim| non_claim.as_str()))),
        checks_value(&[
            "semantic-boundaries-canonical",
            "internal-operations-bounded",
            "debug-profile-not-production-default",
        ]),
    ])
}
