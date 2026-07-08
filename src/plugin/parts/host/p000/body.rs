type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

use crate::bounded::{DiagnosticSink, PushLimited};

pub const PLUGIN_HOST_ABI_VERSION: &str = "molten.plugin.host-abi.v1";

const MAX_PLUGIN_CALLBACKS: usize = 16;
const MAX_PLUGIN_REFS: usize = 4096;
const MAX_PLUGIN_DIAGNOSTICS: usize = 256;
const MAX_PLUGIN_CHECKS: usize = 64;
const MAX_PLUGIN_HOSTCALL_DESCRIPTORS: usize = 128;
const PLUGIN_MANIFEST_BASE_ARITY: usize = 12;
const PLUGIN_MANIFEST_EXTENSION_ARITY: usize = 13;
const PLUGIN_HOSTCALL_RECEIPT_ARITY: usize = 14;
const PLUGIN_CAPABILITY_GRANT_ARITY: usize = 13;
const PLUGIN_CAPABILITY_GRANT_SUBJECT_ARITY: usize = 3;
const PLUGIN_CAPABILITY_GRANT_HOSTCALL_ARITY: usize = 4;
const PLUGIN_CAPABILITY_GRANT_RESOURCE_ARITY: usize = 2;
const PLUGIN_CAPABILITY_GRANT_EFFECTS_ARITY: usize = 2;
const PLUGIN_CAPABILITY_GRANT_REVOCATION_ARITY: usize = 2;
const PLUGIN_CAPABILITY_GRANT_ATTENUATION_ARITY: usize = 5;
const PLUGIN_CAPABILITY_GRANT_VALIDITY_ARITY: usize = 2;
const PLUGIN_EXTENSION_CONTRACT_ARITY: usize = 11;
const PLUGIN_HOSTCALL_DESCRIPTOR_ARITY: usize = 9;
const PLUGIN_CONFORMANCE_ARITY: usize = 3;
const PLUGIN_NEGOTIATION_RECEIPT_ARITY: usize = 9;
const PLUGIN_COMPATIBILITY_RECEIPT_ARITY: usize = 11;
const PLUGIN_SEMVER_PARTS: usize = 3;
const PLUGIN_INITIAL_TURN: u64 = 0;
const PLUGIN_LIFECYCLE_INSTALL_MISSING: &str = "plugin lifecycle install receipt missing";
const PLUGIN_LIFECYCLE_INSTALL_FAILED: &str = "plugin lifecycle install receipt did not pass";
const PLUGIN_LIFECYCLE_PERMISSION_MISSING: &str = "plugin lifecycle permission receipt missing";
const PLUGIN_LIFECYCLE_PERMISSION_FAILED: &str = "plugin lifecycle permission receipt did not pass";
const PLUGIN_LIFECYCLE_PERMISSION_BINDING_MISMATCH: &str = "plugin lifecycle permission binding mismatch";
const PLUGIN_LIFECYCLE_ACTIVATION_MISSING: &str = "plugin lifecycle activation receipt missing";
const PLUGIN_LIFECYCLE_ACTIVATION_FAILED: &str = "plugin lifecycle activation receipt did not pass";
const PLUGIN_LIFECYCLE_ACTIVATION_BINDING_MISMATCH: &str = "plugin lifecycle activation binding mismatch";
const PLUGIN_LIFECYCLE_HOSTCALL_FAILED: &str = "plugin lifecycle hostcall receipt did not pass";
const PLUGIN_LIFECYCLE_HOSTCALL_BINDING_MISMATCH: &str = "plugin lifecycle hostcall binding mismatch";
const PLUGIN_LIFECYCLE_HOSTCALL_UNDECLARED: &str = "plugin lifecycle hostcall is not declared by manifest";
const PLUGIN_LIFECYCLE_HEALTH_FAILED: &str = "plugin lifecycle failed health blocks further use";
const PLUGIN_LIFECYCLE_UPGRADE_FAILED: &str = "plugin lifecycle upgrade receipt did not pass";
const PLUGIN_LIFECYCLE_UPGRADE_BINDING_MISMATCH: &str = "plugin lifecycle upgrade manifest binding mismatch";
const PLUGIN_LIFECYCLE_REMOVAL_FAILED: &str = "plugin lifecycle removal cleanup incomplete";
const PLUGIN_LIFECYCLE_REMOVAL_BINDING_MISMATCH: &str = "plugin lifecycle removal binding mismatch";
const PLUGIN_LIFECYCLE_AUTHORITY_CLOSED: &str = "plugin lifecycle authority closed by removal";
const PLUGIN_LIFECYCLE_NEGOTIATION_MISSING: &str = "plugin lifecycle extension negotiation receipt missing";
const PLUGIN_LIFECYCLE_NEGOTIATION_FAILED: &str = "plugin lifecycle extension negotiation receipt did not pass";
const PLUGIN_LIFECYCLE_NEGOTIATION_BINDING_MISMATCH: &str = "plugin lifecycle extension negotiation manifest binding mismatch";
const PLUGIN_LIFECYCLE_COMPATIBILITY_MISSING: &str = "plugin lifecycle extension compatibility receipt missing";
const PLUGIN_LIFECYCLE_COMPATIBILITY_FAILED: &str = "plugin lifecycle extension compatibility receipt did not pass";
const PLUGIN_LIFECYCLE_COMPATIBILITY_BINDING_MISMATCH: &str = "plugin lifecycle extension compatibility manifest binding mismatch";
const PLUGIN_LIFECYCLE_ACTIVATION_OPERATION: &str = "start";
const PLUGIN_DECISION_PASS: &str = "pass";
const PLUGIN_DECISION_DENY: &str = "deny";
const PLUGIN_CHECK_FAIL: &str = "fail";
const PLUGIN_PROFILE_PRODUCTION: &str = "production";
const PLUGIN_PROFILE_DEVELOPMENT: &str = "development";
const _: () = assert!(MAX_PLUGIN_CALLBACKS > 0);
const _: () = assert!(MAX_PLUGIN_REFS > MAX_PLUGIN_CALLBACKS);
const _: () = assert!(MAX_PLUGIN_DIAGNOSTICS > 0);
const _: () = assert!(MAX_PLUGIN_CHECKS > 0);
const _: () = assert!(MAX_PLUGIN_HOSTCALL_DESCRIPTORS > 0);
const _: () = assert!(PLUGIN_MANIFEST_EXTENSION_ARITY > PLUGIN_MANIFEST_BASE_ARITY);

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginManifestInput<'a> {
    pub plugin_id: &'a str,
    pub artifact_ref: &'a str,
    pub abi: &'a str,
    pub lifecycle_callbacks: &'a [String],
    pub effect_manifest_refs: &'a [String],
    pub hostcall_refs: &'a [String],
    pub schema_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub supply_chain_refs: &'a [String],
    pub extension_contract_refs: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionReviewInput<'a> {
    pub manifest_value: &'a IoValue,
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub effect_receipt_refs: &'a [String],
    pub supply_chain_refs: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleReceiptInput<'a> {
    pub operation: &'a str,
    pub manifest_value: &'a IoValue,
    pub permission_receipt_ref: &'a str,
    pub executor_receipt_ref: &'a str,
    pub authority_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub effect_receipt_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostcallReceiptInput<'a> {
    pub manifest_value: &'a IoValue,
    pub operation: &'a str,
    pub hostcall_ref: &'a str,
    pub executor_receipt_ref: &'a str,
    pub effect_receipt_ref: &'a str,
    pub authority_refs: &'a [String],
    pub capability_grants: &'a [PluginCapabilityGrant],
    pub resource_refs: &'a [String],
    pub extension_contracts: &'a [PluginExtensionContract],
    pub input_schema_ref: Option<&'a str>,
    pub output_schema_ref: Option<&'a str>,
    pub evaluation_turn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthReceiptInput<'a> {
    pub manifest_value: &'a IoValue,
    pub lifecycle_receipt_ref: &'a str,
    pub service_refs: &'a [String],
    pub health_status: &'a str,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpgradeReceiptInput<'a> {
    pub old_manifest_value: &'a IoValue,
    pub new_manifest_value: &'a IoValue,
    pub rollback_ref: &'a str,
    pub cleanup_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovalReceiptInput<'a> {
    pub manifest_value: &'a IoValue,
    pub lifecycle_receipt_ref: &'a str,
    pub owned_service_refs: &'a [String],
    pub assertion_refs: &'a [String],
    pub handle_refs: &'a [String],
    pub catalog_entry_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginLifecycleEvaluationKind {
    CompleteTrace,
    ActivationRequest,
    HostcallRequest,
    UpgradeRequest,
    RemovalRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginLifecycleStateInput<'a> {
    pub evaluation_kind: PluginLifecycleEvaluationKind,
    pub manifest: &'a PluginManifest,
    pub install: Option<&'a PluginInstallReceipt>,
    pub permission: Option<&'a PluginPermissionReceipt>,
    pub activation: Option<&'a PluginLifecycleReceipt>,
    pub hostcall: Option<&'a PluginHostcallReceipt>,
    pub health: Option<&'a PluginHealthReceipt>,
    pub removal: Option<&'a PluginRemovalReceipt>,
    pub upgrade: Option<&'a PluginUpgradeReceipt>,
    pub negotiation: Option<&'a PluginExtensionNegotiationReceipt>,
    pub compatibility: Option<&'a PluginExtensionCompatibilityReceipt>,
    pub recovery_receipt_ref: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostAbiResultInput<'a> {
    pub status: &'a str,
    pub payload_ref: Option<&'a str>,
    pub error: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifest {
    pub manifest_ref: String,
    pub plugin_ref: String,
    pub plugin_id: String,
    pub artifact_ref: String,
    pub abi: String,
    pub lifecycle_callbacks: Vec<String>,
    pub effect_manifest_refs: Vec<String>,
    pub hostcall_refs: Vec<String>,
    pub schema_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub supply_chain_refs: Vec<String>,
    pub extension_contract_refs: Vec<String>,
    pub checks: Vec<(String, String)>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstallReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub artifact_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPermissionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginLifecycleReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHostcallReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub hostcall_ref: String,
    pub capability_grant_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrantRef {
    value: String,
}

impl CapabilityGrantRef {
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginCapabilityGrantAttenuationInput<'a> {
    pub delegated_scope: &'a str,
    pub current_delegation_depth: u64,
    pub max_delegation_depth: u64,
    pub budget_refs: &'a [String],
    pub valid_from_turn: u64,
    pub valid_until_turn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginCapabilityGrantInput<'a> {
    pub plugin_ref: &'a str,
    pub plugin_id: &'a str,
    pub manifest_ref: &'a str,
    pub extension_contract_ref: Option<&'a str>,
    pub hostcall_descriptor_ref: &'a str,
    pub operation: &'a str,
    pub input_schema_ref: &'a str,
    pub output_schema_ref: &'a str,
    pub resource_refs: &'a [String],
    pub resource_scope: &'a str,
    pub effect_manifest_refs: &'a [String],
    pub effect_receipt_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub issuer_ref: &'a str,
    pub proof_refs: &'a [String],
    pub attenuation: PluginCapabilityGrantAttenuationInput<'a>,
    pub revocation_refs: &'a [String],
    pub revoked: bool,
    pub replay_class: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCapabilityGrantAttenuation {
    pub delegated_scope: String,
    pub current_delegation_depth: u64,
    pub max_delegation_depth: u64,
    pub budget_refs: Vec<String>,
    pub valid_from_turn: u64,
    pub valid_until_turn: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCapabilityGrant {
    pub grant_ref: String,
    pub typed_ref: CapabilityGrantRef,
    pub plugin_ref: String,
    pub plugin_id: String,
    pub manifest_ref: String,
    pub extension_contract_ref: Option<String>,
    pub hostcall_descriptor_ref: String,
    pub operation: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub resource_refs: Vec<String>,
    pub resource_scope: String,
    pub effect_manifest_refs: Vec<String>,
    pub effect_receipt_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub issuer_ref: String,
    pub proof_refs: Vec<String>,
    pub attenuation: PluginCapabilityGrantAttenuation,
    pub revocation_refs: Vec<String>,
    pub revoked: bool,
    pub replay_class: String,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginReferenceRole {
    CapabilityGrant,
    OtherArtifact,
}

pub fn classify_plugin_reference_value(value: &IoValue) -> PluginReferenceRole {
    if parse_plugin_capability_grant(value).is_ok() {
        PluginReferenceRole::CapabilityGrant
    } else {
        PluginReferenceRole::OtherArtifact
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHealthReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRemovalReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginUpgradeReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub old_manifest_ref: String,
    pub new_manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginHostcallDescriptorInput<'a> {
    pub operation: &'a str,
    pub descriptor_ref: &'a str,
    pub input_schema_ref: &'a str,
    pub output_schema_ref: &'a str,
    pub authority_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub effect_manifest_refs: &'a [String],
    pub replay_class: &'a str,
    pub error_class_refs: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginExtensionConformanceInput<'a> {
    pub positive_suite_ref: &'a str,
    pub negative_suite_ref: &'a str,
    pub property_suite_ref: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginExtensionContractInput<'a> {
    pub extension_id: &'a str,
    pub version: &'a str,
    pub compatible_host_abi: &'a str,
    pub lifecycle_callbacks: &'a [String],
    pub hostcall_descriptors: &'a [PluginHostcallDescriptorInput<'a>],
    pub conformance: PluginExtensionConformanceInput<'a>,
    pub policy_refs: &'a [String],
    pub supply_chain_refs: &'a [String],
    pub production_profile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHostcallDescriptor {
    pub operation: String,
    pub descriptor_ref: String,
    pub input_schema_ref: String,
    pub output_schema_ref: String,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_manifest_refs: Vec<String>,
    pub replay_class: String,
    pub error_class_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginExtensionConformance {
    pub positive_suite_ref: String,
    pub negative_suite_ref: String,
    pub property_suite_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginExtensionContract {
    pub contract_ref: String,
    pub extension_id: String,
    pub version: String,
    pub compatible_host_abi: String,
    pub lifecycle_callbacks: Vec<String>,
    pub hostcall_descriptors: Vec<PluginHostcallDescriptor>,
    pub conformance: PluginExtensionConformance,
    pub policy_refs: Vec<String>,
    pub supply_chain_refs: Vec<String>,
    pub production_profile: bool,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginExtensionNegotiationInput<'a> {
    pub manifest: &'a PluginManifest,
    pub required_contract_refs: &'a [String],
    pub optional_contract_refs: &'a [String],
    pub host_supported_contract_refs: &'a [String],
    pub host_feature_snapshot_ref: &'a str,
    pub extension_contracts: &'a [PluginExtensionContract],
    pub production_profile: bool,
    pub allow_optional_omission: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginExtensionNegotiationReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub manifest_ref: String,
    pub required_contract_refs: Vec<String>,
    pub optional_contract_refs: Vec<String>,
    pub selected_contract_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginExtensionCompatibilityInput<'a> {
    pub old_manifest: &'a PluginManifest,
    pub new_manifest: &'a PluginManifest,
    pub old_contracts: &'a [PluginExtensionContract],
    pub new_contracts: &'a [PluginExtensionContract],
    pub migration_refs: &'a [String],
    pub rollback_ref: &'a str,
    pub cleanup_refs: &'a [String],
    pub production_profile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginExtensionCompatibilityReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub old_manifest_ref: String,
    pub new_manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginLifecycleStateDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub side_effect_authorized: bool,
    pub authority_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginFixtureRun {
    pub decision: String,
    pub manifest_ref: String,
    pub install_receipt_ref: String,
    pub permission_receipt_ref: String,
    pub start_receipt_ref: String,
    pub hostcall_receipt_ref: String,
    pub health_receipt_ref: String,
    pub stop_receipt_ref: String,
    pub removal_receipt_ref: String,
    pub upgrade_receipt_ref: String,
    pub report_value: IoValue,
    pub evidence_values: Vec<IoValue>,
}

pub fn plugin_manifest_value(input: &PluginManifestInput<'_>) -> Result<IoValue> {
    validate_plugin_id(input.plugin_id)?;
    validate_ref(input.artifact_ref, "plugin artifact ref")?;
    validate_abi(input.abi)?;
    validate_lifecycle_callbacks(input.lifecycle_callbacks)?;
    require_non_empty_refs(input.effect_manifest_refs, "plugin effect manifest refs")?;
    require_non_empty_refs(input.hostcall_refs, "plugin hostcall refs")?;
    require_non_empty_refs(input.schema_refs, "plugin schema refs")?;
    require_non_empty_refs(input.policy_refs, "plugin policy refs")?;
    require_non_empty_refs(input.resource_refs, "plugin resource refs")?;
    require_non_empty_refs(input.supply_chain_refs, "plugin supply-chain refs")?;
    validate_refs(input.extension_contract_refs, "plugin extension contract refs")?;
    Ok(record("plugin-manifest-v1", vec![
        string(crate::preserves_rail::PLUGIN_MANIFEST_SCHEMA),
        record("plugin-id", vec![string(input.plugin_id)]),
        record("artifact", vec![string(input.artifact_ref)]),
        record("abi", vec![string(input.abi)]),
        record("lifecycle", vec![strings_sequence(input.lifecycle_callbacks)]),
        record("effects", vec![refs_sequence(input.effect_manifest_refs)]),
        record("hostcalls", vec![refs_sequence(input.hostcall_refs)]),
        record("schemas", vec![refs_sequence(input.schema_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("supply-chain", vec![refs_sequence(input.supply_chain_refs)]),
        record("extension-contracts", vec![refs_sequence(input.extension_contract_refs)]),
        checks_value(&[
            ("artifact-backed", PLUGIN_DECISION_PASS),
            ("host-abi-version", PLUGIN_DECISION_PASS),
            ("declared-lifecycle", PLUGIN_DECISION_PASS),
            ("declared-effects", PLUGIN_DECISION_PASS),
            ("declared-hostcalls", PLUGIN_DECISION_PASS),
            ("extension-contracts-bound", PLUGIN_DECISION_PASS),
            ("explicit-policy", PLUGIN_DECISION_PASS),
            ("explicit-resource", PLUGIN_DECISION_PASS),
            ("supply-chain-bound", PLUGIN_DECISION_PASS),
            ("no-ambient-authority", PLUGIN_DECISION_PASS),
        ]),
    ]))
}
