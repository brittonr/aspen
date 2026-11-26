pub(super) fn runtime_configuration_ref(
    profile: &crate::component::model::RuntimeProfile,
    facts: &super::ArtifactFacts,
    materialization: &crate::component::materialization::Admission,
    grants: &super::GrantPlan,
) -> alloc::string::String {
    let mut lines = alloc::vec![
        alloc::format!("profile-ref:{}", crate::component::profile::r#ref(profile)),
        alloc::format!("component-ref:{}", materialization.component_ref),
        alloc::format!("wit-ref:{}", materialization.wit_ref),
        alloc::format!("artifact-kind:{}", facts.artifact_kind.as_str()),
        alloc::format!("declared-profile:{}", facts.declared_profile_id),
        alloc::format!("declared-cohort:{}", facts.declared_cohort_ref),
        alloc::format!("world:{}", facts.declared_world),
        alloc::format!("memory:{}:{:?}:{}", facts.memory.initial, facts.memory.maximum, facts.memory.strategy.as_str()),
        alloc::format!("table:{}:{:?}:{}", facts.table.initial, facts.table.maximum, facts.table.strategy.as_str()),
        alloc::format!("instances:{}", facts.instances),
        alloc::format!("memories:{}", facts.memories),
        alloc::format!("tables:{}", facts.tables),
    ];
    lines.extend(facts.imports.iter().map(|value| alloc::format!("import:{value}")));
    lines.extend(facts.exports.iter().map(|value| alloc::format!("export:{value}")));
    lines.extend(facts.enabled_features.iter().map(|value| alloc::format!("feature:{value}")));
    lines.extend(materialization.mantle_evidence_refs.iter().map(|value| alloc::format!("mantle-evidence:{value}")));
    lines.extend(materialization.valence_evidence_refs.iter().map(|value| alloc::format!("valence-evidence:{value}")));
    lines.extend(materialization.cairn_evidence_refs.iter().map(|value| alloc::format!("cairn-evidence:{value}")));
    lines.extend(materialization.policy_refs.iter().map(|value| alloc::format!("materialization-policy:{value}")));
    lines.extend(
        materialization
            .authority_refs
            .iter()
            .map(|value| alloc::format!("materialization-authority:{value}")),
    );
    lines.extend(materialization.resource_refs.iter().map(|value| alloc::format!("materialization-resource:{value}")));
    lines.extend(grants.bindings.iter().map(|value| alloc::format!("grant-binding:{value}")));
    lines.extend(grants.capabilities.iter().map(|value| alloc::format!("capability:{value}")));
    lines.extend(grants.policy_refs.iter().map(|value| alloc::format!("grant-policy:{value}")));
    lines.extend(grants.authority_refs.iter().map(|value| alloc::format!("grant-authority:{value}")));
    lines.extend(grants.resource_refs.iter().map(|value| alloc::format!("grant-resource:{value}")));
    lines.extend(grants.recorded_effect_refs.iter().map(|value| alloc::format!("effect:{value}")));
    crate::component::model::content_ref(lines.join("\n").as_bytes())
}
