//! Reproducible provenance build-verification tests.

#[test]
fn control_reproducible_provenance_requires_build_verification_binding() {
    let root = super::fixtures::temp_dir("node-control-reproducible-provenance");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:reproducible-provenance",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");

    let case = build_case(&root);
    assert_install_passes(&root, &case);
}

struct BuildMaterial {
    authority_refs: Vec<String>,
    policy_refs: Vec<String>,
    resource_refs: Vec<String>,
    payload_ref: String,
    source_refs: Vec<String>,
    toolchain_refs: Vec<String>,
    dependency_ref: String,
    builder_ref: String,
}

struct BuildCase {
    material: BuildMaterial,
    evidence_refs: Vec<String>,
}

fn build_case(root: &std::path::Path) -> BuildCase {
    let material = build_material(root);
    let evidence_refs = verified_refs(root, &material);
    BuildCase {
        material,
        evidence_refs,
    }
}

fn build_material(root: &std::path::Path) -> BuildMaterial {
    let payload_value =
        crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
            "reproducible-provenance",
        )]);
    BuildMaterial {
        authority_refs: vec![
            super::super::storage::local_ref("node-control-authority", "reproducible").expect("authority ref"),
        ],
        policy_refs: vec![super::super::storage::local_ref("node-control-policy", "reproducible").expect("policy ref")],
        resource_refs: vec![
            super::super::storage::local_ref("node-control-resource", "reproducible").expect("resource ref"),
        ],
        payload_ref: super::super::storage::import_artifact(root, &payload_value).expect("import payload"),
        source_refs: vec![super::super::storage::local_ref("node-control-source", "reproducible").expect("source ref")],
        toolchain_refs: vec![
            super::super::storage::local_ref("node-control-toolchain", "reproducible").expect("toolchain ref"),
        ],
        dependency_ref: super::super::storage::local_ref("node-control-deps", "reproducible").expect("deps ref"),
        builder_ref: super::super::storage::local_ref("node-control-builder", "reproducible").expect("builder ref"),
    }
}

fn build_record_for(material: &BuildMaterial) -> preserves::IOValue {
    crate::provenance::build_record_value(&crate::provenance::BuildRecordInput {
        expected_artifact_ref: &material.payload_ref,
        source_refs: &material.source_refs,
        dependency_closure_ref: &material.dependency_ref,
        toolchain_refs: &material.toolchain_refs,
        build_params: &[],
        builder_ref: &material.builder_ref,
        nix_derivation_refs: &[],
        policy_refs: &material.policy_refs,
        evidence_refs: &[],
    })
    .expect("build record")
}

fn provenance_record_for(material: &BuildMaterial, build_record_refs: &[String]) -> preserves::IOValue {
    crate::provenance::record_value(&crate::provenance::RecordInput {
        artifact_ref: &material.payload_ref,
        trust_state: crate::provenance::TRUST_STATE_REPRODUCIBLE_VERIFIED,
        source_refs: &material.source_refs,
        dependency_closure_ref: &material.dependency_ref,
        toolchain_refs: &material.toolchain_refs,
        builder_ref: &material.builder_ref,
        review_refs: &[],
        test_refs: &[],
        source_gate_refs: &[],
        policy_refs: &material.policy_refs,
        build_record_refs,
    })
    .expect("reproducible provenance")
}

fn verified_refs(root: &std::path::Path, material: &BuildMaterial) -> Vec<String> {
    let build_record = build_record_for(material);
    let build_record_ref = super::super::storage::import_artifact(root, &build_record).expect("import build record");
    let build_verification = crate::provenance::verify_build(&crate::provenance::BuildVerificationInput {
        build_record_value: &build_record,
        actual_artifact_ref: &material.payload_ref,
        prior_diagnostics: &[],
    })
    .expect("verify build");
    let build_verification_ref = super::super::storage::import_artifact(root, &build_verification.receipt_value)
        .expect("import build verification");
    let build_record_refs = vec![build_record_ref];
    let provenance_record = provenance_record_for(material, &build_record_refs);
    let provenance_ref = super::super::storage::import_artifact(root, &provenance_record).expect("import provenance");
    vec![provenance_ref, build_verification_ref]
}

fn install_request_for(case: &BuildCase) -> preserves::IOValue {
    crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "install",
        target_ref: None,
        payload_ref: Some(&case.material.payload_ref),
        authority_refs: &case.material.authority_refs,
        policy_refs: &case.material.policy_refs,
        resource_refs: &case.material.resource_refs,
        evidence_refs: &case.evidence_refs,
    })
    .expect("reproducible install request")
}

fn assert_install_passes(root: &std::path::Path, case: &BuildCase) {
    let request = install_request_for(case);
    let submitted = super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: root,
        request_value: &request,
    })
    .expect("submit reproducible request");
    let dispatch =
        super::super::submission::dispatch_control_request_entry(&super::super::requests::ControlDispatchEntryInput {
            state_root: root,
            request_entry: Some(&submitted.inbox_entry),
        })
        .expect("dispatch reproducible request");
    let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value)
        .expect("reproducible control receipt");
    assert_eq!(receipt.decision, "pass");
    assert!(receipt.diagnostics.is_empty());
    assert!(
        !crate::artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
            .expect("list registry")
            .is_empty()
    );
}
