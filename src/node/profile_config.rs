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

fn collect_source_kind_diagnostics(profile: &CheckedNodeProfile, diagnostics: &mut Vec<String>) {
    if profile.source_kind == SOURCE_KIND_NICKEL_SOURCE {
        diagnostics.push("runtime-nickel-evaluation-denied:startup-consumes-checked-exports".to_string());
    }
}

fn collect_profile_ref_diagnostics(profile: &CheckedNodeProfile, diagnostics: &mut Vec<String>) {
    if let Some(actual) = profile.actual_profile_ref.as_ref()
        && actual != &profile.profile_ref
    {
        diagnostics.push(format!("profile-ref-mismatch:expected={}:actual={actual}", profile.profile_ref));
    }
}

fn collect_adapter_diagnostics(adapters: &[NodeAdapterBinding], diagnostics: &mut Vec<String>) {
    for adapter in adapters {
        if !is_required_runtime_adapter(&adapter.name) {
            diagnostics.push(format!("unsupported-node-adapter-profile:{}", adapter.name));
        }
    }
    for required in crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS {
        if !adapters.iter().any(|adapter| adapter.name == *required) {
            diagnostics.push(format!("missing-required-node-adapter:{required}"));
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
    diagnostics: &mut Vec<String>,
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
            diagnostics.push("denied-release-invariant-override:policy-refs-empty".to_string());
        }
    }
    accepted
}

fn collect_override_diagnostic(
    profile: &CheckedNodeProfile,
    field: &str,
    value: &str,
    diagnostics: &mut Vec<String>,
    accepted: &mut Vec<String>,
) {
    if profile.overrideable_fields.iter().any(|allowed| allowed == field) && profile.tier != TIER_RELEASE {
        accepted.push(format!("accepted-override:{field}={value}"));
    } else {
        diagnostics.push(format!("denied-profile-override:{field}"));
    }
}

fn effective_profile(profile: &CheckedNodeProfile, overrides: &NodeProfileOverrides) -> CheckedNodeProfile {
    let mut effective = profile.clone();
    if profile.tier != TIER_RELEASE {
        if let Some(state_root_ref) = overrides.state_root_ref.as_ref()
            && profile.overrideable_fields.iter().any(|field| field == OVERRIDE_STATE_ROOT_REF)
        {
            effective.state_root_ref.clone_from(state_root_ref);
        }
        if let Some(adapters) = overrides.adapters.as_ref()
            && profile.overrideable_fields.iter().any(|field| field == OVERRIDE_ADAPTER_REFS)
        {
            effective.adapters.clone_from(adapters);
        }
        if let Some(policy_refs) = overrides.policy_refs.as_ref()
            && profile.overrideable_fields.iter().any(|field| field == OVERRIDE_POLICY_REFS)
        {
            effective.policy_refs.clone_from(policy_refs);
        }
    }
    effective
}

struct FinishResolutionInput<'a> {
    identity_ref: &'a str,
    profile_ref: &'a str,
    tier: &'a str,
    schema_id: &'a str,
    schema_version: &'a str,
    source_language: &'a str,
    profile_identity: &'a str,
    accepted_overrides: Vec<String>,
    diagnostics: Vec<String>,
    config_value: IoValue,
    caveats: Vec<String>,
}

fn finish_resolution(input: FinishResolutionInput<'_>) -> Result<ResolvedNodeConfig> {
    let FinishResolutionInput {
        identity_ref,
        profile_ref,
        tier,
        schema_id,
        schema_version,
        source_language,
        profile_identity,
        accepted_overrides,
        mut diagnostics,
        config_value,
        caveats,
    } = input;
    diagnostics.sort();
    diagnostics.dedup();
    ensure_diagnostic_bound(diagnostics.len())?;
    let config_ref = crate::preserves_rail::canonical_hash(&config_value)?;
    let decision = if diagnostics.iter().any(|diagnostic| {
        diagnostic.starts_with("denied")
            || diagnostic.contains("mismatch")
            || diagnostic.contains("unsupported")
            || diagnostic.contains("missing-required")
            || diagnostic.contains("runtime-nickel")
    }) {
        DECISION_DENY
    } else {
        DECISION_PASS
    };
    let resolution_value = resolution_value(ResolutionValueInput {
        decision,
        identity_ref,
        profile_ref,
        config_ref: &config_ref,
        tier,
        schema_id,
        schema_version,
        source_language,
        profile_identity,
        accepted_overrides: &accepted_overrides,
        diagnostics: &diagnostics,
        caveats: &caveats,
    })?;
    let resolution_ref = crate::preserves_rail::canonical_hash(&resolution_value)?;
    let profile_metadata_refs = vec![profile_ref.to_string(), resolution_ref.clone()];
    Ok(ResolvedNodeConfig {
        decision: decision.to_string(),
        diagnostics,
        accepted_overrides,
        profile_metadata_refs,
        config_ref,
        config_value,
        resolution_ref,
        resolution_value,
    })
}

