//! Pure component execution admission over parsed artifact facts and typed import grants.

mod features;
mod identity;

use alloc::string::ToString;

pub const COMPONENT_INVOKE_EXPORT: &str = "invoke";
pub const WASI_IMPORT_PREFIX: &str = "wasi:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrowthFacts {
    pub initial: u64,
    pub maximum: Option<u64>,
    pub strategy: super::model::GrowthStrategy,
}

/// Facts the shell parsed from component bytes; the core never reads bytes itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFacts {
    pub artifact_kind: super::model::ArtifactKind,
    pub declared_profile_id: alloc::string::String,
    pub declared_cohort_ref: alloc::string::String,
    pub declared_world: alloc::string::String,
    pub imports: alloc::vec::Vec<alloc::string::String>,
    pub exports: alloc::vec::Vec<alloc::string::String>,
    pub enabled_features: alloc::vec::Vec<alloc::string::String>,
    pub memory: GrowthFacts,
    pub table: GrowthFacts,
    pub instances: u64,
    pub memories: u64,
    pub tables: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportGrant {
    pub import: alloc::string::String,
    pub capability: alloc::string::String,
    pub policy_ref: alloc::string::String,
    pub authority_ref: alloc::string::String,
    pub resource_ref: alloc::string::String,
    pub recorded_effect_ref: alloc::string::String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPlan {
    pub component_ref: alloc::string::String,
    pub wit_ref: alloc::string::String,
    pub bundle_ref: Option<alloc::string::String>,
    pub profile_ref: alloc::string::String,
    pub runtime_configuration_ref: alloc::string::String,
    pub imports: alloc::vec::Vec<alloc::string::String>,
    pub capabilities: alloc::vec::Vec<alloc::string::String>,
    pub mantle_evidence_refs: alloc::vec::Vec<alloc::string::String>,
    pub valence_evidence_refs: alloc::vec::Vec<alloc::string::String>,
    pub cairn_evidence_refs: alloc::vec::Vec<alloc::string::String>,
    pub policy_refs: alloc::vec::Vec<alloc::string::String>,
    pub authority_refs: alloc::vec::Vec<alloc::string::String>,
    pub resource_refs: alloc::vec::Vec<alloc::string::String>,
    pub recorded_effect_refs: alloc::vec::Vec<alloc::string::String>,
    pub materialization: super::materialization::Admission,
}

/// Plans one execution: profile, declared import surface, identity, features, resources, then
/// exactly-one authority grant per import. Every denial is classified and deterministic.
///
/// `admit_declared_surface` is the declared import-manifest admission port. The shell binds the
/// manifest identity over canonical Preserves; the port runs after profile validation and before
/// any fact is examined, exactly as the in-process planner always ordered it.
pub fn plan_execution<F>(
    profile: &super::model::RuntimeProfile,
    materialization: super::materialization::Admission,
    facts: &ArtifactFacts,
    grants: &[ImportGrant],
    admit_declared_surface: F,
) -> super::model::Result<ExecutionPlan>
where
    F: FnOnce(
        &super::model::RuntimeProfile,
        &super::materialization::Admission,
        &ArtifactFacts,
    ) -> super::model::Result<()>,
{
    super::profile::validate_runtime(profile)?;
    admit_declared_surface(profile, &materialization, facts)?;
    let mut blockers = alloc::vec::Vec::new();
    validate_identity(profile, &materialization, facts, &mut blockers);
    features::validate(profile, facts, &mut blockers);
    validate_resources(profile, facts, &mut blockers);
    if !blockers.is_empty() {
        return Err(super::model::Denial::from_blockers(blockers));
    }
    let grant_plan = validate_imports(profile, facts, grants, &mut blockers);
    if !blockers.is_empty() {
        return Err(super::model::Denial::from_blockers(blockers));
    }
    let runtime_configuration_ref = identity::runtime_configuration_ref(profile, facts, &materialization, &grant_plan);
    Ok(ExecutionPlan {
        component_ref: materialization.component_ref.clone(),
        wit_ref: materialization.wit_ref.clone(),
        bundle_ref: materialization.bundle_ref.clone(),
        profile_ref: super::profile::r#ref(profile),
        runtime_configuration_ref,
        imports: super::model::sorted_unique(&facts.imports),
        capabilities: grant_plan.capabilities,
        mantle_evidence_refs: materialization.mantle_evidence_refs.clone(),
        valence_evidence_refs: materialization.valence_evidence_refs.clone(),
        cairn_evidence_refs: materialization.cairn_evidence_refs.clone(),
        policy_refs: merge_refs(&materialization.policy_refs, &grant_plan.policy_refs),
        authority_refs: merge_refs(&materialization.authority_refs, &grant_plan.authority_refs),
        resource_refs: merge_refs(&materialization.resource_refs, &grant_plan.resource_refs),
        recorded_effect_refs: grant_plan.recorded_effect_refs,
        materialization,
    })
}

#[derive(Debug)]
struct GrantPlan {
    bindings: alloc::vec::Vec<alloc::string::String>,
    capabilities: alloc::vec::Vec<alloc::string::String>,
    policy_refs: alloc::vec::Vec<alloc::string::String>,
    authority_refs: alloc::vec::Vec<alloc::string::String>,
    resource_refs: alloc::vec::Vec<alloc::string::String>,
    recorded_effect_refs: alloc::vec::Vec<alloc::string::String>,
}

impl GrantPlan {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            bindings: alloc::vec::Vec::with_capacity(capacity),
            capabilities: alloc::vec::Vec::with_capacity(capacity),
            policy_refs: alloc::vec::Vec::with_capacity(capacity),
            authority_refs: alloc::vec::Vec::with_capacity(capacity),
            resource_refs: alloc::vec::Vec::with_capacity(capacity),
            recorded_effect_refs: alloc::vec::Vec::with_capacity(capacity),
        }
    }
}

