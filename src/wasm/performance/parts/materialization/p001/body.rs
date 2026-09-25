
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizerTransformManifest {
    pub schema_id: String,
    pub original_component_ref: String,
    pub transformed_component_ref: String,
    pub initialization_entrypoint: String,
    pub wizer_tool_ref: String,
    pub declared_imports: Vec<String>,
    pub denied_imports: Vec<String>,
    pub virtual_imports: Vec<WizerVirtualImport>,
    pub repeated_output_refs: Vec<String>,
    pub pre_transform_receipt_ref: String,
    pub post_transform_receipt_ref: String,
    pub observed_ambient_state: bool,
    pub non_claims: Vec<String>,
}

pub fn admit_wizer_artifact(
    materialized: &super::model::MaterializedPerformanceArtifact,
    manifest: &WizerTransformManifest,
) -> super::model::PerformanceResult<()> {
    let mut blockers = Vec::new();
    if materialized.kind != super::model::PerformanceArtifactKind::WizerComponent {
        blockers.push("Wizer admission received a non-Wizer artifact".to_string());
    }
    if manifest.schema_id != WIZER_ADMISSION_SCHEMA {
        blockers.push("Wizer transform manifest schema is unsupported".to_string());
    }
    if manifest.original_component_ref != materialized.source_component_ref
        || manifest.transformed_component_ref != materialized.artifact_ref
    {
        blockers.push("Wizer transform identities differ from the admitted Mantle artifact".to_string());
    }
    if manifest.initialization_entrypoint.trim().is_empty()
        || !super::model::valid_content_ref(&manifest.wizer_tool_ref)
    {
        blockers.push("Wizer transform lacks a bounded entrypoint or tool identity".to_string());
    }
    if manifest.observed_ambient_state {
        blockers.push("Wizer transform observed ambient host state".to_string());
    }
    validate_wizer_imports(manifest, &mut blockers);
    if manifest.repeated_output_refs.len() < MINIMUM_REPEATED_WIZER_OUTPUTS
        || manifest.repeated_output_refs.iter().any(|value| value != &manifest.transformed_component_ref)
    {
        blockers.push("independent Wizer transforms did not produce one repeated output identity".to_string());
    }
    if !materialized.build_receipt_refs.iter().any(|value| value == &manifest.pre_transform_receipt_ref)
        || !materialized.build_receipt_refs.iter().any(|value| value == &manifest.post_transform_receipt_ref)
    {
        blockers.push("Wizer transform lacks exact pre/post Mantle receipt links".to_string());
    }
    if !manifest.non_claims.iter().any(|value| value == REQUIRED_WIZER_NON_CLAIM) {
        blockers.push("Wizer transform omits the semantic-equivalence non-claim".to_string());
    }
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

fn validate_bundle_identity_fields(
    bundle: &PerformanceMaterializationBundle,
    blockers: &mut impl crate::bounded::VecSink<String>,
) {
    for (label, value) in [
        ("source component", bundle.source_component_ref.as_str()),
        ("artifact", bundle.artifact_ref.as_str()),
        ("component profile", bundle.component_profile_ref.as_str()),
        ("runtime configuration", bundle.runtime_configuration_ref.as_str()),
    ] {
        if !super::model::valid_content_ref(value) {
            blockers.push_item(format!("performance Mantle bundle {label} ref is malformed"));
        }
    }
    if bundle.wasmtime_revision != super::profile::WASMTIME_COMPONENT_COHORT
        || bundle.target.trim().is_empty()
        || super::model::sorted_unique(&bundle.cpu_features) != bundle.cpu_features
    {
        blockers.push_item("performance Mantle bundle Wasmtime, target, or CPU features are malformed".to_string());
    }
    validate_ref_set("Mantle stage", &bundle.mantle_stage_receipt_refs, blockers);
    validate_ref_set("Valence sidecar", &bundle.valence_sidecar_refs, blockers);
    validate_ref_set("build input", &bundle.build_input_refs, blockers);
}

fn validate_artifact_kind(
    bundle: &PerformanceMaterializationBundle,
    blockers: &mut impl crate::bounded::VecSink<String>,
) {
    match bundle.kind {
        super::model::PerformanceArtifactKind::PortableComponent => {
            if bundle.source_component_ref != bundle.artifact_ref {
                blockers
                    .push_item("portable performance artifact must retain its source component identity".to_string());
            }
        }
        super::model::PerformanceArtifactKind::WizerComponent
        | super::model::PerformanceArtifactKind::PrecompiledComponent => {
            if bundle.source_component_ref == bundle.artifact_ref {
                blockers.push_item(
                    "transformed performance artifact must retain distinct source and output identities".to_string(),
                );
            }
        }
    }
}

fn validate_ref_set(label: &str, refs: &[String], blockers: &mut impl crate::bounded::VecSink<String>) {
    if refs.len() > MAX_PERFORMANCE_EVIDENCE_REFS || !super::model::valid_ref_collection(refs) {
        blockers.push_item(format!("performance {label} refs are missing, malformed, duplicate, or unsorted"));
    }
}

fn validate_wizer_imports(manifest: &WizerTransformManifest, blockers: &mut impl crate::bounded::VecSink<String>) {
    if manifest.declared_imports.len() > MAX_PERFORMANCE_EVIDENCE_REFS
        || super::model::sorted_unique(&manifest.declared_imports) != manifest.declared_imports
        || super::model::sorted_unique(&manifest.denied_imports) != manifest.denied_imports
    {
        blockers.push_item("Wizer declared and denied imports must be bounded, sorted, and unique".to_string());
    }
    let virtual_names = manifest.virtual_imports.iter().map(|binding| binding.import.clone()).collect::<Vec<_>>();
    if manifest.virtual_imports.len() > MAX_PERFORMANCE_EVIDENCE_REFS
        || super::model::sorted_unique(&virtual_names) != virtual_names
        || manifest
            .virtual_imports
            .iter()
            .any(|binding| binding.import.trim().is_empty() || !super::model::valid_content_ref(&binding.input_ref))
    {
        blockers
            .push_item("Wizer virtual imports must bind sorted interfaces to exact deterministic inputs".to_string());
    }
    if manifest
        .denied_imports
        .iter()
        .any(|denied| virtual_names.iter().any(|virtual_name| virtual_name == denied))
    {
        blockers.push_item("Wizer import cannot be both denied and virtualized".to_string());
    }
    let mut admitted_imports = manifest.denied_imports.clone();
    admitted_imports.extend(virtual_names);
    admitted_imports = super::model::sorted_unique(&admitted_imports);
    if admitted_imports != manifest.declared_imports {
        blockers.push_item("Wizer imports are not completely denied or deterministically virtualized".to_string());
    }
}
