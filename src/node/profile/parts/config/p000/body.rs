type IoValue = preserves::IOValue;
type Result<T> = crate::error::Result<T>;
type MoltenError = crate::error::MoltenError;

type NodeAdapterBinding = crate::node_runtime::NodeAdapterBinding;

const PROFILE_RESOLUTION_SCHEMA: &str = "molten.node.profile-config-resolution.v1";
const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const TIER_DEVELOPMENT: &str = "development";
const TIER_PILOT: &str = "pilot";
const TIER_RELEASE: &str = "release";
const SOURCE_KIND_CHECKED_EXPORT: &str = "checked-export";
const SOURCE_KIND_PROFILE_REF: &str = "profile-ref";
const SOURCE_KIND_NICKEL_SOURCE: &str = "nickel-source";
const LOCAL_FIXTURE_CAVEAT: &str = "local-fixture-config";
const EVIDENCE_ONLY_CAVEAT: &str = "profile-backed node config is startup evidence only and does not grant authority, source-gate acceptance, adapter readiness, resource sufficiency, retention clearance, transport correctness, deployment trust, or release eligibility";
const OVERRIDE_STATE_ROOT_REF: &str = "state-root-ref";
const OVERRIDE_POLICY_REFS: &str = "policy-refs";
const OVERRIDE_ADAPTER_REFS: &str = "adapter-profile-refs";
const MAX_DIAGNOSTICS: usize = 256;
const MAX_REFS: usize = 128;
const MAX_ADAPTERS: usize = 16;
const MAX_OVERRIDES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedNodeProfile {
    pub profile_ref: String,
    pub actual_profile_ref: Option<String>,
    pub source_kind: String,
    pub tier: String,
    pub schema_id: String,
    pub schema_version: String,
    pub source_language: String,
    pub profile_identity: String,
    pub state_root_ref: String,
    pub adapters: Vec<NodeAdapterBinding>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub overrideable_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NodeProfileOverrides {
    pub state_root_ref: Option<String>,
    pub adapters: Option<Vec<NodeAdapterBinding>>,
    pub policy_refs: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileBackedConfigInput {
    pub identity_ref: String,
    pub profile: CheckedNodeProfile,
    pub overrides: NodeProfileOverrides,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDefaultConfigInput {
    pub identity_ref: String,
    pub state_root_ref: String,
    pub adapters: Vec<NodeAdapterBinding>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedNodeConfig {
    pub decision: String,
    pub diagnostics: Vec<String>,
    pub accepted_overrides: Vec<String>,
    pub profile_metadata_refs: Vec<String>,
    pub config_ref: String,
    pub config_value: IoValue,
    pub resolution_ref: String,
    pub resolution_value: IoValue,
}

// r[impl molten.node_runtime.profile_backed_config]
// r[impl molten.node_runtime.profile_override_policy]
// r[impl molten.node_runtime.profile_startup_receipt_binding]
pub fn resolve_profile_backed_config(input: &ProfileBackedConfigInput) -> Result<ResolvedNodeConfig> {
    validate_ref(&input.identity_ref, "node profile identity ref")?;
    validate_checked_profile(&input.profile)?;
    validate_overrides(&input.overrides)?;
    let mut diagnostics = Vec::new();
    collect_source_kind_diagnostics(&input.profile, &mut diagnostics);
    collect_profile_ref_diagnostics(&input.profile, &mut diagnostics);
    collect_adapter_diagnostics(&input.profile.adapters, &mut diagnostics);
    let accepted_overrides = collect_override_diagnostics(&input.profile, &input.overrides, &mut diagnostics);
    let effective = effective_profile(&input.profile, &input.overrides);
    let config_value = crate::node_runtime::node_config_value(&crate::node_runtime::ConfigValueInput {
        identity_ref: &input.identity_ref,
        state_root_ref: &effective.state_root_ref,
        adapters: &effective.adapters,
        policy_refs: &effective.policy_refs,
        capability_refs: &effective.capability_refs,
        resource_refs: &effective.resource_refs,
        effect_profile_refs: &effective.effect_profile_refs,
    })?;
    finish_resolution(FinishResolutionInput {
        identity_ref: &input.identity_ref,
        profile_ref: &input.profile.profile_ref,
        tier: &input.profile.tier,
        schema_id: &input.profile.schema_id,
        schema_version: &input.profile.schema_version,
        source_language: &input.profile.source_language,
        profile_identity: &input.profile.profile_identity,
        accepted_overrides,
        diagnostics,
        config_value,
        caveats: Vec::new(),
    })
}

// r[impl molten.node_runtime.local_default_config_caveat]
pub fn resolve_local_default_config(input: &LocalDefaultConfigInput) -> Result<ResolvedNodeConfig> {
    validate_ref(&input.identity_ref, "local node identity ref")?;
    validate_ref(&input.state_root_ref, "local node state root ref")?;
    validate_adapter_diagnostics_or_error(&input.adapters)?;
    validate_refs(&input.policy_refs, "local node policy ref")?;
    validate_refs(&input.capability_refs, "local node capability ref")?;
    validate_refs(&input.resource_refs, "local node resource ref")?;
    validate_refs(&input.effect_profile_refs, "local node effect profile ref")?;
    let config_value = crate::node_runtime::node_config_value(&crate::node_runtime::ConfigValueInput {
        identity_ref: &input.identity_ref,
        state_root_ref: &input.state_root_ref,
        adapters: &input.adapters,
        policy_refs: &input.policy_refs,
        capability_refs: &input.capability_refs,
        resource_refs: &input.resource_refs,
        effect_profile_refs: &input.effect_profile_refs,
    })?;
    let local_fixture_profile_ref = crate::preserves_rail::content_ref_from_bytes(LOCAL_FIXTURE_CAVEAT.as_bytes());
    finish_resolution(FinishResolutionInput {
        identity_ref: &input.identity_ref,
        profile_ref: &local_fixture_profile_ref,
        tier: TIER_DEVELOPMENT,
        schema_id: crate::preserves_rail::NODE_CONFIG_SCHEMA,
        schema_version: "1",
        source_language: "rust-local-defaults",
        profile_identity: LOCAL_FIXTURE_CAVEAT,
        accepted_overrides: Vec::new(),
        diagnostics: vec![LOCAL_FIXTURE_CAVEAT.to_string()],
        config_value,
        caveats: vec![LOCAL_FIXTURE_CAVEAT.to_string()],
    })
}

fn validate_checked_profile(profile: &CheckedNodeProfile) -> Result<()> {
    validate_ref(&profile.profile_ref, "checked node profile ref")?;
    if let Some(actual) = profile.actual_profile_ref.as_ref() {
        validate_ref(actual, "actual checked node profile ref")?;
    }
    validate_tier(&profile.tier)?;
    validate_source_kind(&profile.source_kind)?;
    validate_text("node profile schema id", &profile.schema_id)?;
    validate_text("node profile schema version", &profile.schema_version)?;
    validate_text("node profile source language", &profile.source_language)?;
    validate_text("node profile identity", &profile.profile_identity)?;
    validate_ref(&profile.state_root_ref, "checked node profile state root ref")?;
    validate_adapter_diagnostics_or_error(&profile.adapters)?;
    validate_refs(&profile.policy_refs, "checked node profile policy ref")?;
    validate_refs(&profile.capability_refs, "checked node profile capability ref")?;
    validate_refs(&profile.resource_refs, "checked node profile resource ref")?;
    validate_refs(&profile.effect_profile_refs, "checked node profile effect profile ref")?;
    crate::bounded::ensure_count_at_most(
        profile.overrideable_fields.len(),
        MAX_OVERRIDES,
        "node profile overrideable fields",
    )?;
    for field in &profile.overrideable_fields {
        validate_override_field(field)?;
    }
    Ok(())
}

fn validate_overrides(overrides: &NodeProfileOverrides) -> Result<()> {
    if let Some(state_root_ref) = overrides.state_root_ref.as_ref() {
        validate_ref(state_root_ref, "node profile override state root ref")?;
    }
    if let Some(adapters) = overrides.adapters.as_ref() {
        validate_adapter_diagnostics_or_error(adapters)?;
    }
    if let Some(policy_refs) = overrides.policy_refs.as_ref() {
        validate_refs(policy_refs, "node profile override policy ref")?;
    }
    Ok(())
}

fn validate_tier(tier: &str) -> Result<()> {
    match tier {
        TIER_DEVELOPMENT | TIER_PILOT | TIER_RELEASE => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported node profile tier {other}"))),
    }
}

fn validate_source_kind(kind: &str) -> Result<()> {
    match kind {
        SOURCE_KIND_CHECKED_EXPORT | SOURCE_KIND_PROFILE_REF | SOURCE_KIND_NICKEL_SOURCE => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported node profile source kind {other}"))),
    }
}

fn collect_source_kind_diagnostics(
    profile: &CheckedNodeProfile,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if profile.source_kind == SOURCE_KIND_NICKEL_SOURCE {
        diagnostics.push_item("runtime-nickel-evaluation-denied:startup-consumes-checked-exports".to_string());
    }
}

fn collect_profile_ref_diagnostics(
    profile: &CheckedNodeProfile,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    if let Some(actual) = profile.actual_profile_ref.as_ref()
        && actual != &profile.profile_ref
    {
        diagnostics.push_item(format!("profile-ref-mismatch:expected={}:actual={actual}", profile.profile_ref));
    }
}

fn collect_adapter_diagnostics(
    adapters: &[NodeAdapterBinding],
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) {
    for adapter in adapters {
        if !is_required_runtime_adapter(&adapter.name) {
            diagnostics.push_item(format!("unsupported-node-adapter-profile:{}", adapter.name));
        }
    }
    for required in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        if !adapters.iter().any(|adapter| adapter.name == *required) {
            diagnostics.push_item(format!("missing-required-node-adapter:{required}"));
        }
    }
}

fn validate_adapter_diagnostics_or_error(adapters: &[NodeAdapterBinding]) -> Result<()> {
    crate::bounded::ensure_count_at_most(adapters.len(), MAX_ADAPTERS, "node profile adapters")?;
    for adapter in adapters {
        crate::node_runtime::node_adapter_binding(&adapter.name, &adapter.profile_ref)?;
    }
    Ok(())
}

fn collect_override_diagnostics(
    profile: &CheckedNodeProfile,
    overrides: &NodeProfileOverrides,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Vec<String> {
    let mut accepted = Vec::new();
    if let Some(state_root_ref) = overrides.state_root_ref.as_ref() {
        collect_override_diagnostic(profile, OVERRIDE_STATE_ROOT_REF, state_root_ref, diagnostics, &mut accepted);
    }
    if let Some(adapters) = overrides.adapters.as_ref() {
        let value = adapters
            .iter()
            .map(|adapter| format!("{}={}", adapter.name, adapter.profile_ref))
            .collect::<Vec<_>>()
            .join(",");
        collect_override_diagnostic(profile, OVERRIDE_ADAPTER_REFS, &value, diagnostics, &mut accepted);
        if profile.tier == TIER_RELEASE {
            collect_adapter_diagnostics(adapters, diagnostics);
        }
    }
    if let Some(policy_refs) = overrides.policy_refs.as_ref() {
        let value = policy_refs.join(",");
        collect_override_diagnostic(profile, OVERRIDE_POLICY_REFS, &value, diagnostics, &mut accepted);
        if profile.tier == TIER_RELEASE && policy_refs.is_empty() {
            diagnostics.push_item("denied-release-invariant-override:policy-refs-empty".to_string());
        }
    }
    accepted
}
