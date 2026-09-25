type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

use crate::bounded::PushLimited;

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
