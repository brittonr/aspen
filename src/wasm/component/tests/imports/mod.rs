use super::super::*;

fn fixture_record(fixture: &super::support::ComponentFixture) -> AdmissionRecord {
    let component_ref = fixture.bundle.component.content_ref.clone();
    record_surface_admission(
        &fixture.profile,
        ManifestSource::Provided(&fixture.import_manifest),
        &component_ref,
        &fixture.component_bytes,
    )
    .expect("surface admission record")
}

#[test]
fn exact_manifest_world_and_import_set_admit_without_instantiation() {
    // r[verify aspen.wasm_import_admission.manifest]
    // r[verify aspen.wasm_import_admission.surface]
    // r[verify aspen.wasm_import_admission.admission]
    let fixture = super::support::ComponentFixture::new(ComponentConsumer::Actor);
    let record = fixture_record(&fixture);
    assert!(record.admission.is_admitted);
    assert_eq!(record.observation.imports, Vec::<String>::new());
    assert_eq!(record.observation.exports, vec![COMPONENT_INVOKE_EXPORT.to_string()]);
    assert_eq!(record.observation.extraction_tool, expected_extraction_tool());
    assert_eq!(record.observation.verifier, ADMISSION_VERIFIER);
    assert!(validate_admission_evidence(&record.receipt).is_ok());
    assert!(validate_non_claims(&record.receipt).is_ok());
}

#[test]
fn identical_surface_admissions_replay_to_identical_receipts() {
    // r[verify aspen.wasm_import_admission.admission]
    // r[verify aspen.wasm_import_admission.evidence]
    let fixture = super::support::ComponentFixture::new(ComponentConsumer::Actor);
    let left = fixture_record(&fixture);
    let right = fixture_record(&fixture);
    assert_eq!(left, right);
    assert_eq!(left.receipt.receipt_ref, right.receipt.receipt_ref);
}

#[test]
fn drifting_world_partial_manifest_and_wasi_surface_fail_admission() {
    // r[verify aspen.wasm_import_admission.manifest]
    // r[verify aspen.wasm_import_admission.surface]
    let fixture = super::support::ComponentFixture::new(ComponentConsumer::Actor);
    let world = declared_world(&fixture.profile);

    let mut drifting = fixture.import_manifest.clone();
    drifting.world = "other-world".to_string();
    let observation = facts_observation(&fixture.bundle.component.content_ref, &fixture.facts);
    let verdict = admit_declared(&drifting, &world, &observation).expect("verdict");
    assert!(!verdict.is_admitted);

    let wasi = build_manifest(ManifestInput {
        profile_id: fixture.profile.profile_id.clone(),
        wit_package: fixture.profile.wit.package.clone(),
        world: fixture.profile.wit.world.clone(),
        imports: vec!["wasi:filesystem/types@0.2.6".to_string()],
        exports: vec![COMPONENT_INVOKE_EXPORT.to_string()],
    })
    .expect("wasi manifest");
    let verdict = admit_declared(&wasi, &world, &observation).expect("verdict");
    assert!(!verdict.is_admitted);
    assert!(verdict.blockers.iter().any(|blocker| blocker.contains("ambient WASI")));

    let partial = build_manifest(ManifestInput {
        profile_id: fixture.profile.profile_id.clone(),
        wit_package: fixture.profile.wit.package.clone(),
        world: fixture.profile.wit.world.clone(),
        imports: Vec::new(),
        exports: Vec::new(),
    })
    .expect("partial manifest");
    let verdict = admit_declared(&partial, &world, &observation).expect("verdict");
    assert!(!verdict.is_admitted);
    assert!(verdict.blockers.iter().any(|blocker| blocker.contains("manifest exports")));
}

#[test]
fn undeclared_observed_import_and_tool_drift_fail_admission() {
    // r[verify aspen.wasm_import_admission.surface]
    // r[verify aspen.wasm_import_admission.admission]
    let fixture = super::support::ComponentFixture::new(ComponentConsumer::Actor);
    let world = declared_world(&fixture.profile);
    let mut undeclared = facts_observation(&fixture.bundle.component.content_ref, &fixture.facts);
    undeclared.imports.push("wasi:sockets/tcp@0.2.6".to_string());
    let verdict = admit_declared(&fixture.import_manifest, &world, &undeclared).expect("verdict");
    assert!(!verdict.is_admitted);
    assert!(verdict.blockers.iter().any(|blocker| blocker.contains("ambient WASI")));

    let mut stale_tool = facts_observation(&fixture.bundle.component.content_ref, &fixture.facts);
    stale_tool.extraction_tool = "wasmparser/0.100.0".to_string();
    let verdict = admit_declared(&fixture.import_manifest, &world, &stale_tool).expect("verdict");
    assert!(!verdict.is_admitted);
    assert!(verdict.blockers.iter().any(|blocker| blocker.contains("extraction tool")));
}