struct ResolutionValueInput<'a> {
    decision: &'a str,
    identity_ref: &'a str,
    profile_ref: &'a str,
    config_ref: &'a str,
    tier: &'a str,
    schema_id: &'a str,
    schema_version: &'a str,
    source_language: &'a str,
    profile_identity: &'a str,
    accepted_overrides: &'a [String],
    diagnostics: &'a [String],
    caveats: &'a [String],
}

fn resolution_value(input: ResolutionValueInput<'_>) -> Result<IoValue> {
    let mut caveats = input.caveats.to_vec();
    caveats.push(EVIDENCE_ONLY_CAVEAT.to_string());
    Ok(record("node-profile-config-resolution-v1", vec![
        string(PROFILE_RESOLUTION_SCHEMA),
        field_string("decision", input.decision),
        field_string("identity", input.identity_ref),
        field_string("profile", input.profile_ref),
        field_string("node-config", input.config_ref),
        record("metadata", vec![
            field_string("tier", input.tier),
            field_string("schema-id", input.schema_id),
            field_string("schema-version", input.schema_version),
            field_string("source-language", input.source_language),
            field_string("profile-identity", input.profile_identity),
        ]),
        field_sequence("accepted-overrides", string_values(input.accepted_overrides)?),
        field_sequence("diagnostics", string_values(input.diagnostics)?),
        field_sequence("caveats", string_values(&caveats)?),
    ]))
}

fn validate_override_field(field: &str) -> Result<()> {
    match field {
        OVERRIDE_STATE_ROOT_REF | OVERRIDE_POLICY_REFS | OVERRIDE_ADAPTER_REFS => Ok(()),
        other => Err(MoltenError::invalid_harness(format!("unsupported node profile override field {other}"))),
    }
}

fn is_required_runtime_adapter(name: &str) -> bool {
    crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS.iter().any(|required| required == &name)
}

fn validate_refs(refs: &[String], label: &str) -> Result<()> {
    crate::bounded::ensure_count_at_most(refs.len(), MAX_REFS, label)?;
    for reference in refs {
        validate_ref(reference, label)?;
    }
    Ok(())
}

fn validate_ref(reference: &str, label: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(reference)
        .map_err(|error| MoltenError::invalid_harness(format!("invalid {label} {reference}: {error}")))
}

fn validate_text(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{label} must not be empty")))
    } else {
        Ok(())
    }
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn field_string(label: &'static str, value: &str) -> IoValue {
    record(label, vec![string(value)])
}

fn field_sequence(label: &'static str, values: Vec<IoValue>) -> IoValue {
    record(label, vec![crate::preserves_rail::sequence(values)])
}

fn string(value: &str) -> IoValue {
    crate::preserves_rail::string(value)
}

fn string_values(values: &[String]) -> Result<Vec<IoValue>> {
    ensure_diagnostic_bound(values.len())?;
    Ok(values.iter().map(|value| string(value)).collect())
}

