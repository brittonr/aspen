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
    pub kind: super::model::PerformanceArtifactKind,
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
    lines.extend(
        super::model::sorted_unique(&bundle.cpu_features)
            .into_iter()
            .map(|value| format!("cpu-feature:{value}")),
    );
    lines.extend(
        super::model::sorted_unique(&bundle.mantle_stage_receipt_refs)
            .into_iter()
            .map(|value| format!("mantle-stage-ref:{value}")),
    );
    lines.extend(
        super::model::sorted_unique(&bundle.valence_sidecar_refs)
            .into_iter()
            .map(|value| format!("valence-sidecar-ref:{value}")),
    );
    lines.extend(
        super::model::sorted_unique(&bundle.build_input_refs)
            .into_iter()
            .map(|value| format!("build-input-ref:{value}")),
    );
    super::model::content_ref(lines.join("\n").as_bytes())
}

pub fn verify_performance_materialization(
    suite: &super::model::BenchmarkSuite,
    bundle: &PerformanceMaterializationBundle,
    artifact_bytes: &[u8],
) -> super::model::PerformanceResult<super::model::MaterializedPerformanceArtifact> {
    let component_profile = crate::wasm_component::supported_component_profile().map_err(|error| {
        super::model::PerformanceDenial::new(format!(
            "component profile required by performance admission is invalid: {error}"
        ))
    })?;
    let expected_component_profile_ref = crate::wasm_component::component_profile_ref(&component_profile);
    let artifact_length = u64::try_from(artifact_bytes.len()).map_err(|error| {
        super::model::PerformanceDenial::new(format!("performance artifact length is unsupported: {error}"))
    })?;
    let measured_artifact_ref = super::model::content_ref(artifact_bytes);
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
    validate_component_bytes(bundle, artifact_bytes, &mut blockers);
    if !blockers.is_empty() {
        return Err(super::model::PerformanceDenial::from_blockers(blockers));
    }
    Ok(super::model::MaterializedPerformanceArtifact {
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
        _admission_seal: super::model::MaterializationAdmissionSeal,
    })
}

fn validate_component_bytes(
    bundle: &PerformanceMaterializationBundle,
    artifact_bytes: &[u8],
    blockers: &mut impl crate::bounded::VecSink<String>,
) {
    if matches!(
        bundle.kind,
        super::model::PerformanceArtifactKind::PortableComponent
            | super::model::PerformanceArtifactKind::WizerComponent
    ) && crate::wasm_component::classify_for_profile(
        crate::wasm_component::RequestedExecutionProfile::ComponentV1,
        artifact_bytes,
    )
    .is_err()
    {
        blockers.push_item("portable or Wizer performance artifact is not a valid component".to_string());
    }
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

    pub fn verify_bytes_before_deserialization(&self, bytes: &[u8]) -> super::model::PerformanceResult<()> {
        if bytes.is_empty() || super::model::content_ref(bytes) != self.output_ref {
            return Err(super::model::PerformanceDenial::new(
                "precompiled bytes differ from the sealed admission identity",
            ));
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
    materialized: &super::model::MaterializedPerformanceArtifact,
    manifest: &PrecompiledComponentManifest,
    expectation: &PrecompiledRuntimeExpectation,
) -> super::model::PerformanceResult<AdmittedPrecompiledComponent> {
    let mut blockers = Vec::new();
    if materialized.kind != super::model::PerformanceArtifactKind::PrecompiledComponent {
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
        Err(super::model::PerformanceDenial::from_blockers(blockers))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WizerVirtualImport {
    pub import: String,
    pub input_ref: String,
}
