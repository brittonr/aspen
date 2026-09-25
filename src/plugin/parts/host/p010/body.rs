
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
    pub prior_state: PluginLifecycleState,
    pub event: PluginLifecycleEvent,
    pub next_state: PluginLifecycleState,
    pub guard_refs: Vec<String>,
    pub side_effect_class: String,
    pub value: IoValue,
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
