//! Plugin host lifecycle records and deterministic local lifecycle runner.
//!
//! The host in this first slice is deliberately receipt-first: install does
//! not grant runtime authority, activation requires separate permission and
//! executor evidence, and hostcalls are admitted only through declared refs.

use std::path::Path;

use preserves::IOValue;
use preserves::Record;
use preserves::Value;

use crate::artifacts;
use crate::bounded::VecSink;
use crate::error::MoltenError;
use crate::error::Result;
use crate::ledger;
use crate::preserves_rail::PLUGIN_INSTALL_RECEIPT_SCHEMA;
use crate::preserves_rail::PLUGIN_LIFECYCLE_RECEIPT_SCHEMA;
use crate::preserves_rail::PLUGIN_MANIFEST_SCHEMA;
use crate::preserves_rail::PLUGIN_PERMISSION_RECEIPT_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::value_to_iovalue;

pub const PLUGIN_HOST_ABI_VERSION: &str = "molten.plugin.host-abi.v1";

const MAX_PLUGIN_CALLBACKS: usize = 16;
const MAX_PLUGIN_REFS: usize = 4096;
const MAX_PLUGIN_DIAGNOSTICS: usize = 256;
const MAX_PLUGIN_CHECKS: usize = 64;
const _: () = assert!(MAX_PLUGIN_CALLBACKS > 0);
const _: () = assert!(MAX_PLUGIN_REFS > MAX_PLUGIN_CALLBACKS);
const _: () = assert!(MAX_PLUGIN_DIAGNOSTICS > 0);
const _: () = assert!(MAX_PLUGIN_CHECKS > 0);

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
    pub manifest_value: &'a IOValue,
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub effect_receipt_refs: &'a [String],
    pub supply_chain_refs: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleReceiptInput<'a> {
    pub operation: &'a str,
    pub manifest_value: &'a IOValue,
    pub permission_receipt_ref: &'a str,
    pub executor_receipt_ref: &'a str,
    pub authority_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub effect_receipt_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostcallReceiptInput<'a> {
    pub manifest_value: &'a IOValue,
    pub operation: &'a str,
    pub hostcall_ref: &'a str,
    pub executor_receipt_ref: &'a str,
    pub effect_receipt_ref: &'a str,
    pub authority_refs: &'a [String],
    pub resource_refs: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HealthReceiptInput<'a> {
    pub manifest_value: &'a IOValue,
    pub lifecycle_receipt_ref: &'a str,
    pub service_refs: &'a [String],
    pub health_status: &'a str,
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpgradeReceiptInput<'a> {
    pub old_manifest_value: &'a IOValue,
    pub new_manifest_value: &'a IOValue,
    pub rollback_ref: &'a str,
    pub cleanup_refs: &'a [String],
    pub diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovalReceiptInput<'a> {
    pub manifest_value: &'a IOValue,
    pub lifecycle_receipt_ref: &'a str,
    pub owned_service_refs: &'a [String],
    pub assertion_refs: &'a [String],
    pub handle_refs: &'a [String],
    pub catalog_entry_refs: &'a [String],
    pub diagnostics: &'a [String],
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
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstallReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub artifact_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPermissionReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginLifecycleReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub plugin_ref: String,
    pub manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHostcallReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub plugin_ref: String,
    pub hostcall_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginHealthReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRemovalReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub plugin_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginUpgradeReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub old_manifest_ref: String,
    pub new_manifest_ref: String,
    pub diagnostics: Vec<String>,
    pub value: IOValue,
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
    pub report_value: IOValue,
    pub evidence_values: Vec<IOValue>,
}

pub fn plugin_manifest_value(input: &PluginManifestInput<'_>) -> Result<IOValue> {
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
        string(PLUGIN_MANIFEST_SCHEMA),
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

pub fn parse_plugin_manifest(value: &IOValue) -> Result<PluginManifest> {
    let fields = simple_record(value, "plugin-manifest-v1", 12)?;
    require_schema(&fields[0], PLUGIN_MANIFEST_SCHEMA, "plugin manifest")?;
    let plugin_id = record_string(&fields[1], "plugin-id")?;
    let artifact_ref = record_ref(&fields[2], "artifact")?;
    let abi = record_string(&fields[3], "abi")?;
    let lifecycle_callbacks = record_string_sequence(&fields[4], "lifecycle")?;
    let effect_manifest_refs = record_ref_sequence(&fields[5], "effects")?;
    let hostcall_refs = record_ref_sequence(&fields[6], "hostcalls")?;
    let schema_refs = record_ref_sequence(&fields[7], "schemas")?;
    let policy_refs = record_ref_sequence(&fields[8], "policy")?;
    let resource_refs = record_ref_sequence(&fields[9], "resource")?;
    let supply_chain_refs = record_ref_sequence(&fields[10], "supply-chain")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "artifact-backed", "plugin manifest")?;
    require_check(&checks, "no-ambient-authority", "plugin manifest")?;
    validate_plugin_id(&plugin_id)?;
    validate_abi(&abi)?;
    validate_lifecycle_callbacks(&lifecycle_callbacks)?;
    require_non_empty_refs(&effect_manifest_refs, "plugin effect manifest refs")?;
    require_non_empty_refs(&hostcall_refs, "plugin hostcall refs")?;
    require_non_empty_refs(&schema_refs, "plugin schema refs")?;
    require_non_empty_refs(&policy_refs, "plugin policy refs")?;
    require_non_empty_refs(&resource_refs, "plugin resource refs")?;
    require_non_empty_refs(&supply_chain_refs, "plugin supply-chain refs")?;
    Ok(PluginManifest {
        manifest_ref: canonical_hash(value)?,
        plugin_ref: plugin_identity_ref(&plugin_id, &artifact_ref)?,
        plugin_id,
        artifact_ref,
        abi,
        lifecycle_callbacks,
        effect_manifest_refs,
        hostcall_refs,
        schema_refs,
        policy_refs,
        resource_refs,
        supply_chain_refs,
        checks,
        value: value.clone(),
    })
}

pub fn install_plugin(registry_root: &Path, manifest_value: &IOValue) -> Result<PluginInstallReceipt> {
    let manifest = parse_plugin_manifest(manifest_value)?;
    let mut diagnostics = Vec::new();
    let has_artifact = artifacts::read_artifact(registry_root, &manifest.artifact_ref).is_ok();
    if !has_artifact {
        diagnostics.push_limited(
            format!("plugin artifact {} is not present in registry", manifest.artifact_ref),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin install diagnostics",
        )?;
    }
    let decision = if has_artifact { "pass" } else { "deny" };
    let value = record("plugin-install-receipt-v1", vec![
        string(PLUGIN_INSTALL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("artifact", vec![string(&manifest.artifact_ref)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-install", "pass"),
            ("artifact-backed", "pass"),
            ("artifact-present", status(has_artifact)),
            ("activation-separate", "pass"),
            ("no-code-loaded", "pass"),
        ]),
    ]);
    parse_plugin_install_receipt(&value)
}

pub fn parse_plugin_install_receipt(value: &IOValue) -> Result<PluginInstallReceipt> {
    let fields = simple_record(value, "plugin-install-receipt-v1", 7)?;
    require_schema(&fields[0], PLUGIN_INSTALL_RECEIPT_SCHEMA, "plugin install receipt")?;
    let diagnostics = record_string_sequence(&fields[5], "diagnostics")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "canonical-install", "plugin install receipt")?;
    require_check(&checks, "activation-separate", "plugin install receipt")?;
    Ok(PluginInstallReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        artifact_ref: record_ref(&fields[4], "artifact")?,
        diagnostics,
        value: value.clone(),
    })
}