fn ensure_diagnostic_bound(count: usize) -> Result<()> {
    crate::bounded::ensure_count_at_most(count, MAX_DIAGNOSTICS, "node profile diagnostics")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_ref(label: &str) -> String {
        crate::preserves_rail::content_ref_from_bytes(label.as_bytes())
    }

    fn required_adapters() -> Vec<NodeAdapterBinding> {
        crate::node_runtime::REQUIRED_RUNTIME_ADAPTERS
            .iter()
            .map(|adapter| {
                crate::node_runtime::node_adapter_binding(adapter, &local_ref(&format!("adapter-{adapter}"))).unwrap()
            })
            .collect()
    }

    fn refs(label: &str) -> Vec<String> {
        vec![local_ref(label)]
    }

    fn profile() -> CheckedNodeProfile {
        CheckedNodeProfile {
            profile_ref: local_ref("node-profile"),
            actual_profile_ref: Some(local_ref("node-profile")),
            source_kind: SOURCE_KIND_CHECKED_EXPORT.to_string(),
            tier: TIER_PILOT.to_string(),
            schema_id: "molten.prod-ops.deployment-profile.v1".to_string(),
            schema_version: "1".to_string(),
            source_language: "nickel".to_string(),
            profile_identity: "pilot-node".to_string(),
            state_root_ref: local_ref("state-root"),
            adapters: required_adapters(),
            policy_refs: refs("policy"),
            capability_refs: refs("capability"),
            resource_refs: refs("resource"),
            effect_profile_refs: refs("effects"),
            overrideable_fields: vec![OVERRIDE_STATE_ROOT_REF.to_string()],
        }
    }

    fn input() -> ProfileBackedConfigInput {
        ProfileBackedConfigInput {
            identity_ref: local_ref("identity"),
            profile: profile(),
            overrides: NodeProfileOverrides::default(),
        }
    }

    // r[verify molten.node_runtime.profile_backed_config]
    // r[verify molten.node_runtime.profile_startup_receipt_binding]
    #[test]
    fn profile_backed_config_builds_canonical_node_config_and_metadata_refs() {
        let resolved = resolve_profile_backed_config(&input()).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_PASS);
        assert_eq!(resolved.diagnostics, Vec::<String>::new());
        assert_eq!(resolved.profile_metadata_refs.len(), 2);
        let config = crate::node_runtime::parse_node_config(&resolved.config_value).expect("config parse");
        assert_eq!(config.config_ref, resolved.config_ref);
        assert_eq!(config.policy_refs, refs("policy"));
        assert!(
            crate::preserves_rail::to_text(&resolved.resolution_value)
                .expect("resolution text")
                .contains("node-profile-config-resolution-v1")
        );
    }

    // r[verify molten.node_runtime.profile_override_policy]
    #[test]
    fn development_profile_records_allowed_override() {
        let mut with_override = input();
        with_override.profile.tier = TIER_DEVELOPMENT.to_string();
        let override_ref = local_ref("override-state-root");
        with_override.overrides.state_root_ref = Some(override_ref.clone());
        let resolved = resolve_profile_backed_config(&with_override).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_PASS);
        assert!(
            resolved
                .accepted_overrides
                .iter()
                .any(|item| item == &format!("accepted-override:{OVERRIDE_STATE_ROOT_REF}={override_ref}"))
        );
        let config = crate::node_runtime::parse_node_config(&resolved.config_value).expect("config parse");
        assert_eq!(config.state_root_ref, override_ref);
    }

    #[test]
    fn release_profile_denies_invariant_weakening_override() {
        let mut with_override = input();
        with_override.profile.tier = TIER_RELEASE.to_string();
        with_override.overrides.state_root_ref = Some(local_ref("override-state-root"));
        let resolved = resolve_profile_backed_config(&with_override).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_DENY);
        assert!(
            resolved
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == &format!("denied-profile-override:{OVERRIDE_STATE_ROOT_REF}"))
        );
    }

    #[test]
    fn profile_resolution_denies_tampered_ref_runtime_nickel_and_unsupported_adapter() {
        let mut bad = input();
        bad.profile.actual_profile_ref = Some(local_ref("tampered-node-profile"));
        bad.profile.source_kind = SOURCE_KIND_NICKEL_SOURCE.to_string();
        bad.profile.adapters.push(
            crate::node_runtime::node_adapter_binding("unsupported-adapter", &local_ref("unsupported-adapter"))
                .expect("unsupported shape ok"),
        );
        let resolved = resolve_profile_backed_config(&bad).expect("profile resolution");
        assert_eq!(resolved.decision, DECISION_DENY);
        assert!(resolved.diagnostics.iter().any(|diagnostic| diagnostic.starts_with("profile-ref-mismatch")));
        assert!(
            resolved
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "runtime-nickel-evaluation-denied:startup-consumes-checked-exports")
        );
        assert!(
            resolved
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "unsupported-node-adapter-profile:unsupported-adapter")
        );
    }

    // r[verify molten.node_runtime.local_default_config_caveat]
    #[test]
    fn local_default_config_is_fixture_scoped_and_not_release_evidence() {
        let local = resolve_local_default_config(&LocalDefaultConfigInput {
            identity_ref: local_ref("identity"),
            state_root_ref: local_ref("state-root"),
            adapters: required_adapters(),
            policy_refs: refs("policy"),
            capability_refs: refs("capability"),
            resource_refs: refs("resource"),
            effect_profile_refs: refs("effects"),
        })
        .expect("local resolution");
        assert_eq!(local.decision, DECISION_PASS);
        assert!(local.diagnostics.iter().any(|diagnostic| diagnostic == LOCAL_FIXTURE_CAVEAT));
        assert!(
            crate::preserves_rail::to_text(&local.resolution_value)
                .expect("local resolution text")
                .contains(LOCAL_FIXTURE_CAVEAT)
        );
    }
}
