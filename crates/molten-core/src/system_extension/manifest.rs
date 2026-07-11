use std::collections::BTreeSet;

use super::CallbackKind;
use super::ExecutionProfile;
use super::INITIAL_SYSTEM_EXTENSION_GENERATION;
use super::MAX_SYSTEM_EXTENSION_ITEMS;
use super::REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS;
use super::ResourceEnvelope;
use super::ResourceIssue;
use super::SYSTEM_EXTENSION_MANIFEST_SCHEMA;
use super::SystemExtensionNonClaim;
use super::duplicates;
use super::valid_ref;
use super::valid_token;
use super::validate_resource_envelope;
use crate::fabric::ExtensionTier;
use crate::fabric::ExtensionTierAdmission;
use crate::fabric::FabricAuthority;
use crate::fabric::FabricPortBinding;
use crate::fabric::FabricPortIssue;
use crate::fabric::FabricPortKey;
use crate::fabric::FabricPortRegistry;
use crate::fabric::FabricPortRequirement;
use crate::fabric::REQUIRED_SYSTEM_EXTENSION_EVIDENCE;
use crate::fabric::resolve_fabric_port_binding;

const REQUIRED_LIFECYCLE_CALLBACK_COUNT: usize = 4;

pub const REQUIRED_LIFECYCLE_CALLBACKS: [CallbackKind; REQUIRED_LIFECYCLE_CALLBACK_COUNT] = [
    CallbackKind::Initialize,
    CallbackKind::Start,
    CallbackKind::Drain,
    CallbackKind::Shutdown,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemExtensionManifestInput {
    pub schema: String,
    pub extension_id: String,
    pub service_id: String,
    pub implementation_ref: String,
    pub callback_groups: Vec<String>,
    pub required_ports: Vec<FabricPortRequirement>,
    pub optional_ports: Vec<FabricPortRequirement>,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub resources: ResourceEnvelope,
    pub execution_profile: ExecutionProfile,
    pub state_schema: String,
    pub compatible_state_schemas: Vec<String>,
    pub evidence_profile_ref: String,
    pub initial_generation: u64,
    pub non_claims: Vec<SystemExtensionNonClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedSystemExtensionManifest {
    pub extension_id: String,
    pub service_id: String,
    pub implementation_ref: String,
    pub callbacks: Vec<CallbackKind>,
    pub required_port_requirements: Vec<FabricPortRequirement>,
    pub optional_port_requirements: Vec<FabricPortRequirement>,
    pub required_port_bindings: Vec<FabricPortBinding>,
    pub optional_port_bindings: Vec<FabricPortBinding>,
    pub capability_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub resources: ResourceEnvelope,
    pub execution_profile: ExecutionProfile,
    pub state_schema: String,
    pub compatible_state_schemas: Vec<String>,
    pub evidence_profile_ref: String,
    pub initial_generation: u64,
    pub non_claims: Vec<SystemExtensionNonClaim>,
}

impl AdmittedSystemExtensionManifest {
    pub fn declares_callback(&self, callback: CallbackKind) -> bool {
        self.callbacks.contains(&callback)
    }

    pub fn all_port_requirements(&self) -> impl Iterator<Item = &FabricPortRequirement> {
        self.required_port_requirements.iter().chain(self.optional_port_requirements.iter())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SystemExtensionAdmissionContext<'a> {
    pub registry: &'a FabricPortRegistry,
    pub tier_admission: &'a ExtensionTierAdmission,
    pub admitted_execution_profiles: &'a [ExecutionProfile],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestIssue {
    SchemaMismatch {
        actual: String,
        expected: String,
    },
    MalformedIdentifier {
        field: &'static str,
        value: String,
    },
    MalformedRef {
        field: &'static str,
        value: String,
    },
    EmptyRefSet(&'static str),
    TooManyItems {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    DuplicateItem(&'static str),
    UnknownCallback(String),
    MissingLifecycleCallback(CallbackKind),
    MissingRequiredPort,
    DuplicatePortKey(FabricPortKey),
    RequiredPortDenied {
        key: FabricPortKey,
        issues: Vec<FabricPortIssue>,
    },
    OptionalPortDenied {
        key: FabricPortKey,
        issues: Vec<FabricPortIssue>,
    },
    PortAuthorityNotAdmitted {
        key: FabricPortKey,
        authority: FabricAuthority,
    },
    WrongExtensionTier(ExtensionTier),
    IncompleteTierAdmission,
    ExecutionProfileNotAdmitted(ExecutionProfile),
    StateSchemaNotSelfCompatible(String),
    InitialGenerationMismatch {
        actual: u64,
        expected: u64,
    },
    Resource(ResourceIssue),
    MissingNonClaim(SystemExtensionNonClaim),
}

// r[impl molten.system_extension.manifest]
// r[impl molten.system_extension.execution_profiles]
pub fn admit_system_extension_manifest(
    input: &SystemExtensionManifestInput,
    context: SystemExtensionAdmissionContext<'_>,
) -> Result<AdmittedSystemExtensionManifest, Vec<ManifestIssue>> {
    let mut issues = Vec::new();
    validate_manifest_identity(input, &mut issues);
    let callbacks = validate_callbacks(input, &mut issues);
    validate_tier(context.tier_admission, &mut issues);
    validate_profile(input, context.admitted_execution_profiles, &mut issues);
    validate_refs(input, &mut issues);
    validate_state(input, &mut issues);
    validate_non_claims(input, &mut issues);
    for issue in validate_resource_envelope(&input.resources) {
        issues.push(ManifestIssue::Resource(issue));
    }
    let (required_bindings, optional_bindings) = validate_ports(input, context, &mut issues);
    if !issues.is_empty() {
        return Err(issues);
    }

    let mut callbacks = callbacks;
    callbacks.sort();
    let mut required_port_requirements = input.required_ports.clone();
    required_port_requirements.sort_by_key(requirement_key);
    let mut optional_port_requirements = input.optional_ports.clone();
    optional_port_requirements.sort_by_key(requirement_key);
    let mut capability_refs = sorted(input.capability_refs.clone());
    let mut policy_refs = sorted(input.policy_refs.clone());
    let mut provenance_refs = sorted(input.provenance_refs.clone());
    let mut compatible_state_schemas = sorted(input.compatible_state_schemas.clone());
    let mut non_claims = input.non_claims.clone();
    non_claims.sort();
    capability_refs.dedup();
    policy_refs.dedup();
    provenance_refs.dedup();
    compatible_state_schemas.dedup();
    Ok(AdmittedSystemExtensionManifest {
        extension_id: input.extension_id.clone(),
        service_id: input.service_id.clone(),
        implementation_ref: input.implementation_ref.clone(),
        callbacks,
        required_port_requirements,
        optional_port_requirements,
        required_port_bindings: required_bindings,
        optional_port_bindings: optional_bindings,
        capability_refs,
        policy_refs,
        provenance_refs,
        resources: input.resources.clone(),
        execution_profile: input.execution_profile,
        state_schema: input.state_schema.clone(),
        compatible_state_schemas,
        evidence_profile_ref: input.evidence_profile_ref.clone(),
        initial_generation: input.initial_generation,
        non_claims,
    })
}

fn validate_manifest_identity(input: &SystemExtensionManifestInput, issues: &mut Vec<ManifestIssue>) {
    if input.schema != SYSTEM_EXTENSION_MANIFEST_SCHEMA {
        issues.push(ManifestIssue::SchemaMismatch {
            actual: input.schema.clone(),
            expected: SYSTEM_EXTENSION_MANIFEST_SCHEMA.to_string(),
        });
    }
    for (field, value) in [
        ("extension-id", input.extension_id.as_str()),
        ("service-id", input.service_id.as_str()),
    ] {
        if !valid_token(value) {
            issues.push(ManifestIssue::MalformedIdentifier {
                field,
                value: value.to_string(),
            });
        }
    }
    if !valid_ref(&input.implementation_ref) {
        issues.push(ManifestIssue::MalformedRef {
            field: "implementation-ref",
            value: input.implementation_ref.clone(),
        });
    }
    if input.initial_generation != INITIAL_SYSTEM_EXTENSION_GENERATION {
        issues.push(ManifestIssue::InitialGenerationMismatch {
            actual: input.initial_generation,
            expected: INITIAL_SYSTEM_EXTENSION_GENERATION,
        });
    }
}

fn validate_callbacks(input: &SystemExtensionManifestInput, issues: &mut Vec<ManifestIssue>) -> Vec<CallbackKind> {
    validate_count("callback-groups", input.callback_groups.len(), issues);
    if duplicates(&input.callback_groups) {
        issues.push(ManifestIssue::DuplicateItem("callback-groups"));
    }
    let mut callbacks = Vec::new();
    for callback in &input.callback_groups {
        match CallbackKind::parse(callback) {
            Some(callback) => callbacks.push(callback),
            None => issues.push(ManifestIssue::UnknownCallback(callback.clone())),
        }
    }
    for required in REQUIRED_LIFECYCLE_CALLBACKS {
        if !callbacks.contains(&required) {
            issues.push(ManifestIssue::MissingLifecycleCallback(required));
        }
    }
    callbacks
}

fn validate_tier(admission: &ExtensionTierAdmission, issues: &mut Vec<ManifestIssue>) {
    if admission.tier != ExtensionTier::SystemExtension {
        issues.push(ManifestIssue::WrongExtensionTier(admission.tier));
    }
    let complete = REQUIRED_SYSTEM_EXTENSION_EVIDENCE
        .iter()
        .all(|required| admission.supporting_evidence.contains(required));
    if !complete {
        issues.push(ManifestIssue::IncompleteTierAdmission);
    }
}

fn validate_profile(
    input: &SystemExtensionManifestInput,
    admitted_profiles: &[ExecutionProfile],
    issues: &mut Vec<ManifestIssue>,
) {
    if !admitted_profiles.contains(&input.execution_profile) {
        issues.push(ManifestIssue::ExecutionProfileNotAdmitted(input.execution_profile));
    }
}

fn validate_refs(input: &SystemExtensionManifestInput, issues: &mut Vec<ManifestIssue>) {
    validate_ref_set("capability-refs", &input.capability_refs, true, issues);
    validate_ref_set("policy-refs", &input.policy_refs, true, issues);
    validate_ref_set("provenance-refs", &input.provenance_refs, true, issues);
    if !valid_ref(&input.evidence_profile_ref) {
        issues.push(ManifestIssue::MalformedRef {
            field: "evidence-profile-ref",
            value: input.evidence_profile_ref.clone(),
        });
    }
}

fn validate_state(input: &SystemExtensionManifestInput, issues: &mut Vec<ManifestIssue>) {
    if !valid_token(&input.state_schema) {
        issues.push(ManifestIssue::MalformedIdentifier {
            field: "state-schema",
            value: input.state_schema.clone(),
        });
    }
    validate_count("compatible-state-schemas", input.compatible_state_schemas.len(), issues);
    if duplicates(&input.compatible_state_schemas) {
        issues.push(ManifestIssue::DuplicateItem("compatible-state-schemas"));
    }
    for schema in &input.compatible_state_schemas {
        if !valid_token(schema) {
            issues.push(ManifestIssue::MalformedIdentifier {
                field: "compatible-state-schema",
                value: schema.clone(),
            });
        }
    }
    if !input.compatible_state_schemas.contains(&input.state_schema) {
        issues.push(ManifestIssue::StateSchemaNotSelfCompatible(input.state_schema.clone()));
    }
}

fn validate_non_claims(input: &SystemExtensionManifestInput, issues: &mut Vec<ManifestIssue>) {
    validate_count("non-claims", input.non_claims.len(), issues);
    if duplicates(&input.non_claims) {
        issues.push(ManifestIssue::DuplicateItem("non-claims"));
    }
    for required in REQUIRED_SYSTEM_EXTENSION_NON_CLAIMS {
        if !input.non_claims.contains(&required) {
            issues.push(ManifestIssue::MissingNonClaim(required));
        }
    }
}

fn validate_ports(
    input: &SystemExtensionManifestInput,
    context: SystemExtensionAdmissionContext<'_>,
    issues: &mut Vec<ManifestIssue>,
) -> (Vec<FabricPortBinding>, Vec<FabricPortBinding>) {
    if input.required_ports.is_empty() {
        issues.push(ManifestIssue::MissingRequiredPort);
    }
    validate_count("required-ports", input.required_ports.len(), issues);
    validate_count("optional-ports", input.optional_ports.len(), issues);
    validate_port_keys(input, issues);

    let mut required = Vec::new();
    for requirement in &input.required_ports {
        validate_port_authority(requirement, context.tier_admission, issues);
        match resolve_fabric_port_binding(context.registry, requirement) {
            Ok(binding) => required.push(binding),
            Err(port_issues) => issues.push(ManifestIssue::RequiredPortDenied {
                key: requirement_key(requirement),
                issues: port_issues,
            }),
        }
    }

    let mut optional = Vec::new();
    for requirement in &input.optional_ports {
        validate_port_authority(requirement, context.tier_admission, issues);
        let is_available =
            context.registry.descriptors().iter().any(|descriptor| descriptor.port_id == requirement.port_id);
        if !is_available {
            continue;
        }
        match resolve_fabric_port_binding(context.registry, requirement) {
            Ok(binding) => optional.push(binding),
            Err(port_issues) => issues.push(ManifestIssue::OptionalPortDenied {
                key: requirement_key(requirement),
                issues: port_issues,
            }),
        }
    }
    required.sort_by(|left, right| left.key.cmp(&right.key));
    optional.sort_by(|left, right| left.key.cmp(&right.key));
    (required, optional)
}

fn validate_port_keys(input: &SystemExtensionManifestInput, issues: &mut Vec<ManifestIssue>) {
    let mut keys = BTreeSet::new();
    for requirement in input.required_ports.iter().chain(input.optional_ports.iter()) {
        let key = requirement_key(requirement);
        if !keys.insert(key.clone()) {
            issues.push(ManifestIssue::DuplicatePortKey(key));
        }
    }
}

fn validate_port_authority(
    requirement: &FabricPortRequirement,
    admission: &ExtensionTierAdmission,
    issues: &mut Vec<ManifestIssue>,
) {
    for authority in &requirement.allowed_authorities {
        if !admission.admitted_authorities.contains(authority) {
            issues.push(ManifestIssue::PortAuthorityNotAdmitted {
                key: requirement_key(requirement),
                authority: *authority,
            });
        }
    }
}

fn validate_ref_set(field: &'static str, refs: &[String], require_nonempty: bool, issues: &mut Vec<ManifestIssue>) {
    if require_nonempty && refs.is_empty() {
        issues.push(ManifestIssue::EmptyRefSet(field));
    }
    validate_count(field, refs.len(), issues);
    if duplicates(refs) {
        issues.push(ManifestIssue::DuplicateItem(field));
    }
    for reference in refs {
        if !valid_ref(reference) {
            issues.push(ManifestIssue::MalformedRef {
                field,
                value: reference.clone(),
            });
        }
    }
}

fn validate_count(field: &'static str, actual: usize, issues: &mut Vec<ManifestIssue>) {
    if actual > MAX_SYSTEM_EXTENSION_ITEMS {
        issues.push(ManifestIssue::TooManyItems {
            field,
            actual,
            maximum: MAX_SYSTEM_EXTENSION_ITEMS,
        });
    }
}

fn requirement_key(requirement: &FabricPortRequirement) -> FabricPortKey {
    FabricPortKey {
        port_id: requirement.port_id.clone(),
        version: requirement.version.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateMigrationPlan {
    pub source_schema: String,
    pub target_schema: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateMigrationIssue {
    MalformedSchema { field: &'static str, value: String },
    SourceSchemaNotCompatible(String),
    TargetSchemaMismatch { actual: String, expected: String },
}

// r[impl molten.system_extension.lifecycle]
pub fn plan_state_migration(
    manifest: &AdmittedSystemExtensionManifest,
    source_schema: &str,
    target_schema: &str,
) -> Result<StateMigrationPlan, Vec<StateMigrationIssue>> {
    let mut issues = Vec::new();
    for (field, schema) in [("source-schema", source_schema), ("target-schema", target_schema)] {
        if !valid_token(schema) {
            issues.push(StateMigrationIssue::MalformedSchema {
                field,
                value: schema.to_string(),
            });
        }
    }
    if !manifest.compatible_state_schemas.iter().any(|schema| schema == source_schema) {
        issues.push(StateMigrationIssue::SourceSchemaNotCompatible(source_schema.to_string()));
    }
    if target_schema != manifest.state_schema {
        issues.push(StateMigrationIssue::TargetSchemaMismatch {
            actual: target_schema.to_string(),
            expected: manifest.state_schema.clone(),
        });
    }
    if issues.is_empty() {
        Ok(StateMigrationPlan {
            source_schema: source_schema.to_string(),
            target_schema: target_schema.to_string(),
        })
    } else {
        Err(issues)
    }
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}
