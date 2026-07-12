use super::super::*;
use super::support::fixture_alternate_component_bytes;
use super::support::fixture_bundle;
use super::support::fixture_bytes_ref;
use super::support::fixture_component_bytes;
use super::support::fixture_materialized;
use super::support::fixture_precompiled_component_bytes;
use super::support::fixture_ref;

#[test]
fn portable_wizer_and_precompiled_artifacts_require_exact_mantle_materialization() {
    // r[verify molten.wasm_performance.materialization]
    // r[verify molten.wasm_performance.aot_admission]
    // r[verify molten.wasm_performance.wizer]
    let profile = supported_performance_profile().expect("supported performance profile");
    let portable_bytes = fixture_component_bytes();
    let portable_source = fixture_bytes_ref(&portable_bytes);
    let (_portable_suite, _portable_bundle, portable) =
        fixture_materialized(&profile, PerformanceArtifactKind::PortableComponent, &portable_bytes, portable_source);
    assert_eq!(portable.kind, PerformanceArtifactKind::PortableComponent);
    assert_eq!(portable.consumer, crate::wasm_component::ComponentConsumer::Actor);

    let mut system_bundle =
        fixture_bundle(PerformanceArtifactKind::PortableComponent, &portable_bytes, portable.artifact_ref.clone());
    system_bundle.consumer = crate::wasm_component::ComponentConsumer::SystemExtension;
    system_bundle.bundle_ref = performance_materialization_bundle_ref(&system_bundle);
    let mut system_suite = profile.fast.clone();
    system_suite.materialization_bundle_refs = vec![system_bundle.bundle_ref.clone()];
    let system_extension = verify_performance_materialization(&system_suite, &system_bundle, &portable_bytes)
        .expect("system-extension performance materialization");
    assert_eq!(system_extension.consumer, crate::wasm_component::ComponentConsumer::SystemExtension);

    let source_ref = portable.artifact_ref.clone();
    let wizer_bytes = fixture_alternate_component_bytes();
    let (_wizer_suite, _wizer_bundle, wizer) =
        fixture_materialized(&profile, PerformanceArtifactKind::WizerComponent, &wizer_bytes, source_ref.clone());
    let wizer_manifest = WizerTransformManifest {
        schema_id: WIZER_ADMISSION_SCHEMA.to_string(),
        original_component_ref: source_ref.clone(),
        transformed_component_ref: wizer.artifact_ref.clone(),
        initialization_entrypoint: "wizer.initialize".to_string(),
        wizer_tool_ref: fixture_ref("wizer-tool"),
        declared_imports: vec!["wasi:cli/environment".to_string()],
        denied_imports: vec!["wasi:cli/environment".to_string()],
        virtual_imports: Vec::new(),
        repeated_output_refs: vec![wizer.artifact_ref.clone(), wizer.artifact_ref.clone()],
        pre_transform_receipt_ref: fixture_ref("wizer-pre-receipt"),
        post_transform_receipt_ref: fixture_ref("wizer-post-receipt"),
        observed_ambient_state: false,
        non_claims: vec!["not-semantic-equivalence".to_string()],
    };
    admit_wizer_artifact(&wizer, &wizer_manifest).expect("Wizer artifact admitted");

    let precompiled_bytes = fixture_precompiled_component_bytes();
    let (_aot_suite, _aot_bundle, precompiled) =
        fixture_materialized(&profile, PerformanceArtifactKind::PrecompiledComponent, &precompiled_bytes, source_ref);
    let manifest = PrecompiledComponentManifest {
        schema_id: PRECOMPILED_ADMISSION_SCHEMA.to_string(),
        source_component_ref: precompiled.source_component_ref.clone(),
        output_ref: precompiled.artifact_ref.clone(),
        wasmtime_revision: precompiled.wasmtime_revision.clone(),
        runtime_configuration_ref: precompiled.runtime_configuration_ref.clone(),
        component_profile_ref: precompiled.component_profile_ref.clone(),
        target: precompiled.target.clone(),
        cpu_features: precompiled.cpu_features.clone(),
        build_input_refs: precompiled.build_input_refs.clone(),
        mantle_precompile_receipt_ref: fixture_ref("mantle-precompile-receipt"),
        valence_sidecar_refs: precompiled.valence_sidecar_refs.clone(),
    };
    let expectation = PrecompiledRuntimeExpectation {
        wasmtime_revision: manifest.wasmtime_revision.clone(),
        runtime_configuration_ref: manifest.runtime_configuration_ref.clone(),
        component_profile_ref: manifest.component_profile_ref.clone(),
        target: manifest.target.clone(),
        cpu_features: manifest.cpu_features.clone(),
    };
    let admission = admit_precompiled_component(&precompiled, &manifest, &expectation)
        .expect("precompiled artifact admitted before deserialization");
    assert_eq!(admission.output_ref(), precompiled.artifact_ref);
    admission
        .verify_bytes_before_deserialization(&precompiled_bytes)
        .expect("sealed AOT bytes remeasure before deserialization");
    assert!(admission.verify_bytes_before_deserialization(b"tampered-precompile").is_err());
}

