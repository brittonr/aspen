use std::collections::BTreeSet;

use super::FabricAuthority;
use super::FabricNonClaim;
use super::MAX_FABRIC_COLLECTION_ITEMS;
use super::MAX_FABRIC_PORTS;
use super::has_duplicates;
use super::valid_blake3_ref;
use super::valid_fabric_token;
use super::validate_required_non_claims;

pub const FABRIC_PORT_DESCRIPTOR_SCHEMA: &str = "molten.fabric.port-descriptor.v1";
pub const FABRIC_PORT_REGISTRY_SCHEMA: &str = "molten.fabric.port-registry.v1";
pub const FABRIC_PORT_BINDING_SCHEMA: &str = "molten.fabric.port-binding.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricPortClass {
    Authority,
    Transport,
    DurableState,
    Time,
    Scheduling,
    Membership,
    Placement,
    Consistency,
    Supervision,
    Policy,
    Resources,
    Simulation,
    Evidence,
}

impl FabricPortClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authority => "authority",
            Self::Transport => "transport",
            Self::DurableState => "durable-state",
            Self::Time => "time",
            Self::Scheduling => "scheduling",
            Self::Membership => "membership",
            Self::Placement => "placement",
            Self::Consistency => "consistency",
            Self::Supervision => "supervision",
            Self::Policy => "policy",
            Self::Resources => "resources",
            Self::Simulation => "simulation",
            Self::Evidence => "evidence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeterminismClass {
    Pure,
    DeterministicWithRecordedInputs,
    ExternalEffect,
}

impl DeterminismClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pure => "pure",
            Self::DeterministicWithRecordedInputs => "deterministic-with-recorded-inputs",
            Self::ExternalEffect => "external-effect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplayClass {
    Recompute,
    RecordedEffectRequired,
    EvidenceOnly,
    NotReplayable,
}

impl ReplayClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Recompute => "recompute",
            Self::RecordedEffectRequired => "recorded-effect-required",
            Self::EvidenceOnly => "evidence-only",
            Self::NotReplayable => "not-replayable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FabricResource {
    Memory,
    StorageBytes,
    NetworkBytes,
    Concurrency,
    QueueDepth,
    LogicalTime,
    Diagnostics,
}

