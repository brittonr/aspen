use super::model::BenchmarkSuite;
use super::model::MaterializationAdmissionSeal;
use super::model::MaterializedPerformanceArtifact;
use super::model::PerformanceArtifactKind;
use super::model::PerformanceDenial;
use super::model::PerformanceResult;
use super::model::content_ref;
use super::model::sorted_unique;
use super::model::valid_content_ref;
use super::model::valid_ref_collection;
use super::profile::WASMTIME_COMPONENT_COHORT;

pub const PERFORMANCE_MANTLE_BUNDLE_SCHEMA: &str = "mantle.wasm-performance-materialization-bundle.v1";
pub const PRECOMPILED_ADMISSION_SCHEMA: &str = "molten.wasm-precompiled-admission.v1";
pub const WIZER_ADMISSION_SCHEMA: &str = "molten.wasm-wizer-admission.v1";
const MAX_PERFORMANCE_ARTIFACT_BYTES: u64 = 67_108_864;
const MAX_PERFORMANCE_EVIDENCE_REFS: usize = 128;
const MINIMUM_REPEATED_WIZER_OUTPUTS: usize = 2;
const REQUIRED_WIZER_NON_CLAIM: &str = "not-semantic-equivalence";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceMaterializationBundle {
    pub schema_id: String,
    pub bundle_ref: String,
    pub kind: PerformanceArtifactKind,
    pub consumer: crate::wasm_component::ComponentConsumer,
    pub source_component_ref: String,
    pub artifact_ref: String,
    pub artifact_length: u64,
    pub component_profile_ref: String,
    pub runtime_configuration_ref: String,
    pub wasmtime_revision: String,
    pub target: String,
    pub cpu_features: Vec<String>,
    pub mantle_stage_receipt_refs: Vec<String>,
    pub valence_sidecar_refs: Vec<String>,
    pub build_input_refs: Vec<String>,
    pub produced_by_mantle: bool,
    pub locally_produced_transform: bool,
}

pub fn performance_materialization_bundle_ref(bundle: &PerformanceMaterializationBundle) -> String {
    let mut lines = vec![
        format!("schema:{}", bundle.schema_id),
        format!("kind:{}", bundle.kind.as_str()),
        format!("consumer:{}", bundle.consumer.as_str()),
        format!("source-component-ref:{}", bundle.source_component_ref),
        format!("artifact-ref:{}", bundle.artifact_ref),
        format!("artifact-length:{}", bundle.artifact_length),
        format!("component-profile-ref:{}", bundle.component_profile_ref),
        format!("runtime-configuration-ref:{}", bundle.runtime_configuration_ref),
        format!("wasmtime-revision:{}", bundle.wasmtime_revision),
        format!("target:{}", bundle.target),
        format!("produced-by-mantle:{}", bundle.produced_by_mantle),
        format!("locally-produced-transform:{}", bundle.locally_produced_transform),
    ];
    lines.extend(sorted_unique(&bundle.cpu_features).into_iter().map(|value| format!("cpu-feature:{value}")));
    lines.extend(
        sorted_unique(&bundle.mantle_stage_receipt_refs)
            .into_iter()
            .map(|value| format!("mantle-stage-ref:{value}")),
    );
    lines.extend(
        sorted_unique(&bundle.valence_sidecar_refs)
            .into_iter()
            .map(|value| format!("valence-sidecar-ref:{value}")),
    );
    lines.extend(sorted_unique(&bundle.build_input_refs).into_iter().map(|value| format!("build-input-ref:{value}")));
    content_ref(lines.join("\n").as_bytes())
}