#[test]
fn missing_manifest_unsorted_surface_and_tampered_identity_fail_closed() {
    // r[verify aspen.wasm_import_admission.manifest]
    let fixture = super::support::ComponentFixture::new(ComponentConsumer::Actor);
    let component_ref = fixture.bundle.component.content_ref.clone();
    let missing =
        record_surface_admission(&fixture.profile, ManifestSource::Missing, &component_ref, &fixture.component_bytes);
    assert!(missing.is_err());

    let unsorted = build_manifest(ManifestInput {
        profile_id: fixture.profile.profile_id.clone(),
        wit_package: fixture.profile.wit.package.clone(),
        world: fixture.profile.wit.world.clone(),
        imports: vec!["b:first@1.0.0".to_string(), "a:second@1.0.0".to_string()],
        exports: Vec::new(),
    });
    assert!(unsorted.is_err());

    let mut tampered = fixture.import_manifest.clone();
    tampered.imports = vec!["molten:undeclared/import@1.0.0".to_string()];
    let world = declared_world(&fixture.profile);
    let observation = facts_observation(&fixture.bundle.component.content_ref, &fixture.facts);
    let verdict = admit_declared(&tampered, &world, &observation).expect("verdict");
    assert!(!verdict.is_admitted);
    assert!(verdict.blockers.iter().any(|blocker| blocker.contains("manifest identity")));
}

#[test]
fn overclaim_labels_and_oversized_receipt_payloads_are_rejected() {
    // r[verify aspen.wasm_import_admission.evidence]
    // r[verify aspen.wasm_import_admission.nonclaims]
    let fixture = super::support::ComponentFixture::new(ComponentConsumer::Actor);
    let record = fixture_record(&fixture);
    let mut overclaim_input = record.receipt.input.clone();
    overclaim_input.labels = vec!["sandbox-containment".to_string(), "component-correctness".to_string()];
    let overclaim_receipt = build_admission_evidence(overclaim_input).expect("labeled receipt");
    assert!(validate_admission_evidence(&overclaim_receipt).is_ok());
    assert!(validate_non_claims(&overclaim_receipt).is_err());
    assert!(validate_non_claims(&record.receipt).is_ok());

    let oversized = build_admission_evidence(AdmissionInput {
        manifest_ref: record.receipt.input.manifest_ref.clone(),
        wit_package: record.receipt.input.wit_package.clone(),
        world: record.receipt.input.world.clone(),
        wit_ref: record.receipt.input.wit_ref.clone(),
        import_set_ref: record.receipt.input.import_set_ref.clone(),
        extraction_tool: record.receipt.input.extraction_tool.clone(),
        verifier: record.receipt.input.verifier.clone(),
        is_admitted: true,
        blockers: vec!["0".repeat(300)],
        labels: Vec::new(),
    });
    assert!(oversized.is_err());

    let mut tampered_receipt = record.receipt.clone();
    tampered_receipt.receipt_ref = super::support::fixture_ref("tampered-receipt");
    assert!(validate_admission_evidence(&tampered_receipt).is_err());
}

#[test]
fn drifted_manifest_denies_execution_before_instantiation() {
    // r[verify aspen.wasm_import_admission.surface]
    // r[verify aspen.wasm_import_admission.fixtures]
    let mut fixture = super::support::ComponentFixture::new(ComponentConsumer::Actor);
    fixture.import_manifest.world = "drifting-world".to_string();
    let input = super::support::input_value();
    let outcome = execute_component(&fixture.request(&input));
    assert!(!outcome.is_pass());
    assert_eq!(outcome.receipts.len(), 1);
    assert_eq!(outcome.receipts[0].input.stage, ComponentReceiptStage::Denial);
    assert_eq!(outcome.receipts[0].input.trap_class.as_deref(), Some("component-admission-denial"));
    assert!(outcome.diagnostics.iter().any(|diagnostic| diagnostic.contains("declared WIT world")));
}
