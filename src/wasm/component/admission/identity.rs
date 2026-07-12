use super::super::evidence::materialization::MaterializationAdmission;
use super::super::model::ComponentRuntimeProfile;
use super::super::model::content_ref;
use super::super::profile::component_profile_ref;
use super::ComponentArtifactFacts;
use super::GrantPlan;

pub(super) fn runtime_configuration_ref(
    profile: &ComponentRuntimeProfile,
    facts: &ComponentArtifactFacts,
    materialization: &MaterializationAdmission,
    grants: &GrantPlan,
) -> String {
    let mut lines = vec![
        format!("profile-ref:{}", component_profile_ref(profile)),
        format!("component-ref:{}", materialization.component_ref),
        format!("wit-ref:{}", materialization.wit_ref),
        format!("artifact-kind:{}", facts.artifact_kind.as_str()),
        format!("declared-profile:{}", facts.declared_profile_id),
        format!("declared-cohort:{}", facts.declared_cohort_ref),
        format!("world:{}", facts.declared_world),
        format!("memory:{}:{:?}:{}", facts.memory.initial, facts.memory.maximum, facts.memory.strategy.as_str()),
        format!("table:{}:{:?}:{}", facts.table.initial, facts.table.maximum, facts.table.strategy.as_str()),
        format!("instances:{}", facts.instances),
        format!("memories:{}", facts.memories),
        format!("tables:{}", facts.tables),
    ];
    lines.extend(facts.imports.iter().map(|value| format!("import:{value}")));
    lines.extend(facts.exports.iter().map(|value| format!("export:{value}")));
    lines.extend(facts.enabled_features.iter().map(|value| format!("feature:{value}")));
    lines.extend(materialization.mantle_evidence_refs.iter().map(|value| format!("mantle-evidence:{value}")));
    lines.extend(materialization.valence_evidence_refs.iter().map(|value| format!("valence-evidence:{value}")));
    lines.extend(materialization.cairn_evidence_refs.iter().map(|value| format!("cairn-evidence:{value}")));
    lines.extend(materialization.policy_refs.iter().map(|value| format!("materialization-policy:{value}")));
    lines.extend(materialization.authority_refs.iter().map(|value| format!("materialization-authority:{value}")));
    lines.extend(materialization.resource_refs.iter().map(|value| format!("materialization-resource:{value}")));
    lines.extend(grants.bindings.iter().map(|value| format!("grant-binding:{value}")));
    lines.extend(grants.capabilities.iter().map(|value| format!("capability:{value}")));
    lines.extend(grants.policy_refs.iter().map(|value| format!("grant-policy:{value}")));
    lines.extend(grants.authority_refs.iter().map(|value| format!("grant-authority:{value}")));
    lines.extend(grants.resource_refs.iter().map(|value| format!("grant-resource:{value}")));
    lines.extend(grants.recorded_effect_refs.iter().map(|value| format!("effect:{value}")));
    content_ref(lines.join("\n").as_bytes())
}