pub fn verify_performance_materialization(
    suite: &BenchmarkSuite,
    bundle: &PerformanceMaterializationBundle,
    artifact_bytes: &[u8],
) -> PerformanceResult<MaterializedPerformanceArtifact> {
    let component_profile = crate::wasm_component::supported_component_profile().map_err(|error| {
        PerformanceDenial::new(format!("component profile required by performance admission is invalid: {error}"))
    })?;
    let expected_component_profile_ref = crate::wasm_component::component_profile_ref(&component_profile);
    let artifact_length = u64::try_from(artifact_bytes.len())
        .map_err(|error| PerformanceDenial::new(format!("performance artifact length is unsupported: {error}")))?;
    let measured_artifact_ref = content_ref(artifact_bytes);
    let mut blockers = Vec::new();
    if bundle.schema_id != PERFORMANCE_MANTLE_BUNDLE_SCHEMA {
        blockers.push("performance artifact uses an unsupported Mantle bundle schema".to_string());
    }
    if bundle.bundle_ref != performance_materialization_bundle_ref(bundle) {
        blockers.push("performance Mantle bundle identity does not match its canonical fields".to_string());
    }
    if !suite.materialization_bundle_refs.iter().any(|value| value == &bundle.bundle_ref) {
        blockers.push("performance suite does not bind the supplied Mantle bundle".to_string());
    }
    if artifact_length == 0
        || artifact_length > MAX_PERFORMANCE_ARTIFACT_BYTES
        || artifact_length != bundle.artifact_length
        || measured_artifact_ref != bundle.artifact_ref
    {
        blockers.push("performance artifact bytes differ from the remeasured Mantle identity or bound".to_string());
    }
    if bundle.component_profile_ref != expected_component_profile_ref {
        blockers.push("performance Mantle bundle targets a stale component runtime profile".to_string());
    }
    if !bundle.produced_by_mantle || bundle.locally_produced_transform {
        blockers.push(
            "accepted performance artifacts must be produced by Mantle, never by the benchmark shell".to_string(),
        );
    }
    validate_bundle_identity_fields(bundle, &mut blockers);
    validate_artifact_kind(bundle, &mut blockers);
    if matches!(bundle.kind, PerformanceArtifactKind::PortableComponent | PerformanceArtifactKind::WizerComponent)
        && crate::wasm_component::classify_for_profile(
            crate::wasm_component::RequestedExecutionProfile::ComponentV1,
            artifact_bytes,
        )
        .is_err()
    {
        blockers.push("portable or Wizer performance artifact is not a valid component".to_string());
    }
    if !blockers.is_empty() {
        return Err(PerformanceDenial::from_blockers(blockers));
    }
    Ok(MaterializedPerformanceArtifact {
        kind: bundle.kind,
        consumer: bundle.consumer,
        source_component_ref: bundle.source_component_ref.clone(),
        artifact_ref: bundle.artifact_ref.clone(),
        artifact_length: bundle.artifact_length,
        mantle_bundle_ref: bundle.bundle_ref.clone(),
        valence_sidecar_refs: bundle.valence_sidecar_refs.clone(),
        build_receipt_refs: bundle.mantle_stage_receipt_refs.clone(),
        build_input_refs: bundle.build_input_refs.clone(),
        component_profile_ref: bundle.component_profile_ref.clone(),
        runtime_configuration_ref: bundle.runtime_configuration_ref.clone(),
        wasmtime_revision: bundle.wasmtime_revision.clone(),
        target: bundle.target.clone(),
        cpu_features: bundle.cpu_features.clone(),
        _admission_seal: MaterializationAdmissionSeal,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecompiledComponentManifest {
    pub schema_id: String,
    pub source_component_ref: String,
    pub output_ref: String,
    pub wasmtime_revision: String,
    pub runtime_configuration_ref: String,
    pub component_profile_ref: String,
    pub target: String,
    pub cpu_features: Vec<String>,
    pub build_input_refs: Vec<String>,
    pub mantle_precompile_receipt_ref: String,
    pub valence_sidecar_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PrecompiledAdmissionSeal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedPrecompiledComponent {
    output_ref: String,
    runtime_configuration_ref: String,
    component_profile_ref: String,
    target: String,
    cpu_features: Vec<String>,
    mantle_bundle_ref: String,
    _admission_seal: PrecompiledAdmissionSeal,
}

impl AdmittedPrecompiledComponent {
    pub fn output_ref(&self) -> &str {
        &self.output_ref
    }

    pub fn runtime_configuration_ref(&self) -> &str {
        &self.runtime_configuration_ref
    }

    pub fn component_profile_ref(&self) -> &str {
        &self.component_profile_ref
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn cpu_features(&self) -> &[String] {
        &self.cpu_features
    }

    pub fn mantle_bundle_ref(&self) -> &str {
        &self.mantle_bundle_ref
    }

    pub fn verify_bytes_before_deserialization(&self, bytes: &[u8]) -> PerformanceResult<()> {
        if bytes.is_empty() || content_ref(bytes) != self.output_ref {
            return Err(PerformanceDenial::new("precompiled bytes differ from the sealed admission identity"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecompiledRuntimeExpectation {
    pub wasmtime_revision: String,
    pub runtime_configuration_ref: String,
    pub component_profile_ref: String,
    pub target: String,
    pub cpu_features: Vec<String>,
}

pub fn admit_precompiled_component(
    materialized: &MaterializedPerformanceArtifact,
    manifest: &PrecompiledComponentManifest,
    expectation: &PrecompiledRuntimeExpectation,
) -> PerformanceResult<AdmittedPrecompiledComponent> {
    let mut blockers = Vec::new();
    if materialized.kind != PerformanceArtifactKind::PrecompiledComponent {
        blockers.push("precompiled admission received a non-precompiled artifact".to_string());
    }
    if manifest.schema_id != PRECOMPILED_ADMISSION_SCHEMA {
        blockers.push("precompiled component manifest schema is unsupported".to_string());
    }
    if manifest.source_component_ref != materialized.source_component_ref
        || manifest.output_ref != materialized.artifact_ref
        || manifest.runtime_configuration_ref != materialized.runtime_configuration_ref
        || manifest.component_profile_ref != materialized.component_profile_ref
        || manifest.wasmtime_revision != materialized.wasmtime_revision
        || manifest.target != materialized.target
        || manifest.cpu_features != materialized.cpu_features
    {
        blockers.push("precompiled component manifest differs from the admitted Mantle artifact".to_string());
    }
    if manifest.wasmtime_revision != expectation.wasmtime_revision
        || manifest.runtime_configuration_ref != expectation.runtime_configuration_ref
        || manifest.component_profile_ref != expectation.component_profile_ref
        || manifest.target != expectation.target
        || manifest.cpu_features != expectation.cpu_features
    {
        blockers
            .push("precompiled component is stale, cross-target, cross-profile, or cross-configuration".to_string());
    }
    validate_ref_set("precompiled build input", &manifest.build_input_refs, &mut blockers);
    validate_ref_set("precompiled Valence sidecar", &manifest.valence_sidecar_refs, &mut blockers);
    if manifest.build_input_refs != materialized.build_input_refs
        || manifest.valence_sidecar_refs != materialized.valence_sidecar_refs
        || !materialized.build_receipt_refs.iter().any(|value| value == &manifest.mantle_precompile_receipt_ref)
    {
        blockers.push("precompiled component evidence differs from the exact Mantle and Valence links".to_string());
    }
    if blockers.is_empty() {
        Ok(AdmittedPrecompiledComponent {
            output_ref: materialized.artifact_ref.clone(),
            runtime_configuration_ref: materialized.runtime_configuration_ref.clone(),
            component_profile_ref: materialized.component_profile_ref.clone(),
            target: materialized.target.clone(),
            cpu_features: materialized.cpu_features.clone(),
            mantle_bundle_ref: materialized.mantle_bundle_ref.clone(),
            _admission_seal: PrecompiledAdmissionSeal,
        })
    } else {
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizerVirtualImport {
    pub import: String,
    pub input_ref: String,
}

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
    materialized: &MaterializedPerformanceArtifact,
    manifest: &WizerTransformManifest,
) -> PerformanceResult<()> {
    let mut blockers = Vec::new();
    if materialized.kind != PerformanceArtifactKind::WizerComponent {
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
    if manifest.initialization_entrypoint.trim().is_empty() || !valid_content_ref(&manifest.wizer_tool_ref) {
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
        Err(PerformanceDenial::from_blockers(blockers))
    }
}

fn validate_bundle_identity_fields(bundle: &PerformanceMaterializationBundle, blockers: &mut Vec<String>) {
    for (label, value) in [
        ("source component", bundle.source_component_ref.as_str()),
        ("artifact", bundle.artifact_ref.as_str()),
        ("component profile", bundle.component_profile_ref.as_str()),
        ("runtime configuration", bundle.runtime_configuration_ref.as_str()),
    ] {
        if !valid_content_ref(value) {
            blockers.push(format!("performance Mantle bundle {label} ref is malformed"));
        }
    }
    if bundle.wasmtime_revision != WASMTIME_COMPONENT_COHORT
        || bundle.target.trim().is_empty()
        || sorted_unique(&bundle.cpu_features) != bundle.cpu_features
    {
        blockers.push("performance Mantle bundle Wasmtime, target, or CPU features are malformed".to_string());
    }
    validate_ref_set("Mantle stage", &bundle.mantle_stage_receipt_refs, blockers);
    validate_ref_set("Valence sidecar", &bundle.valence_sidecar_refs, blockers);
    validate_ref_set("build input", &bundle.build_input_refs, blockers);
}

fn validate_artifact_kind(bundle: &PerformanceMaterializationBundle, blockers: &mut Vec<String>) {
    match bundle.kind {
        PerformanceArtifactKind::PortableComponent => {
            if bundle.source_component_ref != bundle.artifact_ref {
                blockers.push("portable performance artifact must retain its source component identity".to_string());
            }
        }
        PerformanceArtifactKind::WizerComponent | PerformanceArtifactKind::PrecompiledComponent => {
            if bundle.source_component_ref == bundle.artifact_ref {
                blockers.push(
                    "transformed performance artifact must retain distinct source and output identities".to_string(),
                );
            }
        }
    }
}

fn validate_ref_set(label: &str, refs: &[String], blockers: &mut Vec<String>) {
    if refs.len() > MAX_PERFORMANCE_EVIDENCE_REFS || !valid_ref_collection(refs) {
        blockers.push(format!("performance {label} refs are missing, malformed, duplicate, or unsorted"));
    }
}

fn validate_wizer_imports(manifest: &WizerTransformManifest, blockers: &mut Vec<String>) {
    if manifest.declared_imports.len() > MAX_PERFORMANCE_EVIDENCE_REFS
        || sorted_unique(&manifest.declared_imports) != manifest.declared_imports
        || sorted_unique(&manifest.denied_imports) != manifest.denied_imports
    {
        blockers.push("Wizer declared and denied imports must be bounded, sorted, and unique".to_string());
    }
    let virtual_names = manifest.virtual_imports.iter().map(|binding| binding.import.clone()).collect::<Vec<_>>();
    if manifest.virtual_imports.len() > MAX_PERFORMANCE_EVIDENCE_REFS
        || sorted_unique(&virtual_names) != virtual_names
        || manifest
            .virtual_imports
            .iter()
            .any(|binding| binding.import.trim().is_empty() || !valid_content_ref(&binding.input_ref))
    {
        blockers.push("Wizer virtual imports must bind sorted interfaces to exact deterministic inputs".to_string());
    }
    if manifest
        .denied_imports
        .iter()
        .any(|denied| virtual_names.iter().any(|virtual_name| virtual_name == denied))
    {
        blockers.push("Wizer import cannot be both denied and virtualized".to_string());
    }
    let mut admitted_imports = manifest.denied_imports.clone();
    admitted_imports.extend(virtual_names);
    admitted_imports = sorted_unique(&admitted_imports);
    if admitted_imports != manifest.declared_imports {
        blockers.push("Wizer imports are not completely denied or deterministically virtualized".to_string());
    }
}