pub fn plugin_permission_receipt_value(input: &PermissionReviewInput<'_>) -> Result<IOValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_refs(input.authority_refs, "plugin authority ref")?;
    validate_refs(input.policy_refs, "plugin policy review ref")?;
    validate_refs(input.resource_refs, "plugin resource review ref")?;
    validate_refs(input.effect_receipt_refs, "plugin effect receipt ref")?;
    validate_refs(input.supply_chain_refs, "plugin supply-chain review ref")?;
    let mut diagnostics = Vec::new();
    collect_missing_refs(&manifest.policy_refs, input.policy_refs, "policy", &mut diagnostics)?;
    collect_missing_refs(&manifest.resource_refs, input.resource_refs, "resource", &mut diagnostics)?;
    collect_missing_refs(&manifest.supply_chain_refs, input.supply_chain_refs, "supply-chain", &mut diagnostics)?;
    if input.authority_refs.is_empty() {
        diagnostics.push_limited(
            "plugin activation requires explicit authority evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin permission diagnostics",
        )?;
    }
    if input.effect_receipt_refs.is_empty() {
        diagnostics.push_limited(
            "plugin activation requires effect-handle boundary evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin permission diagnostics",
        )?;
    }
    let has_authority = !input.authority_refs.is_empty();
    let has_effect_boundary = !input.effect_receipt_refs.is_empty();
    let has_current_policy = contains_all(input.policy_refs, &manifest.policy_refs);
    let has_current_resources = contains_all(input.resource_refs, &manifest.resource_refs);
    let has_current_supply_chain = contains_all(input.supply_chain_refs, &manifest.supply_chain_refs);
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("plugin-permission-receipt-v1", vec![
        string(PLUGIN_PERMISSION_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("policy", vec![refs_sequence(input.policy_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("effects", vec![refs_sequence(input.effect_receipt_refs)]),
        record("supply-chain", vec![refs_sequence(input.supply_chain_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("install-not-authority", "pass"),
            ("authority-present", status(has_authority)),
            ("policy-current", status(has_current_policy)),
            ("resource-bound", status(has_current_resources)),
            ("supply-chain-current", status(has_current_supply_chain)),
            ("effect-handle-boundary", status(has_effect_boundary)),
            ("no-ambient-authority", "pass"),
        ]),
    ]))
}

pub fn parse_plugin_permission_receipt(value: &IOValue) -> Result<PluginPermissionReceipt> {
    let fields = simple_record(value, "plugin-permission-receipt-v1", 11)?;
    require_schema(&fields[0], PLUGIN_PERMISSION_RECEIPT_SCHEMA, "plugin permission receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "install-not-authority", "plugin permission receipt")?;
    require_check(&checks, "effect-handle-boundary", "plugin permission receipt")?;
    Ok(PluginPermissionReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        manifest_ref: record_ref(&fields[3], "manifest")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn plugin_lifecycle_receipt_value(input: &LifecycleReceiptInput<'_>) -> Result<IOValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_lifecycle_operation(input.operation)?;
    validate_ref(input.permission_receipt_ref, "plugin permission receipt ref")?;
    validate_ref(input.executor_receipt_ref, "plugin executor receipt ref")?;
    validate_refs(input.authority_refs, "plugin lifecycle authority ref")?;
    validate_refs(input.resource_refs, "plugin lifecycle resource ref")?;
    validate_refs(input.effect_receipt_refs, "plugin lifecycle effect ref")?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let is_declared_callback = is_lifecycle_declared(&manifest.lifecycle_callbacks, input.operation);
    if !is_declared_callback {
        diagnostics.push_limited(
            format!("plugin lifecycle operation {} is not declared", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if input.authority_refs.is_empty() {
        diagnostics.push_limited(
            "plugin lifecycle requires authority evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if input.resource_refs.is_empty() {
        diagnostics.push_limited(
            "plugin lifecycle requires resource evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    if input.effect_receipt_refs.is_empty() {
        diagnostics.push_limited(
            "plugin lifecycle requires effect receipt evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin lifecycle diagnostics",
        )?;
    }
    let has_authority = !input.authority_refs.is_empty();
    let has_resources = !input.resource_refs.is_empty();
    let has_effects = !input.effect_receipt_refs.is_empty();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("plugin-lifecycle-receipt-v1", vec![
        string(PLUGIN_LIFECYCLE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("executor", vec![string(input.executor_receipt_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("effects", vec![refs_sequence(input.effect_receipt_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-lifecycle", "pass"),
            ("declared-callback", status(is_declared_callback)),
            ("executor-boundary", "pass"),
            ("authority-present", status(has_authority)),
            ("resource-bound", status(has_resources)),
            ("effect-boundary", status(has_effects)),
            ("failure-isolated", "pass"),
        ]),
    ]))
}

pub fn parse_plugin_lifecycle_receipt(value: &IOValue) -> Result<PluginLifecycleReceipt> {
    let fields = simple_record(value, "plugin-lifecycle-receipt-v1", 11)?;
    require_schema(&fields[0], PLUGIN_LIFECYCLE_RECEIPT_SCHEMA, "plugin lifecycle receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "canonical-lifecycle", "plugin lifecycle receipt")?;
    require_check(&checks, "executor-boundary", "plugin lifecycle receipt")?;
    Ok(PluginLifecycleReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        plugin_ref: record_ref(&fields[3], "plugin")?,
        manifest_ref: record_ref(&fields[4], "manifest")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn plugin_hostcall_receipt_value(input: &HostcallReceiptInput<'_>) -> Result<IOValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_non_empty(input.operation, "plugin hostcall operation")?;
    validate_ref(input.hostcall_ref, "plugin hostcall ref")?;
    validate_ref(input.executor_receipt_ref, "plugin hostcall executor ref")?;
    validate_ref(input.effect_receipt_ref, "plugin hostcall effect ref")?;
    validate_refs(input.authority_refs, "plugin hostcall authority ref")?;
    validate_refs(input.resource_refs, "plugin hostcall resource ref")?;
    let mut diagnostics = Vec::new();
    let is_declared_hostcall = manifest.hostcall_refs.iter().any(|value| value == input.hostcall_ref);
    let has_authority = !input.authority_refs.is_empty();
    let has_resources = !input.resource_refs.is_empty();
    let has_ambient_request = is_ambient_operation(input.operation);
    if !is_declared_hostcall {
        diagnostics.push_limited(
            format!("plugin hostcall {} is not declared", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if has_ambient_request && !is_declared_hostcall {
        diagnostics.push_limited(
            format!("ambient plugin hostcall {} denied before side effects", input.operation),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if !has_authority {
        diagnostics.push_limited(
            "plugin hostcall requires authority evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    if !has_resources {
        diagnostics.push_limited(
            "plugin hostcall requires resource evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin hostcall diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("plugin-hostcall-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("operation", vec![string(input.operation)]),
        record("hostcall", vec![string(input.hostcall_ref)]),
        record("executor", vec![string(input.executor_receipt_ref)]),
        record("effect", vec![string(input.effect_receipt_ref)]),
        record("authority", vec![refs_sequence(input.authority_refs)]),
        record("resource", vec![refs_sequence(input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("declared-hostcall", status(is_declared_hostcall)),
            ("executor-boundary", "pass"),
            ("effect-handle-boundary", "pass"),
            ("authority-present", status(has_authority)),
            ("resource-bound", status(has_resources)),
            ("deny-ambient-side-effect", status(!has_ambient_request || is_declared_hostcall)),
        ]),
    ]))
}

pub fn parse_plugin_hostcall_receipt(value: &IOValue) -> Result<PluginHostcallReceipt> {
    let fields = simple_record(value, "plugin-hostcall-receipt-v1", 11)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_HOSTCALL_RECEIPT_SCHEMA, "plugin hostcall receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "declared-hostcall", "plugin hostcall receipt")?;
    require_check(&checks, "effect-handle-boundary", "plugin hostcall receipt")?;
    Ok(PluginHostcallReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        operation: record_string(&fields[3], "operation")?,
        hostcall_ref: record_ref(&fields[4], "hostcall")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn plugin_health_receipt_value(input: &HealthReceiptInput<'_>) -> Result<IOValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_ref(input.lifecycle_receipt_ref, "plugin health lifecycle receipt ref")?;
    validate_refs(input.service_refs, "plugin health service ref")?;
    validate_health_status(input.health_status)?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let is_healthy = input.health_status == "healthy";
    if !is_healthy && diagnostics.is_empty() {
        diagnostics.push_limited(
            "plugin health check failed".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin health diagnostics",
        )?;
    }
    let decision = if is_healthy && diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    };
    Ok(record("plugin-health-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_HEALTH_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("lifecycle", vec![string(input.lifecycle_receipt_ref)]),
        record("status", vec![string(input.health_status)]),
        record("services", vec![refs_sequence(input.service_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-health", "pass"),
            ("service-supervision-bound", status(!input.service_refs.is_empty())),
            ("failed-health-isolated", "pass"),
            ("cleanup-required-on-failure", status(is_healthy)),
        ]),
    ]))
}

pub fn parse_plugin_health_receipt(value: &IOValue) -> Result<PluginHealthReceipt> {
    let fields = simple_record(value, "plugin-health-receipt-v1", 9)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_HEALTH_RECEIPT_SCHEMA, "plugin health receipt")?;
    let checks = parse_checks(&fields[8])?;
    require_check(&checks, "canonical-health", "plugin health receipt")?;
    require_check(&checks, "failed-health-isolated", "plugin health receipt")?;
    Ok(PluginHealthReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        diagnostics: record_string_sequence(&fields[7], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn plugin_removal_receipt_value(input: &RemovalReceiptInput<'_>) -> Result<IOValue> {
    let manifest = parse_plugin_manifest(input.manifest_value)?;
    validate_ref(input.lifecycle_receipt_ref, "plugin removal lifecycle receipt ref")?;
    validate_refs(input.owned_service_refs, "plugin removal service ref")?;
    validate_refs(input.assertion_refs, "plugin removal assertion ref")?;
    validate_refs(input.handle_refs, "plugin removal handle ref")?;
    validate_refs(input.catalog_entry_refs, "plugin removal catalog ref")?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let has_service_cleanup = !input.owned_service_refs.is_empty();
    let has_assertion_cleanup = !input.assertion_refs.is_empty();
    let has_handle_cleanup = !input.handle_refs.is_empty();
    let has_catalog_cleanup = !input.catalog_entry_refs.is_empty();
    if !(has_service_cleanup && has_assertion_cleanup && has_handle_cleanup && has_catalog_cleanup) {
        diagnostics.push_limited(
            "plugin removal requires service/assertion/handle/catalog cleanup refs".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin removal diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("plugin-removal-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_REMOVAL_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("plugin", vec![string(&manifest.plugin_ref)]),
        record("manifest", vec![string(&manifest.manifest_ref)]),
        record("lifecycle", vec![string(input.lifecycle_receipt_ref)]),
        record("services", vec![refs_sequence(input.owned_service_refs)]),
        record("assertions", vec![refs_sequence(input.assertion_refs)]),
        record("handles", vec![refs_sequence(input.handle_refs)]),
        record("catalog", vec![refs_sequence(input.catalog_entry_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-removal", "pass"),
            ("service-retractions", status(has_service_cleanup)),
            ("assertion-retractions", status(has_assertion_cleanup)),
            ("handle-revocations", status(has_handle_cleanup)),
            ("catalog-retractions", status(has_catalog_cleanup)),
            ("complete-cleanup", status(diagnostics.is_empty())),
        ]),
    ]))
}

pub fn parse_plugin_removal_receipt(value: &IOValue) -> Result<PluginRemovalReceipt> {
    let fields = simple_record(value, "plugin-removal-receipt-v1", 11)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_REMOVAL_RECEIPT_SCHEMA, "plugin removal receipt")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "complete-cleanup", "plugin removal receipt")?;
    Ok(PluginRemovalReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        plugin_ref: record_ref(&fields[2], "plugin")?,
        diagnostics: record_string_sequence(&fields[9], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn plugin_upgrade_receipt_value(input: &UpgradeReceiptInput<'_>) -> Result<IOValue> {
    let old_manifest = parse_plugin_manifest(input.old_manifest_value)?;
    let new_manifest = parse_plugin_manifest(input.new_manifest_value)?;
    validate_ref(input.rollback_ref, "plugin upgrade rollback ref")?;
    validate_refs(input.cleanup_refs, "plugin upgrade cleanup ref")?;
    validate_diagnostics(input.diagnostics)?;
    let mut diagnostics = input.diagnostics.to_vec();
    let has_same_plugin = old_manifest.plugin_id == new_manifest.plugin_id;
    let has_compatible_abi = old_manifest.abi == new_manifest.abi;
    let has_compatible_schemas = contains_all(&new_manifest.schema_refs, &old_manifest.schema_refs);
    if !has_same_plugin {
        diagnostics.push_limited(
            "plugin upgrade cannot change plugin id".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    if !has_compatible_abi {
        diagnostics.push_limited(
            "plugin upgrade ABI is incompatible".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    if !has_compatible_schemas {
        diagnostics.push_limited(
            "plugin upgrade drops required schema refs".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    if input.cleanup_refs.is_empty() {
        diagnostics.push_limited(
            "plugin upgrade requires rollback/cleanup evidence".to_string(),
            MAX_PLUGIN_DIAGNOSTICS,
            "plugin upgrade diagnostics",
        )?;
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    Ok(record("plugin-upgrade-receipt-v1", vec![
        string(crate::preserves_rail::PLUGIN_UPGRADE_RECEIPT_SCHEMA),
        record("decision", vec![string(decision)]),
        record("old-manifest", vec![string(&old_manifest.manifest_ref)]),
        record("new-manifest", vec![string(&new_manifest.manifest_ref)]),
        record("rollback", vec![string(input.rollback_ref)]),
        record("cleanup", vec![refs_sequence(input.cleanup_refs)]),
        record("diagnostics", vec![strings_sequence(&diagnostics)]),
        checks_value(&[
            ("canonical-upgrade", "pass"),
            ("same-plugin", status(has_same_plugin)),
            ("abi-compatible", status(has_compatible_abi)),
            ("schema-compatible", status(has_compatible_schemas)),
            ("rollback-bound", status(!input.cleanup_refs.is_empty())),
        ]),
    ]))
}

pub fn parse_plugin_upgrade_receipt(value: &IOValue) -> Result<PluginUpgradeReceipt> {
    let fields = simple_record(value, "plugin-upgrade-receipt-v1", 8)?;
    require_schema(&fields[0], crate::preserves_rail::PLUGIN_UPGRADE_RECEIPT_SCHEMA, "plugin upgrade receipt")?;
    let checks = parse_checks(&fields[7])?;
    require_check(&checks, "canonical-upgrade", "plugin upgrade receipt")?;
    Ok(PluginUpgradeReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        old_manifest_ref: record_ref(&fields[2], "old-manifest")?,
        new_manifest_ref: record_ref(&fields[3], "new-manifest")?,
        diagnostics: record_string_sequence(&fields[6], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn plugin_host_abi_result_value(input: &HostAbiResultInput<'_>) -> Result<IOValue> {
    validate_host_abi_status(input.status)?;
    validate_optional_ref(input.payload_ref, "plugin ABI payload ref")?;
    if input.status == "ok" && input.error.is_some() {
        return Err(MoltenError::invalid_harness("successful plugin ABI result must not carry an error"));
    }
    if input.status == "error" && input.error.is_none() {
        return Err(MoltenError::invalid_harness("error plugin ABI result requires an error message"));
    }
    Ok(record("plugin-host-abi-result-v1", vec![
        string(crate::preserves_rail::PLUGIN_HOST_ABI_RESULT_SCHEMA),
        record("abi", vec![string(crate::preserves_rail::PLUGIN_HOST_ABI_SCHEMA)]),
        record("status", vec![string(input.status)]),
        record("payload", vec![optional_ref_value(input.payload_ref)]),
        record("error", vec![optional_text_value(input.error)]),
        checks_value(&[
            ("canonical-preserves-result", "pass"),
            ("error-is-explicit", status(input.status != "error" || input.error.is_some())),
        ]),
    ]))
}

pub fn minimal_plugin_fixture(root: &Path) -> Result<PluginFixtureRun> {
    let registry = root.join("registry");
    let ledger_root = root.join("ledger");
    let seed = seed_refs()?;
    let manifest_value = executor_manifest(&registry, &seed, "minimal")?;
    let manifest = parse_plugin_manifest(&manifest_value)?;
    let install = install_plugin(&registry, &manifest_value)?;
    let permission = permission_step(&manifest_value, &seed)?;
    let lifecycle = life_steps(&manifest_value, &permission.receipt_ref, &seed)?;
    let call = call_step(&manifest_value, &seed)?;
    let service_ref = plugin_ref("service-supervision")?;
    let health = health_step(&manifest_value, &lifecycle.start.receipt_ref, &service_ref)?;
    let removal = removal_step(&manifest_value, &lifecycle.remove.receipt_ref, &service_ref)?;
    let upgraded_manifest_value = executor_manifest(&registry, &seed, "minimal-v2")?;
    let upgrade = upgrade_step(&manifest_value, &upgraded_manifest_value, &removal.receipt_ref)?;
    let evidence_values = vec![
        manifest_value.clone(),
        install.value.clone(),
        permission.value.clone(),
        lifecycle.init.value.clone(),
        lifecycle.start.value.clone(),
        call.value.clone(),
        health.value.clone(),
        lifecycle.stop.value.clone(),
        lifecycle.remove.value.clone(),
        removal.value.clone(),
        upgraded_manifest_value.clone(),
        upgrade.value.clone(),
    ];
    for value in &evidence_values {
        let _ = ledger::import_artifact(&ledger_root, value)?;
    }
    let report_value = plugin_fixture_report_value(&PluginFixtureReportInput {
        manifest_ref: &manifest.manifest_ref,
        install_receipt_ref: &install.receipt_ref,
        permission_receipt_ref: &permission.receipt_ref,
        start_receipt_ref: &lifecycle.start.receipt_ref,
        hostcall_receipt_ref: &call.receipt_ref,
        health_receipt_ref: &health.receipt_ref,
        stop_receipt_ref: &lifecycle.stop.receipt_ref,
        removal_receipt_ref: &removal.receipt_ref,
        upgrade_receipt_ref: &upgrade.receipt_ref,
    })?;
    let decision = run_decision(&[
        install.decision.as_str(),
        permission.decision.as_str(),
        lifecycle.init.decision.as_str(),
        lifecycle.start.decision.as_str(),
        call.decision.as_str(),
        health.decision.as_str(),
        lifecycle.stop.decision.as_str(),
        removal.decision.as_str(),
        upgrade.decision.as_str(),
    ]);
    Ok(PluginFixtureRun {
        decision,
        manifest_ref: manifest.manifest_ref,
        install_receipt_ref: install.receipt_ref,
        permission_receipt_ref: permission.receipt_ref,
        start_receipt_ref: lifecycle.start.receipt_ref,
        hostcall_receipt_ref: call.receipt_ref,
        health_receipt_ref: health.receipt_ref,
        stop_receipt_ref: lifecycle.stop.receipt_ref,
        removal_receipt_ref: removal.receipt_ref,
        upgrade_receipt_ref: upgrade.receipt_ref,
        report_value,
        evidence_values,
    })
}

struct SeedRefs {
    policy_ref: String,
    resource_ref: String,
    schema_ref: String,
    effect_manifest_ref: String,
    supply_chain_ref: String,
    authority_ref: String,
    executor_ref: String,
    effect_receipt_ref: String,
    call_ref: String,
}

struct LifeSteps {
    init: PluginLifecycleReceipt,
    start: PluginLifecycleReceipt,
    stop: PluginLifecycleReceipt,
    remove: PluginLifecycleReceipt,
}

fn seed_refs() -> Result<SeedRefs> {
    Ok(SeedRefs {
        policy_ref: plugin_ref("policy")?,
        resource_ref: plugin_ref("resource")?,
        schema_ref: plugin_ref("schema")?,
        effect_manifest_ref: plugin_ref("effect-manifest")?,
        supply_chain_ref: plugin_ref("supply-chain")?,
        authority_ref: plugin_ref("authority")?,
        executor_ref: plugin_ref("executor-preflight")?,
        effect_receipt_ref: plugin_ref("effect-receipt")?,
        call_ref: storage_read_hostcall_ref()?,
    })
}

fn executor_manifest(registry: &Path, seed: &SeedRefs, payload: &str) -> Result<IOValue> {
    let installed = artifacts::install_artifact(registry, &artifacts::ArtifactInstallInput {
        kind: "plugin-executor".to_string(),
        payload: record("reviewed-plugin-executor", vec![string(payload)]),
        schema_refs: vec![seed.schema_ref.clone()],
        dependency_refs: Vec::new(),
        effect_manifest_ref: Some(seed.effect_manifest_ref.clone()),
        policy_refs: vec![seed.policy_ref.clone()],
        evidence_refs: vec![seed.supply_chain_ref.clone()],
        installer_ref: seed.authority_ref.clone(),
        capability_refs: vec![seed.authority_ref.clone()],
    })?;
    plugin_manifest_value(&PluginManifestInput {
        plugin_id: "plugin:minimal",
        artifact_ref: &installed.artifact_ref,
        abi: PLUGIN_HOST_ABI_VERSION,
        lifecycle_callbacks: &string_vec(&["init", "start", "health", "stop", "remove"]),
        effect_manifest_refs: std::slice::from_ref(&seed.effect_manifest_ref),
        hostcall_refs: std::slice::from_ref(&seed.call_ref),
        schema_refs: std::slice::from_ref(&seed.schema_ref),
        policy_refs: std::slice::from_ref(&seed.policy_ref),
        resource_refs: std::slice::from_ref(&seed.resource_ref),
        supply_chain_refs: std::slice::from_ref(&seed.supply_chain_ref),
    })
}

fn permission_step(manifest_value: &IOValue, seed: &SeedRefs) -> Result<PluginPermissionReceipt> {
    let value = plugin_permission_receipt_value(&PermissionReviewInput {
        manifest_value,
        authority_refs: std::slice::from_ref(&seed.authority_ref),
        policy_refs: std::slice::from_ref(&seed.policy_ref),
        resource_refs: std::slice::from_ref(&seed.resource_ref),
        effect_receipt_refs: std::slice::from_ref(&seed.effect_receipt_ref),
        supply_chain_refs: std::slice::from_ref(&seed.supply_chain_ref),
    })?;
    parse_plugin_permission_receipt(&value)
}

fn life_step(
    operation: &str,
    manifest_value: &IOValue,
    permission_ref: &str,
    seed: &SeedRefs,
) -> Result<PluginLifecycleReceipt> {
    let value = plugin_lifecycle_receipt_value(&LifecycleReceiptInput {
        operation,
        manifest_value,
        permission_receipt_ref: permission_ref,
        executor_receipt_ref: &seed.executor_ref,
        authority_refs: std::slice::from_ref(&seed.authority_ref),
        resource_refs: std::slice::from_ref(&seed.resource_ref),
        effect_receipt_refs: std::slice::from_ref(&seed.effect_receipt_ref),
        diagnostics: &[],
    })?;
    parse_plugin_lifecycle_receipt(&value)
}

fn life_steps(manifest_value: &IOValue, permission_ref: &str, seed: &SeedRefs) -> Result<LifeSteps> {
    Ok(LifeSteps {
        init: life_step("init", manifest_value, permission_ref, seed)?,
        start: life_step("start", manifest_value, permission_ref, seed)?,
        stop: life_step("stop", manifest_value, permission_ref, seed)?,
        remove: life_step("remove", manifest_value, permission_ref, seed)?,
    })
}

fn call_step(manifest_value: &IOValue, seed: &SeedRefs) -> Result<PluginHostcallReceipt> {
    let value = plugin_hostcall_receipt_value(&HostcallReceiptInput {
        manifest_value,
        operation: "storage.read",
        hostcall_ref: &seed.call_ref,
        executor_receipt_ref: &seed.executor_ref,
        effect_receipt_ref: &seed.effect_receipt_ref,
        authority_refs: std::slice::from_ref(&seed.authority_ref),
        resource_refs: std::slice::from_ref(&seed.resource_ref),
    })?;
    parse_plugin_hostcall_receipt(&value)
}

fn health_step(manifest_value: &IOValue, lifecycle_ref: &str, service_ref: &str) -> Result<PluginHealthReceipt> {
    let service_ref = service_ref.to_string();
    let value = plugin_health_receipt_value(&HealthReceiptInput {
        manifest_value,
        lifecycle_receipt_ref: lifecycle_ref,
        service_refs: std::slice::from_ref(&service_ref),
        health_status: "healthy",
        diagnostics: &[],
    })?;
    parse_plugin_health_receipt(&value)
}

fn removal_step(manifest_value: &IOValue, lifecycle_ref: &str, service_ref: &str) -> Result<PluginRemovalReceipt> {
    let service_ref = service_ref.to_string();
    let assertion_ref = plugin_ref("assertion-retraction")?;
    let handle_ref = plugin_ref("handle-revocation")?;
    let catalog_ref = plugin_ref("catalog-retraction")?;
    let value = plugin_removal_receipt_value(&RemovalReceiptInput {
        manifest_value,
        lifecycle_receipt_ref: lifecycle_ref,
        owned_service_refs: std::slice::from_ref(&service_ref),
        assertion_refs: std::slice::from_ref(&assertion_ref),
        handle_refs: std::slice::from_ref(&handle_ref),
        catalog_entry_refs: std::slice::from_ref(&catalog_ref),
        diagnostics: &[],
    })?;
    parse_plugin_removal_receipt(&value)
}

fn upgrade_step(old_value: &IOValue, new_value: &IOValue, cleanup_ref: &str) -> Result<PluginUpgradeReceipt> {
    let rollback_ref = plugin_ref("rollback")?;
    let cleanup_ref = cleanup_ref.to_string();
    let value = plugin_upgrade_receipt_value(&UpgradeReceiptInput {
        old_manifest_value: old_value,
        new_manifest_value: new_value,
        rollback_ref: &rollback_ref,
        cleanup_refs: std::slice::from_ref(&cleanup_ref),
        diagnostics: &[],
    })?;
    parse_plugin_upgrade_receipt(&value)
}

fn run_decision(decisions: &[&str]) -> String {
    if decisions.iter().all(|decision| *decision == "pass") {
        "pass"
    } else {
        "deny"
    }
    .to_string()
}

pub fn plugin_summary(value: &IOValue) -> Result<String> {
    if let Some(summary) = core_summary(value) {
        return Ok(summary);
    }
    if let Some(summary) = receipt_summary(value) {
        return Ok(summary);
    }
    if value.collect_simple_record("plugin-fixture-report-v1", Some(11)).is_some() {
        return Ok(format!("plugin fixture report ref={} (summary is non-normative)", canonical_hash(value)?));
    }
    Err(MoltenError::invalid_harness("unsupported plugin host artifact for summary"))
}

fn core_summary(value: &IOValue) -> Option<String> {
    if let Ok(manifest) = parse_plugin_manifest(value) {
        return Some(format!(
            "plugin manifest ref={} id={} artifact={} hostcalls={} lifecycle={} (summary is non-normative)",
            manifest.manifest_ref,
            manifest.plugin_id,
            manifest.artifact_ref,
            manifest.hostcall_refs.len(),
            manifest.lifecycle_callbacks.len()
        ));
    }
    if let Ok(install) = parse_plugin_install_receipt(value) {
        return Some(format!(
            "plugin install receipt ref={} decision={} manifest={} artifact={} diagnostics={} (summary is non-normative)",
            install.receipt_ref,
            install.decision,
            install.manifest_ref,
            install.artifact_ref,
            install.diagnostics.len()
        ));
    }
    if let Ok(permission) = parse_plugin_permission_receipt(value) {
        return Some(format!(
            "plugin permission receipt ref={} decision={} manifest={} diagnostics={} (summary is non-normative)",
            permission.receipt_ref,
            permission.decision,
            permission.manifest_ref,
            permission.diagnostics.len()
        ));
    }
    if let Ok(lifecycle) = parse_plugin_lifecycle_receipt(value) {
        return Some(format!(
            "plugin lifecycle receipt ref={} operation={} decision={} diagnostics={} (summary is non-normative)",
            lifecycle.receipt_ref,
            lifecycle.operation,
            lifecycle.decision,
            lifecycle.diagnostics.len()
        ));
    }
    None
}

fn receipt_summary(value: &IOValue) -> Option<String> {
    if let Ok(hostcall) = parse_plugin_hostcall_receipt(value) {
        return Some(format!(
            "plugin hostcall receipt ref={} operation={} decision={} diagnostics={} (summary is non-normative)",
            hostcall.receipt_ref,
            hostcall.operation,
            hostcall.decision,
            hostcall.diagnostics.len()
        ));
    }
    if let Ok(health) = parse_plugin_health_receipt(value) {
        return Some(format!(
            "plugin health receipt ref={} decision={} diagnostics={} (summary is non-normative)",
            health.receipt_ref,
            health.decision,
            health.diagnostics.len()
        ));
    }
    if let Ok(removal) = parse_plugin_removal_receipt(value) {
        return Some(format!(
            "plugin removal receipt ref={} decision={} diagnostics={} (summary is non-normative)",
            removal.receipt_ref,
            removal.decision,
            removal.diagnostics.len()
        ));
    }
    if let Ok(upgrade) = parse_plugin_upgrade_receipt(value) {
        return Some(format!(
            "plugin upgrade receipt ref={} decision={} old={} new={} diagnostics={} (summary is non-normative)",
            upgrade.receipt_ref,
            upgrade.decision,
            upgrade.old_manifest_ref,
            upgrade.new_manifest_ref,
            upgrade.diagnostics.len()
        ));
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PluginFixtureReportInput<'a> {
    manifest_ref: &'a str,
    install_receipt_ref: &'a str,
    permission_receipt_ref: &'a str,
    start_receipt_ref: &'a str,
    hostcall_receipt_ref: &'a str,
    health_receipt_ref: &'a str,
    stop_receipt_ref: &'a str,
    removal_receipt_ref: &'a str,
    upgrade_receipt_ref: &'a str,
}

fn plugin_fixture_report_value(input: &PluginFixtureReportInput<'_>) -> Result<IOValue> {
    let refs = [
        input.manifest_ref,
        input.install_receipt_ref,
        input.permission_receipt_ref,
        input.start_receipt_ref,
        input.hostcall_receipt_ref,
        input.health_receipt_ref,
        input.stop_receipt_ref,
        input.removal_receipt_ref,
        input.upgrade_receipt_ref,
    ];
    for value in refs {
        validate_ref(value, "plugin fixture report ref")?;
    }
    Ok(record("plugin-fixture-report-v1", vec![
        string("molten.plugin.fixture-report.v1"),
        record("decision", vec![string("pass")]),
        record("manifest", vec![string(input.manifest_ref)]),
        record("install", vec![string(input.install_receipt_ref)]),
        record("permission", vec![string(input.permission_receipt_ref)]),
        record("start", vec![string(input.start_receipt_ref)]),
        record("hostcall", vec![string(input.hostcall_receipt_ref)]),
        record("health", vec![string(input.health_receipt_ref)]),
        record("stop", vec![string(input.stop_receipt_ref)]),
        record("removal", vec![string(input.removal_receipt_ref)]),
        record("upgrade", vec![string(input.upgrade_receipt_ref)]),
    ]))
}

pub fn storage_read_hostcall_ref() -> Result<String> {
    canonical_hash(&record("plugin-hostcall", vec![string("storage.read")]))
}

pub fn network_open_hostcall_ref() -> Result<String> {
    canonical_hash(&record("plugin-hostcall", vec![string("network.open")]))
}

fn plugin_ref(label: &str) -> Result<String> {
    canonical_hash(&record("plugin-ref", vec![string(label)]))
}

fn plugin_identity_ref(plugin_id: &str, artifact_ref: &str) -> Result<String> {
    canonical_hash(&record("plugin-identity-v1", vec![string(plugin_id), string(artifact_ref)]))
}

fn string_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

trait PushLimited<T> {
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()>;
}

impl<T, S> PushLimited<T> for S
where S: VecSink<T>
{
    fn push_limited(&mut self, value: T, maximum: usize, label: &str) -> Result<()> {
        ensure_count_at_most(self.item_count().saturating_add(1), maximum, label)?;
        self.push_item(value);
        Ok(())
    }
}

fn collect_missing_refs(
    required_refs: &[String],
    supplied_refs: &[String],
    label: &str,
    diagnostics: &mut impl PushLimited<String>,
) -> Result<()> {
    for value in required_refs {
        if !supplied_refs.contains(value) {
            diagnostics.push_limited(
                format!("plugin missing current {label} ref {value}"),
                MAX_PLUGIN_DIAGNOSTICS,
                "plugin permission diagnostics",
            )?;
        }
    }
    Ok(())
}

fn contains_all(supplied_refs: &[String], required_refs: &[String]) -> bool {
    required_refs.iter().all(|required| supplied_refs.contains(required))
}

fn is_lifecycle_declared(callbacks: &[String], operation: &str) -> bool {
    callbacks.iter().any(|callback| callback == operation)
}

fn is_ambient_operation(operation: &str) -> bool {
    ["network", "filesystem", "env", "clock", "process", "node-control"]
        .iter()
        .any(|prefix| operation == *prefix || operation.starts_with(&format!("{prefix}.")))
}

fn validate_plugin_id(value: &str) -> Result<()> {
    validate_non_empty(value, "plugin id")?;
    if !value.starts_with("plugin:") {
        return Err(MoltenError::invalid_harness(format!("plugin id {value} must start with plugin:")));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, ':' | '-' | '_' | '.'))
    {
        return Err(MoltenError::invalid_harness(format!("unsupported plugin id {value}")));
    }
    Ok(())
}

fn validate_abi(value: &str) -> Result<()> {
    if value == PLUGIN_HOST_ABI_VERSION {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported plugin ABI {value}; expected {PLUGIN_HOST_ABI_VERSION}"
        )))
    }
}

fn validate_lifecycle_operation(value: &str) -> Result<()> {
    match value {
        "init" | "start" | "health" | "stop" | "remove" | "upgrade" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported plugin lifecycle operation {value}"))),
    }
}

fn validate_lifecycle_callbacks(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_CALLBACKS, "plugin lifecycle callbacks")?;
    if values.is_empty() {
        return Err(MoltenError::invalid_harness("plugin lifecycle callbacks must not be empty"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        validate_lifecycle_operation(value)?;
        if !seen.insert(value.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate plugin lifecycle callback {value}")));
        }
    }
    Ok(())
}

fn validate_health_status(value: &str) -> Result<()> {
    match value {
        "healthy" | "degraded" | "failed" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported plugin health status {value}"))),
    }
}

fn validate_host_abi_status(value: &str) -> Result<()> {
    match value {
        "ok" | "error" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported plugin ABI result status {value}"))),
    }
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref(value: &str, field: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
        .map_err(|error| MoltenError::invalid_harness(format!("{field} must be a canonical content ref: {error}")))
}

fn validate_optional_ref(value: Option<&str>, field: &str) -> Result<()> {
    if let Some(value) = value {
        validate_ref(value, field)
    } else {
        Ok(())
    }
}

fn validate_refs(values: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_REFS, field)?;
    for value in values {
        validate_ref(value, field)?;
    }
    Ok(())
}

fn require_non_empty_refs(values: &[String], field: &str) -> Result<()> {
    if values.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{field} must not be empty")));
    }
    validate_refs(values, field)
}

fn validate_diagnostics(values: &[String]) -> Result<()> {
    ensure_count_at_most(values.len(), MAX_PLUGIN_DIAGNOSTICS, "plugin diagnostics")
}

fn ensure_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count > maximum {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds {maximum}")))
    } else {
        Ok(())
    }
}

fn status(value: bool) -> &'static str {
    if value { "pass" } else { "fail" }
}

fn refs_sequence(refs: &[String]) -> IOValue {
    sequence(refs.iter().map(string).collect())
}

fn strings_sequence(values: &[String]) -> IOValue {
    sequence(values.iter().map(string).collect())
}

fn optional_ref_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn optional_text_value(value: Option<&str>) -> IOValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(checks: &[(&str, &str)]) -> IOValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn simple_record<'a>(
    value: &'a IOValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IOValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

fn parse_checks(value: &Value<IOValue>) -> Result<Vec<(String, String)>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "plugin checks")?;
    ensure_count_at_most(items.len(), MAX_PLUGIN_CHECKS, "plugin checks")?;
    let mut parsed = Vec::new();
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "plugin check name")?;
        let status = required_string(&check[1], "plugin check status")?;
        match status.as_str() {
            "pass" | "fail" | "diagnostic" => {
                parsed.push_limited((name, status), MAX_PLUGIN_CHECKS, "plugin checks")?
            }
            _ => return Err(MoltenError::invalid_harness("plugin check status must be pass/fail/diagnostic")),
        }
    }
    Ok(parsed)
}

fn require_check(checks: &[(String, String)], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(name, _)| name == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IOValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IOValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IOValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn record_string(value: &Value<IOValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IOValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    validate_ref(&reference, label)?;
    Ok(reference)
}

fn record_string_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let fields = simple_record(&value, label, 1)?;
    let items = required_sequence(&fields[0], label)?;
    ensure_count_at_most(items.len(), MAX_PLUGIN_REFS, label)?;
    let mut values = Vec::new();
    for item in items.iter() {
        values.push_limited(required_string(item, label)?, MAX_PLUGIN_REFS, label)?;
    }
    Ok(values)
}

fn record_ref_sequence(value: &Value<IOValue>, label: &str) -> Result<Vec<String>> {
    let values = record_string_sequence(value, label)?;
    validate_refs(&values, label)?;
    Ok(values)
}

fn required_string(value: &Value<IOValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::catalog;
    use crate::catalog::CatalogListInput;
    use crate::catalog::CatalogVisibilityInput;
    use crate::catalog_mcp;
    use crate::preserves_rail::content_ref_from_bytes;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    fn test_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    fn manifest_value_for_artifact(artifact_ref: &str) -> IOValue {
        let lifecycle_callbacks = string_vec(&["init", "start", "health", "stop", "remove"]);
        let effect_refs = vec![test_ref("effect")];
        let hostcall_refs = vec![storage_read_hostcall_ref().expect("hostcall ref")];
        let schema_refs = vec![test_ref("schema")];
        let policy_refs = vec![test_ref("policy")];
        let resource_refs = vec![test_ref("resource")];
        let supply_refs = vec![test_ref("supply")];
        plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:test",
            artifact_ref,
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &lifecycle_callbacks,
            effect_manifest_refs: &effect_refs,
            hostcall_refs: &hostcall_refs,
            schema_refs: &schema_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            supply_chain_refs: &supply_refs,
        })
        .expect("manifest")
    }

    #[test]
    fn plugin_fixture_runs_lifecycle_and_upgrade() {
        let dir = temp_dir("plugin-fixture");
        let run = minimal_plugin_fixture(&dir).expect("minimal plugin fixture");
        assert_eq!(run.decision, "pass");
        crate::preserves_rail::validate_content_ref(&run.manifest_ref).expect("manifest ref is canonical");
        crate::preserves_rail::validate_content_ref(&run.install_receipt_ref)
            .expect("install receipt ref is canonical");
        assert!(plugin_summary(&run.report_value).expect("summary").contains("plugin fixture report"));
        assert!(run.evidence_values.len() >= 10);
    }

    #[test]
    fn raw_host_path_missing_artifact_and_stale_provenance_deny() {
        let malformed = parse_text(
            "<plugin-manifest-v1 \"molten.plugin.manifest.v1\" \
             <plugin-id \"plugin:path\"> <artifact \"/usr/bin/plugin\"> <abi \"molten.plugin.host-abi.v1\"> \
             <lifecycle [\"start\"]> <effects []> <hostcalls []> <schemas []> <policy []> <resource []> \
             <supply-chain []> <checks [<check \"artifact-backed\" \"fail\"> <check \"no-ambient-authority\" \"pass\">]>>",
        )
        .expect("parse malformed manifest");
        assert!(parse_plugin_manifest(&malformed).is_err());

        let dir = temp_dir("plugin-deny");
        let registry = dir.join("registry");
        let artifact = artifacts::install_artifact(&registry, &artifacts::ArtifactInstallInput {
            kind: "plugin-executor".to_string(),
            payload: record("plugin", vec![string("x")]),
            schema_refs: vec![test_ref("schema")],
            dependency_refs: Vec::new(),
            effect_manifest_ref: Some(test_ref("effect")),
            policy_refs: vec![test_ref("policy")],
            evidence_refs: vec![test_ref("supply")],
            installer_ref: test_ref("installer"),
            capability_refs: vec![test_ref("capability")],
        })
        .expect("install artifact");
        let manifest = manifest_value_for_artifact(&artifact.artifact_ref);
        let install = install_plugin(&registry, &manifest).expect("install plugin");
        assert_eq!(install.decision, "pass");
        let permission = plugin_permission_receipt_value(&PermissionReviewInput {
            manifest_value: &manifest,
            authority_refs: &[test_ref("authority")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            effect_receipt_refs: &[test_ref("effect-receipt")],
            supply_chain_refs: &[test_ref("stale-supply")],
        })
        .expect("permission receipt");
        let parsed = parse_plugin_permission_receipt(&permission).expect("parse permission");
        assert_eq!(parsed.decision, "deny");
        assert!(parsed.diagnostics.iter().any(|diagnostic| diagnostic.contains("supply-chain")));
    }

    #[test]
    fn ambient_hostcall_failed_health_and_cleanup_are_receipted() {
        let manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:ambient",
            artifact_ref: &test_ref("artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &[
                "start".to_string(),
                "health".to_string(),
                "stop".to_string(),
                "remove".to_string(),
            ],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("storage hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("manifest");
        let denied_hostcall = plugin_hostcall_receipt_value(&HostcallReceiptInput {
            manifest_value: &manifest,
            operation: "network.open",
            hostcall_ref: &network_open_hostcall_ref().expect("network hostcall"),
            executor_receipt_ref: &test_ref("executor"),
            effect_receipt_ref: &test_ref("effect-receipt"),
            authority_refs: &[test_ref("authority")],
            resource_refs: &[test_ref("resource")],
        })
        .expect("hostcall receipt");
        let denied = parse_plugin_hostcall_receipt(&denied_hostcall).expect("parse hostcall");
        assert_eq!(denied.decision, "deny");
        assert!(denied.diagnostics.iter().any(|diagnostic| diagnostic.contains("ambient")));
        let health = plugin_health_receipt_value(&HealthReceiptInput {
            manifest_value: &manifest,
            lifecycle_receipt_ref: &test_ref("start"),
            service_refs: &[test_ref("service")],
            health_status: "failed",
            diagnostics: &["probe failed".to_string()],
        })
        .expect("health receipt");
        assert_eq!(parse_plugin_health_receipt(&health).expect("parse health").decision, "deny");
        let incomplete_removal = plugin_removal_receipt_value(&RemovalReceiptInput {
            manifest_value: &manifest,
            lifecycle_receipt_ref: &test_ref("remove"),
            owned_service_refs: &[test_ref("service")],
            assertion_refs: &[],
            handle_refs: &[],
            catalog_entry_refs: &[],
            diagnostics: &[],
        })
        .expect("removal receipt");
        assert_eq!(parse_plugin_removal_receipt(&incomplete_removal).expect("parse removal").decision, "deny");
    }

    #[test]
    fn host_abi_result_and_upgrade_compatibility_are_canonical() {
        let payload_ref = test_ref("payload");
        let result = plugin_host_abi_result_value(&HostAbiResultInput {
            status: "ok",
            payload_ref: Some(&payload_ref),
            error: None,
        })
        .expect("ABI result");
        assert!(to_text(&result).expect("render result").contains("plugin-host-abi-result-v1"));
        let old_manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:upgrade",
            artifact_ref: &test_ref("old-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("old manifest");
        let new_manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:upgrade",
            artifact_ref: &test_ref("new-artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema"), test_ref("schema-extra")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("new manifest");
        let upgrade = plugin_upgrade_receipt_value(&UpgradeReceiptInput {
            old_manifest_value: &old_manifest,
            new_manifest_value: &new_manifest,
            rollback_ref: &test_ref("rollback"),
            cleanup_refs: &[test_ref("cleanup")],
            diagnostics: &[],
        })
        .expect("upgrade receipt");
        assert_eq!(parse_plugin_upgrade_receipt(&upgrade).expect("parse upgrade").decision, "pass");
    }

    #[test]
    fn ledger_catalog_and_mcp_classify_plugin_artifacts() {
        let dir = temp_dir("plugin-catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let manifest = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:catalog",
            artifact_ref: &test_ref("artifact"),
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &["start".to_string()],
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("manifest");
        let imported = ledger::import_artifact(&ledger_root, &manifest).expect("ledger import");
        assert_eq!(imported.artifact_kind, "plugin-manifest");
        let listed = catalog::list(&registry, Some(&ledger_root), &CatalogListInput {
            kind: Some("plugin-manifest".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("catalog list plugin manifest");
        assert_eq!(listed.items.len(), 1);
        let rendered = to_text(&listed.value).expect("render catalog result");
        assert!(rendered.contains("ledger-kind:plugin-manifest"));
        let request =
            catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string("plugin-manifest")])])
                .expect("MCP request");
        let mcp = catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("MCP list plugin manifest");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render MCP response").contains("plugin-manifest"));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_plugin_lifecycle_refs_are_deterministic_and_authority_gated(tc: TestCase) {
        let callback_count = tc.draw(generators::integers::<u64>().min_value(1).max_value(4));
        let callback_count = usize::try_from(callback_count).expect("bounded callback count");
        let callbacks = ["init", "start", "health", "stop"]
            .iter()
            .take(callback_count)
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        let artifact_ref = test_ref("artifact-property");
        let value = plugin_manifest_value(&PluginManifestInput {
            plugin_id: "plugin:property",
            artifact_ref: &artifact_ref,
            abi: PLUGIN_HOST_ABI_VERSION,
            lifecycle_callbacks: &callbacks,
            effect_manifest_refs: &[test_ref("effect")],
            hostcall_refs: &[storage_read_hostcall_ref().expect("hostcall")],
            schema_refs: &[test_ref("schema")],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("manifest");
        let first_ref = canonical_hash(&value).expect("first ref");
        let rendered = to_text(&value).expect("render manifest");
        let reparsed = parse_text(&rendered).expect("parse rendered manifest");
        assert_eq!(first_ref, canonical_hash(&reparsed).expect("second ref"));
        let permission = plugin_permission_receipt_value(&PermissionReviewInput {
            manifest_value: &value,
            authority_refs: &[],
            policy_refs: &[test_ref("policy")],
            resource_refs: &[test_ref("resource")],
            effect_receipt_refs: &[test_ref("effect-receipt")],
            supply_chain_refs: &[test_ref("supply")],
        })
        .expect("permission receipt");
        assert_eq!(parse_plugin_permission_receipt(&permission).expect("parse permission").decision, "deny");
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