impl FabricResource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::StorageBytes => "storage-bytes",
            Self::NetworkBytes => "network-bytes",
            Self::Concurrency => "concurrency",
            Self::QueueDepth => "queue-depth",
            Self::LogicalTime => "logical-time",
            Self::Diagnostics => "diagnostics",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FabricPortKey {
    pub port_id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricPortDescriptor {
    pub schema: String,
    pub port_id: String,
    pub version: String,
    pub class: FabricPortClass,
    pub operation_classes: Vec<String>,
    pub input_schema_refs: Vec<String>,
    pub output_schema_refs: Vec<String>,
    pub authority_requirements: Vec<FabricAuthority>,
    pub resource_requirements: Vec<FabricResource>,
    pub determinism: DeterminismClass,
    pub replay: ReplayClass,
    pub implementation_profile: String,
    pub conformance_refs: Vec<String>,
    pub non_claims: Vec<FabricNonClaim>,
    pub enabled: bool,
}

impl FabricPortDescriptor {
    pub fn key(&self) -> FabricPortKey {
        FabricPortKey {
            port_id: self.port_id.clone(),
            version: self.version.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricPortRequirement {
    pub port_id: String,
    pub version: String,
    pub class: FabricPortClass,
    pub operation_classes: Vec<String>,
    pub input_schema_refs: Vec<String>,
    pub output_schema_refs: Vec<String>,
    pub allowed_authorities: Vec<FabricAuthority>,
    pub available_resources: Vec<FabricResource>,
    pub expected_determinism: DeterminismClass,
    pub expected_replay: ReplayClass,
    pub expected_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricPortBinding {
    pub key: FabricPortKey,
    pub class: FabricPortClass,
    pub implementation_profile: String,
    pub conformance_refs: Vec<String>,
    pub non_claims: Vec<FabricNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricPortRegistry {
    descriptors: Vec<FabricPortDescriptor>,
}

impl FabricPortRegistry {
    pub fn descriptors(&self) -> &[FabricPortDescriptor] {
        &self.descriptors
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FabricPortIssue {
    EmptyRegistry,
    TooManyPorts {
        actual: usize,
        maximum: usize,
    },
    DescriptorSchemaMismatch {
        actual: String,
        expected: String,
    },
    EmptyField(&'static str),
    MalformedField {
        field: &'static str,
        value: String,
    },
    TooManyFieldValues {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    DuplicateFieldValue(&'static str),
    MalformedConformanceRef(String),
    MissingNonClaim(FabricNonClaim),
    DuplicatePort(FabricPortKey),
    UnknownPort(String),
    UnsupportedVersion {
        port_id: String,
        requested: String,
        available: Vec<String>,
    },
    DisabledPort(FabricPortKey),
    ClassMismatch {
        expected: FabricPortClass,
        actual: FabricPortClass,
    },
    MissingOperation(String),
    SchemaSetMismatch(&'static str),
    OverAuthorizingPort(FabricAuthority),
    ResourceUnavailable(FabricResource),
    DeterminismMismatch {
        expected: DeterminismClass,
        actual: DeterminismClass,
    },
    ReplayMismatch {
        expected: ReplayClass,
        actual: ReplayClass,
    },
    SilentProfileSubstitution {
        expected: String,
        actual: String,
    },
}

// r[impl molten.fabric_boundary.port_registry]
pub fn build_fabric_port_registry(
    descriptors: &[FabricPortDescriptor],
) -> Result<FabricPortRegistry, Vec<FabricPortIssue>> {
    let mut issues = Vec::new();
    if descriptors.is_empty() {
        issues.push(FabricPortIssue::EmptyRegistry);
    }
    if descriptors.len() > MAX_FABRIC_PORTS {
        issues.push(FabricPortIssue::TooManyPorts {
            actual: descriptors.len(),
            maximum: MAX_FABRIC_PORTS,
        });
    }

    let mut keys = BTreeSet::new();
    for descriptor in descriptors {
        validate_descriptor(descriptor, &mut issues);
        let key = descriptor.key();
        if !keys.insert(key.clone()) {
            issues.push(FabricPortIssue::DuplicatePort(key));
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut normalized = descriptors.iter().cloned().map(normalize_descriptor).collect::<Vec<_>>();
    normalized.sort_by_key(FabricPortDescriptor::key);
    Ok(FabricPortRegistry {
        descriptors: normalized,
    })
}

// r[impl molten.fabric_boundary.port_registry]
pub fn resolve_fabric_port_binding(
    registry: &FabricPortRegistry,
    requirement: &FabricPortRequirement,
) -> Result<FabricPortBinding, Vec<FabricPortIssue>> {
    let mut issues = Vec::new();
    validate_requirement(requirement, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    let matching_id = registry
        .descriptors
        .iter()
        .filter(|descriptor| descriptor.port_id == requirement.port_id)
        .collect::<Vec<_>>();
    if matching_id.is_empty() {
        return Err(vec![FabricPortIssue::UnknownPort(requirement.port_id.clone())]);
    }

    let Some(descriptor) = matching_id.iter().copied().find(|descriptor| descriptor.version == requirement.version)
    else {
        let mut available = matching_id.iter().map(|descriptor| descriptor.version.clone()).collect::<Vec<_>>();
        available.sort();
        available.dedup();
        return Err(vec![FabricPortIssue::UnsupportedVersion {
            port_id: requirement.port_id.clone(),
            requested: requirement.version.clone(),
            available,
        }]);
    };

    validate_binding_compatibility(descriptor, requirement, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    Ok(FabricPortBinding {
        key: descriptor.key(),
        class: descriptor.class,
        implementation_profile: descriptor.implementation_profile.clone(),
        conformance_refs: descriptor.conformance_refs.clone(),
        non_claims: descriptor.non_claims.clone(),
    })
}

fn validate_descriptor(descriptor: &FabricPortDescriptor, issues: &mut Vec<FabricPortIssue>) {
    if descriptor.schema != FABRIC_PORT_DESCRIPTOR_SCHEMA {
        issues.push(FabricPortIssue::DescriptorSchemaMismatch {
            actual: descriptor.schema.clone(),
            expected: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
        });
    }
    validate_token_field("port-id", &descriptor.port_id, issues);
    validate_token_field("version", &descriptor.version, issues);
    validate_token_field("implementation-profile", &descriptor.implementation_profile, issues);
    validate_token_list("operation-classes", &descriptor.operation_classes, true, issues);
    validate_token_list("input-schema-refs", &descriptor.input_schema_refs, true, issues);
    validate_token_list("output-schema-refs", &descriptor.output_schema_refs, true, issues);
    validate_enum_list("authority-requirements", &descriptor.authority_requirements, issues);
    validate_enum_list("resource-requirements", &descriptor.resource_requirements, issues);
    validate_conformance_refs(&descriptor.conformance_refs, issues);
    validate_descriptor_non_claims(&descriptor.non_claims, issues);
}

fn validate_requirement(requirement: &FabricPortRequirement, issues: &mut Vec<FabricPortIssue>) {
    validate_token_field("required-port-id", &requirement.port_id, issues);
    validate_token_field("required-version", &requirement.version, issues);
    validate_token_field("expected-profile", &requirement.expected_profile, issues);
    validate_token_list("required-operation-classes", &requirement.operation_classes, true, issues);
    validate_token_list("required-input-schema-refs", &requirement.input_schema_refs, true, issues);
    validate_token_list("required-output-schema-refs", &requirement.output_schema_refs, true, issues);
    validate_enum_list("allowed-authorities", &requirement.allowed_authorities, issues);
    validate_enum_list("available-resources", &requirement.available_resources, issues);
}

fn validate_binding_compatibility(
    descriptor: &FabricPortDescriptor,
    requirement: &FabricPortRequirement,
    issues: &mut Vec<FabricPortIssue>,
) {
    if !descriptor.enabled {
        issues.push(FabricPortIssue::DisabledPort(descriptor.key()));
    }
    if descriptor.class != requirement.class {
        issues.push(FabricPortIssue::ClassMismatch {
            expected: requirement.class,
            actual: descriptor.class,
        });
    }
    for operation in &requirement.operation_classes {
        if !descriptor.operation_classes.contains(operation) {
            issues.push(FabricPortIssue::MissingOperation(operation.clone()));
        }
    }
    if sorted_strings(&descriptor.input_schema_refs) != sorted_strings(&requirement.input_schema_refs) {
        issues.push(FabricPortIssue::SchemaSetMismatch("input-schema-refs"));
    }
    if sorted_strings(&descriptor.output_schema_refs) != sorted_strings(&requirement.output_schema_refs) {
        issues.push(FabricPortIssue::SchemaSetMismatch("output-schema-refs"));
    }
    for authority in &descriptor.authority_requirements {
        if !requirement.allowed_authorities.contains(authority) {
            issues.push(FabricPortIssue::OverAuthorizingPort(*authority));
        }
    }
    for resource in &descriptor.resource_requirements {
        if !requirement.available_resources.contains(resource) {
            issues.push(FabricPortIssue::ResourceUnavailable(*resource));
        }
    }
    if descriptor.determinism != requirement.expected_determinism {
        issues.push(FabricPortIssue::DeterminismMismatch {
            expected: requirement.expected_determinism,
            actual: descriptor.determinism,
        });
    }
    if descriptor.replay != requirement.expected_replay {
        issues.push(FabricPortIssue::ReplayMismatch {
            expected: requirement.expected_replay,
            actual: descriptor.replay,
        });
    }
    if descriptor.implementation_profile != requirement.expected_profile {
        issues.push(FabricPortIssue::SilentProfileSubstitution {
            expected: requirement.expected_profile.clone(),
            actual: descriptor.implementation_profile.clone(),
        });
    }
}

fn validate_token_field(field: &'static str, value: &str, issues: &mut Vec<FabricPortIssue>) {
    if value.is_empty() {
        issues.push(FabricPortIssue::EmptyField(field));
        return;
    }
    if !valid_fabric_token(value) {
        issues.push(FabricPortIssue::MalformedField {
            field,
            value: value.to_string(),
        });
    }
}

fn validate_token_list(field: &'static str, values: &[String], nonempty: bool, issues: &mut Vec<FabricPortIssue>) {
    if nonempty && values.is_empty() {
        issues.push(FabricPortIssue::EmptyField(field));
    }
    if values.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricPortIssue::TooManyFieldValues {
            field,
            actual: values.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(values) {
        issues.push(FabricPortIssue::DuplicateFieldValue(field));
    }
    for value in values {
        if !valid_fabric_token(value) {
            issues.push(FabricPortIssue::MalformedField {
                field,
                value: value.clone(),
            });
        }
    }
}

fn validate_enum_list<T: Ord>(field: &'static str, values: &[T], issues: &mut Vec<FabricPortIssue>) {
    if values.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricPortIssue::TooManyFieldValues {
            field,
            actual: values.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(values) {
        issues.push(FabricPortIssue::DuplicateFieldValue(field));
    }
}

fn validate_conformance_refs(refs: &[String], issues: &mut Vec<FabricPortIssue>) {
    if refs.is_empty() {
        issues.push(FabricPortIssue::EmptyField("conformance-refs"));
    }
    if refs.len() > MAX_FABRIC_COLLECTION_ITEMS {
        issues.push(FabricPortIssue::TooManyFieldValues {
            field: "conformance-refs",
            actual: refs.len(),
            maximum: MAX_FABRIC_COLLECTION_ITEMS,
        });
    }
    if has_duplicates(refs) {
        issues.push(FabricPortIssue::DuplicateFieldValue("conformance-refs"));
    }
    for reference in refs {
        if !valid_blake3_ref(reference) {
            issues.push(FabricPortIssue::MalformedConformanceRef(reference.clone()));
        }
    }
}

fn validate_descriptor_non_claims(non_claims: &[FabricNonClaim], issues: &mut Vec<FabricPortIssue>) {
    validate_enum_list("non-claims", non_claims, issues);
    validate_required_non_claims(non_claims, |missing| {
        issues.push(FabricPortIssue::MissingNonClaim(missing));
    });
}

fn normalize_descriptor(mut descriptor: FabricPortDescriptor) -> FabricPortDescriptor {
    descriptor.operation_classes.sort();
    descriptor.input_schema_refs.sort();
    descriptor.output_schema_refs.sort();
    descriptor.authority_requirements.sort();
    descriptor.resource_requirements.sort();
    descriptor.conformance_refs.sort();
    descriptor.non_claims.sort();
    descriptor
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fabric::REQUIRED_FABRIC_NON_CLAIMS;

    const CONFORMANCE_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TRANSPORT_PORT_ID: &str = "molten.fabric.transport.session";
    const TRANSPORT_VERSION: &str = "v1";
    const LIVE_PROFILE: &str = "iroh-live-v1";
    const OTHER_PROFILE: &str = "unreviewed-fallback-v1";
    const SEND_OPERATION: &str = "send-envelope";
    const INPUT_SCHEMA: &str = "molten.fabric.transport-send.v1";
    const OUTPUT_SCHEMA: &str = "molten.fabric.transport-outcome.v1";

    fn valid_descriptor() -> FabricPortDescriptor {
        FabricPortDescriptor {
            schema: FABRIC_PORT_DESCRIPTOR_SCHEMA.to_string(),
            port_id: TRANSPORT_PORT_ID.to_string(),
            version: TRANSPORT_VERSION.to_string(),
            class: FabricPortClass::Transport,
            operation_classes: vec![SEND_OPERATION.to_string()],
            input_schema_refs: vec![INPUT_SCHEMA.to_string()],
            output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
            authority_requirements: vec![FabricAuthority::Transport],
            resource_requirements: vec![FabricResource::NetworkBytes, FabricResource::Concurrency],
            determinism: DeterminismClass::ExternalEffect,
            replay: ReplayClass::RecordedEffectRequired,
            implementation_profile: LIVE_PROFILE.to_string(),
            conformance_refs: vec![CONFORMANCE_REF.to_string()],
            non_claims: REQUIRED_FABRIC_NON_CLAIMS.to_vec(),
            enabled: true,
        }
    }

    fn valid_requirement() -> FabricPortRequirement {
        FabricPortRequirement {
            port_id: TRANSPORT_PORT_ID.to_string(),
            version: TRANSPORT_VERSION.to_string(),
            class: FabricPortClass::Transport,
            operation_classes: vec![SEND_OPERATION.to_string()],
            input_schema_refs: vec![INPUT_SCHEMA.to_string()],
            output_schema_refs: vec![OUTPUT_SCHEMA.to_string()],
            allowed_authorities: vec![FabricAuthority::Transport],
            available_resources: vec![FabricResource::Concurrency, FabricResource::NetworkBytes],
            expected_determinism: DeterminismClass::ExternalEffect,
            expected_replay: ReplayClass::RecordedEffectRequired,
            expected_profile: LIVE_PROFILE.to_string(),
        }
    }

    // r[verify molten.fabric_boundary.port_registry]
    #[test]
    fn compatible_unique_port_resolves_exact_reviewed_profile() {
        let registry = build_fabric_port_registry(&[valid_descriptor()]).expect("valid registry");
        let binding = resolve_fabric_port_binding(&registry, &valid_requirement()).expect("compatible binding");

        assert_eq!(registry.descriptors().len(), 1);
        assert_eq!(binding.key.port_id, TRANSPORT_PORT_ID);
        assert_eq!(binding.key.version, TRANSPORT_VERSION);
        assert_eq!(binding.implementation_profile, LIVE_PROFILE);
        assert_eq!(binding.conformance_refs, vec![CONFORMANCE_REF.to_string()]);
        assert_eq!(binding.non_claims, REQUIRED_FABRIC_NON_CLAIMS);
    }

    // r[verify molten.fabric_boundary.port_registry]
    #[test]
    fn registry_rejects_duplicate_port_keys_even_when_profiles_differ() {
        let first = valid_descriptor();
        let mut second = valid_descriptor();
        second.implementation_profile = OTHER_PROFILE.to_string();

        let issues = build_fabric_port_registry(&[first, second]).expect_err("duplicate key must deny");

        assert!(issues.contains(&FabricPortIssue::DuplicatePort(FabricPortKey {
            port_id: TRANSPORT_PORT_ID.to_string(),
            version: TRANSPORT_VERSION.to_string(),
        })));
    }

    // r[verify molten.fabric_boundary.port_registry]
    #[test]
    fn resolution_rejects_unknown_port_and_unsupported_version_without_fallback() {
        let registry = build_fabric_port_registry(&[valid_descriptor()]).expect("valid registry");
        let mut unknown = valid_requirement();
        unknown.port_id = "molten.fabric.unknown".to_string();
        assert_eq!(
            resolve_fabric_port_binding(&registry, &unknown),
            Err(vec![FabricPortIssue::UnknownPort("molten.fabric.unknown".to_string())])
        );

        let mut unsupported = valid_requirement();
        unsupported.version = "v2".to_string();
        assert_eq!(
            resolve_fabric_port_binding(&registry, &unsupported),
            Err(vec![FabricPortIssue::UnsupportedVersion {
                port_id: TRANSPORT_PORT_ID.to_string(),
                requested: "v2".to_string(),
                available: vec![TRANSPORT_VERSION.to_string()],
            }])
        );
    }

    // r[verify molten.fabric_boundary.port_registry]
    #[test]
    fn resolution_rejects_profile_substitution_over_authority_and_disabled_port() {
        let mut descriptor = valid_descriptor();
        descriptor.authority_requirements.push(FabricAuthority::ProtocolOwnership);
        descriptor.enabled = false;
        let registry = build_fabric_port_registry(&[descriptor]).expect("well-formed disabled descriptor");
        let mut requirement = valid_requirement();
        requirement.expected_profile = OTHER_PROFILE.to_string();

        let issues = resolve_fabric_port_binding(&registry, &requirement).expect_err("incompatible port must deny");

        assert!(issues.contains(&FabricPortIssue::DisabledPort(FabricPortKey {
            port_id: TRANSPORT_PORT_ID.to_string(),
            version: TRANSPORT_VERSION.to_string(),
        })));
        assert!(issues.contains(&FabricPortIssue::OverAuthorizingPort(FabricAuthority::ProtocolOwnership)));
        assert!(issues.contains(&FabricPortIssue::SilentProfileSubstitution {
            expected: OTHER_PROFILE.to_string(),
            actual: LIVE_PROFILE.to_string(),
        }));
    }

    // r[verify molten.fabric_boundary.port_registry]
    #[test]
    fn registry_rejects_malformed_conformance_refs_and_missing_non_claims() {
        let mut descriptor = valid_descriptor();
        descriptor.conformance_refs = vec!["sha256:not-canonical".to_string()];
        descriptor.non_claims.retain(|claim| *claim != FabricNonClaim::ProtocolCompatibility);

        let issues = build_fabric_port_registry(&[descriptor]).expect_err("malformed descriptor must deny");

        assert!(issues.contains(&FabricPortIssue::MalformedConformanceRef("sha256:not-canonical".to_string())));
        assert!(issues.contains(&FabricPortIssue::MissingNonClaim(FabricNonClaim::ProtocolCompatibility)));
    }
}