fn validate_identity(
    profile: &super::model::RuntimeProfile,
    materialization: &super::materialization::Admission,
    facts: &ArtifactFacts,
    blockers: &mut impl crate::refs::VecSink<alloc::string::String>,
) {
    if facts.artifact_kind != super::model::ArtifactKind::Component {
        blockers.push("component execution plan received a core module".to_string());
    }
    let is_admitted_profile = [
        super::profile::COMPONENT_PROFILE_ID,
        super::profile::TIME_SLICED_PROFILE_ID,
        super::profile::HOST_PROFILE_ID,
    ]
    .contains(&facts.declared_profile_id.as_str());
    if !is_admitted_profile || facts.declared_profile_id != profile.profile_id {
        blockers.push("component artifact declared profile is stale or mismatched".to_string());
    }
    if facts.declared_cohort_ref != super::profile::r#ref(profile)
        || materialization.profile_ref != super::profile::r#ref(profile)
    {
        blockers.push("component artifact or materialization cohort identity is stale".to_string());
    }
    if facts.declared_world != profile.wit.world {
        blockers.push("component artifact WIT world does not match the admitted world".to_string());
    }
    if !facts.exports.iter().any(|value| value == COMPONENT_INVOKE_EXPORT) {
        blockers.push("component artifact does not export the admitted invoke function".to_string());
    }
    if super::model::sorted_unique(&facts.exports) != facts.exports {
        blockers.push("component export facts must be sorted and unique".to_string());
    }
}

fn validate_resources(
    profile: &super::model::RuntimeProfile,
    facts: &ArtifactFacts,
    blockers: &mut impl crate::refs::VecSink<alloc::string::String>,
) {
    validate_fixed_growth("memory", &facts.memory, profile.resources.max_memory_bytes, blockers);
    validate_fixed_growth("table", &facts.table, profile.resources.max_table_elements, blockers);
    for (label, actual, maximum) in [
        ("instances", facts.instances, profile.resources.max_instances),
        ("memories", facts.memories, profile.resources.max_memories),
        ("tables", facts.tables, profile.resources.max_tables),
    ] {
        if actual > maximum {
            blockers.push(alloc::format!("component artifact {label} exceed the admitted resource bound"));
        }
    }
    validate_collection_bound("imports", facts.imports.len(), profile.resources.max_imports, blockers);
    validate_collection_bound("exports", facts.exports.len(), profile.resources.max_exports, blockers);
}

