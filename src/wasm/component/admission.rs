pub(crate) mod features;
mod identity;
pub(crate) mod inspection;

use super::evidence::materialization::MaterializationAdmission;
use super::model::ComponentDenial;
use super::model::ComponentResult;
use super::model::ComponentRuntimeProfile;
use super::model::GrowthStrategy;
use super::model::WasmArtifactKind;
use super::model::sorted_unique;
use super::model::valid_content_ref;
use super::profile::COMPONENT_PROFILE_ID;
use super::profile::COMPONENT_WIT_WORLD;
use super::profile::component_profile_ref;
use super::profile::validate_component_profile;

pub const COMPONENT_INVOKE_EXPORT: &str = "invoke";
pub const WASI_IMPORT_PREFIX: &str = "wasi:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentGrowthFacts {
    pub initial: u64,
    pub maximum: Option<u64>,
    pub strategy: GrowthStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentArtifactFacts {
    pub artifact_kind: WasmArtifactKind,
    pub declared_profile_id: String,
    pub declared_cohort_ref: String,
    pub declared_world: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub enabled_features: Vec<String>,
    pub memory: ComponentGrowthFacts,
    pub table: ComponentGrowthFacts,
    pub instances: u64,
    pub memories: u64,
    pub tables: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentImportGrant {
    pub import: String,
    pub capability: String,
    pub policy_ref: String,
    pub authority_ref: String,
    pub resource_ref: String,
    pub recorded_effect_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentExecutionPlan {
    pub component_ref: String,
    pub wit_ref: String,
    pub bundle_ref: Option<String>,
    pub profile_ref: String,
    pub runtime_configuration_ref: String,
    pub imports: Vec<String>,
    pub capabilities: Vec<String>,
    pub mantle_evidence_refs: Vec<String>,
    pub valence_evidence_refs: Vec<String>,
    pub cairn_evidence_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub recorded_effect_refs: Vec<String>,
    pub materialization: MaterializationAdmission,
}

pub fn plan_component_execution(
    profile: &ComponentRuntimeProfile,
    materialization: MaterializationAdmission,
    facts: &ComponentArtifactFacts,
    grants: &[ComponentImportGrant],
) -> ComponentResult<ComponentExecutionPlan> {
    validate_component_profile(profile)?;
    let mut blockers = Vec::new();
    validate_identity(profile, &materialization, facts, &mut blockers);
    features::validate_features(profile, facts, &mut blockers);
    validate_resources(profile, facts, &mut blockers);
    if !blockers.is_empty() {
        return Err(ComponentDenial::from_blockers(blockers));
    }
    let grant_plan = validate_imports(profile, facts, grants, &mut blockers);
    if !blockers.is_empty() {
        return Err(ComponentDenial::from_blockers(blockers));
    }
    let runtime_configuration_ref = identity::runtime_configuration_ref(profile, facts, &materialization, &grant_plan);
    Ok(ComponentExecutionPlan {
        component_ref: materialization.component_ref.clone(),
        wit_ref: materialization.wit_ref.clone(),
        bundle_ref: materialization.bundle_ref.clone(),
        profile_ref: component_profile_ref(profile),
        runtime_configuration_ref,
        imports: sorted_unique(&facts.imports),
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
    bindings: Vec<String>,
    capabilities: Vec<String>,
    policy_refs: Vec<String>,
    authority_refs: Vec<String>,
    resource_refs: Vec<String>,
    recorded_effect_refs: Vec<String>,
}

impl GrantPlan {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            bindings: Vec::with_capacity(capacity),
            capabilities: Vec::with_capacity(capacity),
            policy_refs: Vec::with_capacity(capacity),
            authority_refs: Vec::with_capacity(capacity),
            resource_refs: Vec::with_capacity(capacity),
            recorded_effect_refs: Vec::with_capacity(capacity),
        }
    }
}

fn validate_identity(
    profile: &ComponentRuntimeProfile,
    materialization: &MaterializationAdmission,
    facts: &ComponentArtifactFacts,
    blockers: &mut Vec<String>,
) {
    if facts.artifact_kind != WasmArtifactKind::Component {
        blockers.push("component execution plan received a core module".to_string());
    }
    if facts.declared_profile_id != COMPONENT_PROFILE_ID || facts.declared_profile_id != profile.profile_id {
        blockers.push("component artifact declared profile is stale or mismatched".to_string());
    }
    if facts.declared_cohort_ref != component_profile_ref(profile)
        || materialization.profile_ref != component_profile_ref(profile)
    {
        blockers.push("component artifact or materialization cohort identity is stale".to_string());
    }
    if facts.declared_world != COMPONENT_WIT_WORLD || facts.declared_world != profile.wit.world {
        blockers.push("component artifact WIT world does not match the admitted world".to_string());
    }
    if !facts.exports.iter().any(|value| value == COMPONENT_INVOKE_EXPORT) {
        blockers.push("component artifact does not export the admitted invoke function".to_string());
    }
    if sorted_unique(&facts.exports) != facts.exports {
        blockers.push("component export facts must be sorted and unique".to_string());
    }
}

fn validate_resources(profile: &ComponentRuntimeProfile, facts: &ComponentArtifactFacts, blockers: &mut Vec<String>) {
    validate_fixed_growth("memory", &facts.memory, profile.resources.max_memory_bytes, blockers);
    validate_fixed_growth("table", &facts.table, profile.resources.max_table_elements, blockers);
    for (label, actual, maximum) in [
        ("instances", facts.instances, profile.resources.max_instances),
        ("memories", facts.memories, profile.resources.max_memories),
        ("tables", facts.tables, profile.resources.max_tables),
    ] {
        if actual > maximum {
            blockers.push(format!("component artifact {label} exceed the admitted resource bound"));
        }
    }
    validate_collection_bound("imports", facts.imports.len(), profile.resources.max_imports, blockers);
    validate_collection_bound("exports", facts.exports.len(), profile.resources.max_exports, blockers);
}

fn validate_collection_bound(label: &str, actual: usize, maximum: u64, blockers: &mut Vec<String>) {
    match u64::try_from(actual) {
        Ok(actual) if actual <= maximum => {}
        Ok(_) => blockers.push(format!("component artifact {label} exceed the admitted resource bound")),
        Err(error) => blockers.push(format!("component artifact {label} count is unsupported: {error}")),
    }
}

fn validate_fixed_growth(label: &str, facts: &ComponentGrowthFacts, maximum: u64, blockers: &mut Vec<String>) {
    if facts.strategy != GrowthStrategy::Fixed || facts.maximum != Some(facts.initial) {
        blockers.push(format!("component {label} growth is not fixed up front"));
    }
    if facts.initial > maximum {
        blockers.push(format!("component {label} declaration exceeds the admitted resource bound"));
    }
}

fn validate_imports(
    profile: &ComponentRuntimeProfile,
    facts: &ComponentArtifactFacts,
    grants: &[ComponentImportGrant],
    blockers: &mut Vec<String>,
) -> GrantPlan {
    let mut plan = GrantPlan::with_capacity(facts.imports.len());
    if sorted_unique(&facts.imports) != facts.imports {
        blockers.push("component import facts must be sorted and unique".to_string());
    }
    let mut used_grants = Vec::with_capacity(facts.imports.len());
    for import in &facts.imports {
        if !profile.allowed_imports.iter().any(|allowed| allowed == import) {
            blockers.push(format!("component import {import} is not declared by the profile"));
        }
        if import.starts_with(WASI_IMPORT_PREFIX)
            && !profile.allowed_wasi_interfaces.iter().any(|allowed| allowed == import)
        {
            blockers.push(format!("ambient WASI import {import} is denied"));
        }
        let matching = grants.iter().filter(|grant| &grant.import == import).collect::<Vec<_>>();
        if matching.len() != 1 {
            blockers.push(format!("component import {import} requires exactly one authority grant"));
            continue;
        }
        let grant = matching[0];
        used_grants.push(grant.import.clone());
        if grant.capability.trim().is_empty()
            || !valid_content_ref(&grant.policy_ref)
            || !valid_content_ref(&grant.authority_ref)
            || !valid_content_ref(&grant.resource_ref)
            || !valid_content_ref(&grant.recorded_effect_ref)
        {
            blockers.push(format!("component import {import} has incomplete capability or evidence bindings"));
            continue;
        }
        plan.bindings.push(format!(
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
    if sorted_unique(&used_grants)
        != sorted_unique(&grants.iter().map(|grant| grant.import.clone()).collect::<Vec<_>>())
    {
        blockers.push("component import grants include unused authority".to_string());
    }
    plan.bindings = sorted_unique(&plan.bindings);
    plan.capabilities = sorted_unique(&plan.capabilities);
    plan.policy_refs = sorted_unique(&plan.policy_refs);
    plan.authority_refs = sorted_unique(&plan.authority_refs);
    plan.resource_refs = sorted_unique(&plan.resource_refs);
    plan.recorded_effect_refs = sorted_unique(&plan.recorded_effect_refs);
    plan
}

fn merge_refs(left: &[String], right: &[String]) -> Vec<String> {
    let mut values = left.to_vec();
    values.extend_from_slice(right);
    sorted_unique(&values)
}