#[test]
fn local_tampered_incomplete_aot_and_wizer_artifacts_deny_before_use() {
    // r[verify molten.wasm_performance.materialization]
    // r[verify molten.wasm_performance.aot_admission]
    // r[verify molten.wasm_performance.wizer]
    // r[verify molten.wasm_performance.validation]
    let profile = supported_performance_profile().expect("supported performance profile");
    let invalid_component_bytes = b"not-a-component";
    let invalid_component_ref = fixture_bytes_ref(invalid_component_bytes);
    let invalid_component_bundle =
        fixture_bundle(PerformanceArtifactKind::PortableComponent, invalid_component_bytes, invalid_component_ref);
    let mut invalid_component_suite = profile.fast.clone();
    invalid_component_suite.materialization_bundle_refs = vec![invalid_component_bundle.bundle_ref.clone()];
    assert!(
        verify_performance_materialization(
            &invalid_component_suite,
            &invalid_component_bundle,
            invalid_component_bytes,
        )
        .is_err()
    );

    let source_bytes = fixture_component_bytes();
    let source_ref = fixture_bytes_ref(&source_bytes);
    let bytes = fixture_precompiled_component_bytes();
    let mut bundle = fixture_bundle(PerformanceArtifactKind::PrecompiledComponent, &bytes, source_ref.clone());
    let mut suite = profile.fast.clone();
    suite.materialization_bundle_refs = vec![bundle.bundle_ref.clone()];

    assert!(verify_performance_materialization(&suite, &bundle, b"tampered").is_err());

    bundle.locally_produced_transform = true;
    bundle.bundle_ref = performance_materialization_bundle_ref(&bundle);
    suite.materialization_bundle_refs = vec![bundle.bundle_ref.clone()];
    assert!(verify_performance_materialization(&suite, &bundle, &bytes).is_err());

    bundle.locally_produced_transform = false;
    bundle.valence_sidecar_refs.clear();
    bundle.bundle_ref = performance_materialization_bundle_ref(&bundle);
    suite.materialization_bundle_refs = vec![bundle.bundle_ref.clone()];
    assert!(verify_performance_materialization(&suite, &bundle, &bytes).is_err());

    let (_aot_suite, _aot_bundle, precompiled) =
        fixture_materialized(&profile, PerformanceArtifactKind::PrecompiledComponent, &bytes, source_ref.clone());
    let manifest = PrecompiledComponentManifest {
        schema_id: PRECOMPILED_ADMISSION_SCHEMA.to_string(),
        source_component_ref: precompiled.source_component_ref.clone(),
        output_ref: precompiled.artifact_ref.clone(),
        wasmtime_revision: precompiled.wasmtime_revision.clone(),
        runtime_configuration_ref: precompiled.runtime_configuration_ref.clone(),
        component_profile_ref: precompiled.component_profile_ref.clone(),
        target: precompiled.target.clone(),
        cpu_features: precompiled.cpu_features.clone(),
        build_input_refs: precompiled.build_input_refs.clone(),
        mantle_precompile_receipt_ref: fixture_ref("mantle-precompile-receipt"),
        valence_sidecar_refs: precompiled.valence_sidecar_refs.clone(),
    };
    let mut cross_target = PrecompiledRuntimeExpectation {
        wasmtime_revision: manifest.wasmtime_revision.clone(),
        runtime_configuration_ref: manifest.runtime_configuration_ref.clone(),
        component_profile_ref: manifest.component_profile_ref.clone(),
        target: manifest.target.clone(),
        cpu_features: manifest.cpu_features.clone(),
    };
    cross_target.target = "aarch64-unknown-linux-gnu".to_string();
    assert!(admit_precompiled_component(&precompiled, &manifest, &cross_target).is_err());

    let wizer_bytes = fixture_alternate_component_bytes();
    let (_wizer_suite, _wizer_bundle, wizer) =
        fixture_materialized(&profile, PerformanceArtifactKind::WizerComponent, &wizer_bytes, source_ref);
    let mut drifting = WizerTransformManifest {
        schema_id: WIZER_ADMISSION_SCHEMA.to_string(),
        original_component_ref: wizer.source_component_ref.clone(),
        transformed_component_ref: wizer.artifact_ref.clone(),
        initialization_entrypoint: "wizer.initialize".to_string(),
        wizer_tool_ref: fixture_ref("wizer-tool"),
        declared_imports: vec!["wasi:clocks/wall-clock".to_string()],
        denied_imports: Vec::new(),
        virtual_imports: vec![WizerVirtualImport {
            import: "wasi:clocks/wall-clock".to_string(),
            input_ref: fixture_ref("virtual-clock"),
        }],
        repeated_output_refs: vec![wizer.artifact_ref.clone(), fixture_ref("drifted-output")],
        pre_transform_receipt_ref: fixture_ref("wizer-pre-receipt"),
        post_transform_receipt_ref: fixture_ref("wizer-post-receipt"),
        observed_ambient_state: false,
        non_claims: vec!["not-semantic-equivalence".to_string()],
    };
    assert!(admit_wizer_artifact(&wizer, &drifting).is_err());
    drifting.repeated_output_refs = vec![wizer.artifact_ref.clone(), wizer.artifact_ref.clone()];
    drifting.observed_ambient_state = true;
    assert!(admit_wizer_artifact(&wizer, &drifting).is_err());
}
