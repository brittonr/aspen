use super::super::model::ComponentRuntimeProfile;
use super::super::model::sorted_unique;
use super::ComponentArtifactFacts;

pub(crate) fn validate_features(
    profile: &ComponentRuntimeProfile,
    facts: &ComponentArtifactFacts,
    blockers: &mut Vec<String>,
) {
    if sorted_unique(&facts.enabled_features) != facts.enabled_features {
        blockers.push("component feature facts must be sorted and unique".to_string());
        return;
    }
    let feature_posture = [
        ("bulk-memory", profile.features.bulk_memory),
        ("component-async", profile.features.component_async),
        ("component-model", profile.features.component_model),
        ("custom-page-sizes", profile.features.custom_page_sizes),
        ("exceptions", profile.features.exceptions),
        ("extended-const", profile.features.extended_const),
        ("function-references", profile.features.function_references),
        ("gc", profile.features.gc),
        ("memory64", profile.features.memory64),
        ("multi-memory", profile.features.multi_memory),
        ("multi-value", profile.features.multi_value),
        ("reference-types", profile.features.reference_types),
        ("relaxed-simd", profile.features.relaxed_simd),
        ("simd", profile.features.simd),
        ("tail-call", profile.features.tail_call),
        ("threads", profile.features.threads),
        ("wide-arithmetic", profile.features.wide_arithmetic),
    ];
    let expected = feature_posture
        .iter()
        .filter(|(_, admitted)| *admitted)
        .map(|(name, _)| (*name).to_string())
        .collect::<Vec<_>>();
    if facts.enabled_features != expected {
        blockers.push("component artifact feature posture differs from the admitted profile".to_string());
    }
}
