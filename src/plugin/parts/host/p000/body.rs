type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

use crate::bounded::VecSink;

pub const PLUGIN_HOST_ABI_VERSION: &str = "molten.plugin.host-abi.v1";

const MAX_PLUGIN_CALLBACKS: usize = 16;
const MAX_PLUGIN_REFS: usize = 4096;
const MAX_PLUGIN_DIAGNOSTICS: usize = 256;
const MAX_PLUGIN_CHECKS: usize = 64;
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
const PLUGIN_LIFECYCLE_ACTIVATION_OPERATION: &str = "start";
const _: () = assert!(MAX_PLUGIN_CALLBACKS > 0);
const _: () = assert!(MAX_PLUGIN_REFS > MAX_PLUGIN_CALLBACKS);
const _: () = assert!(MAX_PLUGIN_DIAGNOSTICS > 0);
const _: () = assert!(MAX_PLUGIN_CHECKS > 0);

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
    pub resource_refs: &'a [String],
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
    pub hostcall_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHealthReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRemovalReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
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
        checks_value(&[
            ("artifact-backed", "pass"),
            ("host-abi-version", "pass"),
            ("declared-lifecycle", "pass"),
            ("declared-effects", "pass"),
            ("declared-hostcalls", "pass"),
            ("explicit-policy", "pass"),
            ("explicit-resource", "pass"),
            ("supply-chain-bound", "pass"),
            ("no-ambient-authority", "pass"),
        ]),
    ]))
}