fn validate_collection_bound(
    label: &str,
    actual: usize,
    maximum: u64,
    blockers: &mut impl crate::refs::VecSink<alloc::string::String>,
) {
    match u64::try_from(actual) {
        Ok(actual) if actual <= maximum => {}
        Ok(_) => blockers.push(alloc::format!("component artifact {label} exceed the admitted resource bound")),
        Err(error) => blockers.push(alloc::format!("component artifact {label} count is unsupported: {error}")),
    }
}

fn validate_fixed_growth(
    label: &str,
    facts: &GrowthFacts,
    maximum: u64,
    blockers: &mut impl crate::refs::VecSink<alloc::string::String>,
) {
    if facts.strategy != super::model::GrowthStrategy::Fixed || facts.maximum != Some(facts.initial) {
        blockers.push(alloc::format!("component {label} growth is not fixed up front"));
    }
    if facts.initial > maximum {
        blockers.push(alloc::format!("component {label} declaration exceeds the admitted resource bound"));
    }
}

fn validate_imports(
    profile: &super::model::RuntimeProfile,
    facts: &ArtifactFacts,
    grants: &[ImportGrant],
    blockers: &mut impl crate::refs::VecSink<alloc::string::String>,
) -> GrantPlan {
    let mut plan = GrantPlan::with_capacity(facts.imports.len());
    if super::model::sorted_unique(&facts.imports) != facts.imports {
        blockers.push("component import facts must be sorted and unique".to_string());
    }
    let mut used_grants = alloc::vec::Vec::with_capacity(facts.imports.len());
    for import in &facts.imports {
        if !profile.allowed_imports.iter().any(|allowed| allowed == import) {
            blockers.push(alloc::format!("component import {import} is not declared by the profile"));
        }
        if import.starts_with(WASI_IMPORT_PREFIX)
            && !profile.allowed_wasi_interfaces.iter().any(|allowed| allowed == import)
        {
            blockers.push(alloc::format!("ambient WASI import {import} is denied"));
        }
        let matching = grants.iter().filter(|grant| &grant.import == import).collect::<alloc::vec::Vec<_>>();
        if matching.len() != 1 {
            blockers.push(alloc::format!("component import {import} requires exactly one authority grant"));
            continue;
        }
        let grant = matching[0];
        used_grants.push(grant.import.clone());
        if grant.capability.trim().is_empty()
            || !super::model::valid_content_ref(&grant.policy_ref)
            || !super::model::valid_content_ref(&grant.authority_ref)
            || !super::model::valid_content_ref(&grant.resource_ref)
            || !super::model::valid_content_ref(&grant.recorded_effect_ref)
        {
            blockers.push(alloc::format!("component import {import} has incomplete capability or evidence bindings"));
            continue;
        }
        plan.bindings.push(alloc::format!(
            "{}|{}|{}|{}|{}|{}",
            grant.import,
            grant.capability,
            grant.policy_ref,
            grant.authority_ref,
            grant.resource_ref,
            grant.recorded_effect_ref
        ));
        plan.capabilities.push(grant.capability.clone());
        plan.policy_refs.push(grant.policy_ref.clone());
        plan.authority_refs.push(grant.authority_ref.clone());
        plan.resource_refs.push(grant.resource_ref.clone());
        plan.recorded_effect_refs.push(grant.recorded_effect_ref.clone());
    }
    if super::model::sorted_unique(&used_grants)
        != super::model::sorted_unique(&grants.iter().map(|grant| grant.import.clone()).collect::<alloc::vec::Vec<_>>())
    {
        blockers.push("component import grants include unused authority".to_string());
    }
    plan.bindings = super::model::sorted_unique(&plan.bindings);
    plan.capabilities = super::model::sorted_unique(&plan.capabilities);
    plan.policy_refs = super::model::sorted_unique(&plan.policy_refs);
    plan.authority_refs = super::model::sorted_unique(&plan.authority_refs);
    plan.resource_refs = super::model::sorted_unique(&plan.resource_refs);
    plan.recorded_effect_refs = super::model::sorted_unique(&plan.recorded_effect_refs);
    plan
}

fn merge_refs(
    left: &[alloc::string::String],
    right: &[alloc::string::String],
) -> alloc::vec::Vec<alloc::string::String> {
    let mut values = left.to_vec();
    values.extend_from_slice(right);
    super::model::sorted_unique(&values)
}
