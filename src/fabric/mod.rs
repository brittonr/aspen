//! Canonical Preserves projection for the pure fabric contracts.
//!
//! The in-memory models and validation laws live in `molten-core`. This module
//! assigns canonical Preserves schemas and BLAKE3 refs without performing I/O;
//! adapter shells decide whether and where admitted artifacts are persisted.

pub use molten_core::fabric::*;
use preserves::IOValue;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::bool_value;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;

pub const FABRIC_BOUNDARY_SCHEMA: &str = "molten.fabric.boundary.v1";
pub const FABRIC_TIER_ADMISSION_SCHEMA: &str = "molten.fabric.tier-admission.v1";
pub const FABRIC_REFERENCE_MATRIX_SUITE_SCHEMA: &str = "molten.fabric.reference-matrix-suite.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFabricBoundary {
    pub report: FabricBoundaryReport,
    pub boundary_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalExtensionTierAdmission {
    pub admission: ExtensionTierAdmission,
    pub admission_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFabricPortBinding {
    pub binding: FabricPortBinding,
    pub descriptor_ref: String,
    pub registry_ref: String,
    pub binding_ref: String,
    pub descriptor_value: IOValue,
    pub registry_value: IOValue,
    pub binding_value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalFabricEvidenceProfile {
    pub summary: FabricEvidenceProfileSummary,
    pub profile_ref: String,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalReferenceMatrixSuite {
    pub summary: ReferenceMatrixSummary,
    pub suite_ref: String,
    pub value: IOValue,
}

// r[impl molten.fabric_boundary.fabric_identity]
// r[impl molten.fabric_boundary.mechanism_semantics_separation]
// r[impl molten.fabric_boundary.non_claims]
pub fn canonical_fabric_boundary(descriptor: &FabricBoundaryDescriptor) -> Result<CanonicalFabricBoundary> {
    let report = validate_fabric_boundary(descriptor).map_err(|issues| validation_error("fabric boundary", &issues))?;
    let value = fabric_boundary_value(&report);
    let boundary_ref = canonical_hash(&value)?;
    Ok(CanonicalFabricBoundary {
        report,
        boundary_ref,
        value,
    })
}

// r[impl molten.fabric_boundary.extension_tiers]
pub fn canonical_extension_tier_admission(request: &ExtensionTierRequest) -> Result<CanonicalExtensionTierAdmission> {
    let admission = validate_extension_tier(request).map_err(|issues| validation_error("extension tier", &issues))?;
    let value = extension_tier_admission_value(&admission);
    let admission_ref = canonical_hash(&value)?;
    Ok(CanonicalExtensionTierAdmission {
        admission,
        admission_ref,
        value,
    })
}

// r[impl molten.fabric_boundary.port_registry]
pub fn canonical_fabric_port_descriptor(descriptor: &FabricPortDescriptor) -> Result<(String, IOValue)> {
    let registry = build_fabric_port_registry(std::slice::from_ref(descriptor))
        .map_err(|issues| validation_error("fabric port descriptor", &issues))?;
    let Some(normalized) = registry.descriptors().first() else {
        return Err(MoltenError::invalid_harness("fabric port descriptor validation produced an empty registry"));
    };
    let value = fabric_port_descriptor_value(normalized);
    let descriptor_ref = canonical_hash(&value)?;
    Ok((descriptor_ref, value))
}

// r[impl molten.fabric_boundary.port_registry]
pub fn resolve_canonical_fabric_port_binding(
    descriptors: &[FabricPortDescriptor],
    requirement: &FabricPortRequirement,
) -> Result<CanonicalFabricPortBinding> {
    let registry =
        build_fabric_port_registry(descriptors).map_err(|issues| validation_error("fabric port registry", &issues))?;
    let binding = resolve_fabric_port_binding(&registry, requirement)
        .map_err(|issues| validation_error("fabric port binding", &issues))?;
    let Some(descriptor) = registry.descriptors().iter().find(|descriptor| descriptor.key() == binding.key) else {
        return Err(MoltenError::invalid_harness("fabric port binding resolved without its descriptor"));
    };

    let descriptor_value = fabric_port_descriptor_value(descriptor);
    let descriptor_ref = canonical_hash(&descriptor_value)?;
    let registry_value = fabric_port_registry_value(&registry)?;
    let registry_ref = canonical_hash(&registry_value)?;
    let binding_value = fabric_port_binding_value(&binding, &descriptor_ref, &registry_ref);
    let binding_ref = canonical_hash(&binding_value)?;
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
pub fn canonical_fabric_evidence_profile(profile: &FabricEvidenceProfile) -> Result<CanonicalFabricEvidenceProfile> {
    let summary = validate_fabric_evidence_profile(profile)
        .map_err(|issues| validation_error("fabric evidence profile", &issues))?;
    let value = fabric_evidence_profile_value(&summary);
    let profile_ref = canonical_hash(&value)?;
    Ok(CanonicalFabricEvidenceProfile {
        summary,
        profile_ref,
        value,
    })
}

// r[impl molten.fabric_boundary.reference_system_exit_criteria]
// r[impl molten.fabric_boundary.non_claims]
pub fn canonical_reference_matrix_suite(matrices: &[ReferenceSystemMatrix]) -> Result<CanonicalReferenceMatrixSuite> {
    let summary = validate_reference_system_matrices(matrices)
        .map_err(|issues| validation_error("fabric reference matrix", &issues))?;
    let value = reference_matrix_suite_value(&summary);
    let suite_ref = canonical_hash(&value)?;
    Ok(CanonicalReferenceMatrixSuite {
        summary,
        suite_ref,
        value,
    })
}

fn fabric_boundary_value(report: &FabricBoundaryReport) -> IOValue {
    record("fabric-boundary-v1", vec![
        string(FABRIC_BOUNDARY_SCHEMA),
        field("identity", string(report.identity.as_str())),
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

fn extension_tier_admission_value(admission: &ExtensionTierAdmission) -> IOValue {
    record("fabric-tier-admission-v1", vec![
        string(FABRIC_TIER_ADMISSION_SCHEMA),
        field("tier", string(admission.tier.as_str())),
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

fn fabric_port_descriptor_value(descriptor: &FabricPortDescriptor) -> IOValue {
    record("fabric-port-descriptor-v1", vec![
        string(FABRIC_PORT_DESCRIPTOR_SCHEMA),
        field("port-id", string(&descriptor.port_id)),
        field("version", string(&descriptor.version)),
        field("class", string(descriptor.class.as_str())),
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
        field("determinism", string(descriptor.determinism.as_str())),
        field("replay", string(descriptor.replay.as_str())),
        field("implementation-profile", string(&descriptor.implementation_profile)),
        field("conformance-refs", strings_value(descriptor.conformance_refs.iter().map(String::as_str))),
        field("non-claims", strings_value(descriptor.non_claims.iter().map(|non_claim| non_claim.as_str()))),
        field("enabled", bool_value(descriptor.enabled)),
        checks_value(&[
            "canonical-port-key",
            "profile-explicit",
            "authority-is-requirement-not-grant",
            "adapter-types-excluded",
        ]),
    ])
}

fn fabric_port_registry_value(registry: &FabricPortRegistry) -> Result<IOValue> {
    let mut descriptor_refs = Vec::with_capacity(registry.descriptors().len());
    for descriptor in registry.descriptors() {
        let descriptor_value = fabric_port_descriptor_value(descriptor);
        descriptor_refs.push(canonical_hash(&descriptor_value)?);
    }
    Ok(record("fabric-port-registry-v1", vec![
        string(FABRIC_PORT_REGISTRY_SCHEMA),
        field("descriptor-refs", strings_value(descriptor_refs.iter().map(String::as_str))),
        checks_value(&["keys-unique", "versions-exact", "silent-substitution-denied"]),
    ]))
}

fn fabric_port_binding_value(binding: &FabricPortBinding, descriptor_ref: &str, registry_ref: &str) -> IOValue {
    record("fabric-port-binding-v1", vec![
        string(FABRIC_PORT_BINDING_SCHEMA),
        field("port-id", string(&binding.key.port_id)),
        field("version", string(&binding.key.version)),
        field("class", string(binding.class.as_str())),
        field("implementation-profile", string(&binding.implementation_profile)),
        field("descriptor-ref", string(descriptor_ref)),
        field("registry-ref", string(registry_ref)),
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

fn fabric_evidence_profile_value(summary: &FabricEvidenceProfileSummary) -> IOValue {
    let rules = summary
        .rules
        .iter()
        .map(|rule| record("evidence-rule", vec![string(rule.boundary.as_str()), string(rule.emission.as_str())]))
        .collect::<Vec<_>>();
    record("fabric-evidence-profile-v1", vec![
        string(FABRIC_EVIDENCE_PROFILE_SCHEMA),
        field("profile-id", string(&summary.profile_id)),
        field("class", string(summary.class.as_str())),
        field("rules", sequence(rules)),
        field("aggregate-limit-ref", optional_string_value(summary.aggregate_limit_ref.as_deref())),
        field("non-claims", strings_value(summary.non_claims.iter().map(|non_claim| non_claim.as_str()))),
        checks_value(&[
            "semantic-boundaries-canonical",
            "internal-operations-bounded",
            "debug-profile-not-production-default",
        ]),
    ])
}

fn reference_matrix_suite_value(summary: &ReferenceMatrixSummary) -> IOValue {
    let matrices = summary.matrices.iter().map(reference_matrix_value).collect::<Vec<_>>();
    record("fabric-reference-matrix-suite-v1", vec![
        string(FABRIC_REFERENCE_MATRIX_SUITE_SCHEMA),
        field("matrices", sequence(matrices)),
        checks_value(&[
            "three-reference-classes",
            "ports-not-ambient-access",
            "semantics-extension-owned",
            "conformance-not-correctness-proof",
        ]),
    ])
}

fn reference_matrix_value(matrix: &ReferenceSystemMatrix) -> IOValue {
    let semantics = matrix
        .semantics
        .iter()
        .map(|ownership| {
            record("semantic-ownership", vec![string(ownership.semantic.as_str()), string(ownership.owner.as_str())])
        })
        .collect::<Vec<_>>();
    record("fabric-reference-matrix-v1", vec![
        string(FABRIC_REFERENCE_MATRIX_SCHEMA),
        field("system", string(matrix.system.as_str())),
        field("capabilities", strings_value(matrix.capabilities.iter().map(|capability| capability.as_str()))),
        field("semantics", sequence(semantics)),
        field("ambient-accesses", strings_value(matrix.ambient_accesses.iter().map(String::as_str))),
        field("non-claims", strings_value(matrix.non_claims.iter().map(|non_claim| non_claim.as_str()))),
    ])
}

fn field(label: &'static str, value: IOValue) -> IOValue {
    record(label, vec![value])
}

fn strings_value<'a>(values: impl IntoIterator<Item = &'a str>) -> IOValue {
    sequence(values.into_iter().map(string).collect())
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn checks_value(checks: &[&str]) -> IOValue {
    field("checks", strings_value(checks.iter().copied()))
}

fn validation_error(label: &str, issues: &impl std::fmt::Debug) -> MoltenError {
    MoltenError::invalid_harness(format!("{label} validation denied: {issues:?}"))
}

// r[impl molten.fabric_boundary.final_validation]
#[cfg(test)]
mod tests {
    use super::*;

    const CONFORMANCE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const LIMIT_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const PORT_ID: &str = "molten.fabric.transport.session";
    const PORT_VERSION: &str = "v1";
    const PORT_PROFILE: &str = "iroh-live-v1";
    const PORT_OPERATION: &str = "send-envelope";
    const INPUT_SCHEMA: &str = "molten.fabric.transport-send.v1";
    const OUTPUT_SCHEMA: &str = "molten.fabric.transport-outcome.v1";

    fn descriptor() -> FabricPortDescriptor {
        FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: PORT_ID.to_string(),
            version: PORT_VERSION.to_string(),
            class: FabricPortClass::Transport,
            operation_classes: vec![PORT_OPERATION.to_string()],
            input_schema_refs: vec![INPUT_SCHEMA.to_string()],
            output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
            authority_requirements: vec![FabricAuthority::Transport],
            resource_requirements: vec![FabricResource::NetworkBytes],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: PORT_PROFILE.to_string(),
            conformance_refs: vec![CONFORMANCE_REF.to_string()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        }
    }

    fn requirement() -> FabricPortRequirement {
        FabricPortRequirement {
            port_id: PORT_ID.to_string(),
            version: PORT_VERSION.to_string(),
            class: FabricPortClass::Transport,
            operation_classes: vec![PORT_OPERATION.to_string()],
            input_schema_refs: vec![INPUT_SCHEMA.to_string()],
            output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
            allowed_authorities: vec![FabricAuthority::Transport],
            available_resources: vec![FabricResource::NetworkBytes],
            expected_determinism: DeterminismClass::ExternalEffect,
            expected_replay: ReplayClass::RecordedEffectRequired,
            expected_profile: PORT_PROFILE.to_string(),
        }
    }

    // r[verify molten.fabric_boundary.port_registry]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_port_binding_is_stable_and_names_only_reviewed_profile() {
        let first = resolve_canonical_fabric_port_binding(&[descriptor()], &requirement())
            .expect("canonical compatible binding");
        let second =
            resolve_canonical_fabric_port_binding(&[descriptor()], &requirement()).expect("repeat canonical binding");
        let text = crate::preserves_rail::to_text(&first.binding_value).expect("binding text");

        assert_eq!(first.descriptor_ref, second.descriptor_ref);
        assert_eq!(first.registry_ref, second.registry_ref);
        assert_eq!(first.binding_ref, second.binding_ref);
        assert!(first.binding_ref.starts_with("blake3:"));
        assert!(text.contains("fabric-port-binding-v1"));
        assert!(text.contains(PORT_PROFILE));
        assert!(text.contains("binding-is-not-behavioral-proof"));
        assert!(!text.contains("redb::"));
        assert!(!text.contains("iroh::Endpoint"));
    }

    // r[verify molten.fabric_boundary.fabric_identity]
    // r[verify molten.fabric_boundary.evidence_granularity]
    // r[verify molten.fabric_boundary.reference_system_exit_criteria]
    // r[verify molten.fabric_boundary.non_claims]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_fabric_reports_bind_non_claims_and_reference_scope() {
        let boundary = canonical_fabric_boundary(&default_fabric_boundary_descriptor()).expect("boundary artifact");
        let evidence = canonical_fabric_evidence_profile(&default_production_evidence_profile(LIMIT_REF))
            .expect("evidence artifact");
        let references =
            canonical_reference_matrix_suite(&default_reference_system_matrices()).expect("reference artifact");
        let boundary_text = crate::preserves_rail::to_text(&boundary.value).expect("boundary text");
        let reference_text = crate::preserves_rail::to_text(&references.value).expect("reference text");

        assert!(boundary.boundary_ref.starts_with("blake3:"));
        assert!(evidence.profile_ref.starts_with("blake3:"));
        assert!(references.suite_ref.starts_with("blake3:"));
        assert!(boundary_text.contains("workload-neutral-distributed-systems-fabric"));
        assert!(boundary_text.contains("does-not-prove-global-consensus"));
        assert!(reference_text.contains("transactional-key-value"));
        assert!(reference_text.contains("replicated-log"));
        assert!(reference_text.contains("distributed-scheduler"));
        assert!(reference_text.contains("system-extension"));
    }

    // r[verify molten.fabric_boundary.extension_tiers]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_tier_admission_denies_plugin_system_authority() {
        let request = ExtensionTierRequest {
            tier: ExtensionTier::SandboxedPlugin,
            requested_authorities: vec![FabricAuthority::Consistency],
            admission_evidence: Vec::new(),
        };

        let error = canonical_extension_tier_admission(&request).expect_err("plugin authority must deny");

        assert!(error.to_string().contains("AuthorityRequiresSystemExtension"));
    }

    // r[verify molten.fabric_boundary.port_registry]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_binding_denies_silent_profile_substitution() {
        let mut request = requirement();
        request.expected_profile = "different-profile-v1".to_string();

        let error = resolve_canonical_fabric_port_binding(&[descriptor()], &request)
            .expect_err("profile substitution must deny");

        assert!(error.to_string().contains("SilentProfileSubstitution"));
    }

    // r[verify molten.fabric_boundary.port_registry]
    // r[verify molten.fabric_boundary.final_validation]
    #[test]
    fn canonical_port_identity_is_independent_of_set_and_registry_input_order() {
        let mut ordered = descriptor();
        ordered.operation_classes.push("receive-envelope".to_string());
        ordered.resource_requirements.push(FabricResource::Concurrency);
        let mut reordered = ordered.clone();
        reordered.operation_classes.reverse();
        reordered.resource_requirements.reverse();

        let ordered_ref = canonical_fabric_port_descriptor(&ordered).expect("ordered descriptor").0;
        let reordered_ref = canonical_fabric_port_descriptor(&reordered).expect("reordered descriptor").0;
        assert_eq!(ordered_ref, reordered_ref);

        let mut time_descriptor = descriptor();
        time_descriptor.port_id = "molten.fabric.time.logical".to_string();
        time_descriptor.class = FabricPortClass::Time;
        time_descriptor.operation_classes = vec!["schedule-logical-deadline".to_string()];
        time_descriptor.input_schema_refs = vec!["molten.fabric.time-schedule.v1".to_string()];
        time_descriptor.output_schema_refs = vec!["molten.fabric.time-event.v1".to_string()];
        time_descriptor.authority_requirements = vec![FabricAuthority::Time];
        time_descriptor.resource_requirements = vec![FabricResource::LogicalTime];
        time_descriptor.implementation_profile = "logical-time-v1".to_string();

        let forward = resolve_canonical_fabric_port_binding(&[descriptor(), time_descriptor.clone()], &requirement())
            .expect("forward registry");
        let reversed = resolve_canonical_fabric_port_binding(&[time_descriptor, descriptor()], &requirement())
            .expect("reversed registry");

        assert_eq!(forward.registry_ref, reversed.registry_ref);
        assert_eq!(forward.binding_ref, reversed.binding_ref);
    }
}
