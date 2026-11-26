//! Control provenance gate tests.

#[test]
fn control_provenance_gate_denies_missing_and_tampered_evidence_before_side_effects() {
    let root = super::mailbox::initialized_control_root("node-control-provenance", "node:provenance");
    let refs = case_refs("provenance");

    assert_missing_case(&root, &refs);
    assert_queued_case(&root, &refs);
    assert_tampered_case(&root, &refs);
}

struct CaseRefs {
    authority_refs: Vec<String>,
    policy_refs: Vec<String>,
    resource_refs: Vec<String>,
}

fn case_refs(label: &str) -> CaseRefs {
    CaseRefs {
        authority_refs: vec![super::super::storage::local_ref("node-control-authority", label).expect("authority ref")],
        policy_refs: vec![super::super::storage::local_ref("node-control-policy", label).expect("policy ref")],
        resource_refs: vec![super::super::storage::local_ref("node-control-resource", label).expect("resource ref")],
    }
}

fn request_value(payload_ref: &str, refs: &CaseRefs, evidence_refs: &[String]) -> preserves::IOValue {
    crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "install",
        target_ref: None,
        payload_ref: Some(payload_ref),
        authority_refs: &refs.authority_refs,
        policy_refs: &refs.policy_refs,
        resource_refs: &refs.resource_refs,
        evidence_refs,
    })
    .expect("install request")
}

fn assert_registry_empty(root: &std::path::Path) {
    assert!(
        crate::artifacts::list_artifacts(&root.join("registry"), Some("node-control-artifact"))
            .expect("list registry")
            .is_empty()
    );
}

fn assert_missing_case(root: &std::path::Path, refs: &CaseRefs) {
    let payload_value =
        crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
            "missing-provenance",
        )]);
    let payload_ref = super::super::storage::import_artifact(root, &payload_value).expect("import payload");
    let request = request_value(&payload_ref, refs, &[]);
    let dispatch = super::mailbox::submit_and_dispatch(root, &request);
    let receipt = crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("control receipt");
    assert_eq!(receipt.decision, "deny");
    assert!(
        receipt
            .subreceipt_refs
            .iter()
            .any(|reference| crate::preserves_rail::validate_content_ref(reference).is_ok())
    );
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
    assert_registry_empty(root);
}

fn assert_queued_case(root: &std::path::Path, refs: &CaseRefs) {
    let payload = crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
        "queued-missing-provenance",
    )]);
    let payload_ref = super::super::storage::import_artifact(root, &payload).expect("import queued payload");
    let request = request_value(&payload_ref, refs, &[]);
    let queued = crate::node_runtime::parse_control_request(&request).expect("queued request parse");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: root,
        request_value: &request,
    })
    .expect("submit queued missing provenance");
    let loop_result = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: root,
        max_requests: 1,
    })
    .expect("process queued missing provenance");
    assert_eq!(loop_result.processed_request_refs, vec![queued.request_ref.clone()]);
    let state_root = crate::node_state::DirectoryRoot::open(root).expect("open node state root");
    let value = super::super::storage::read_preserves(
        &state_root,
        &super::super::paths::control_outbox_receipt_path(&queued.request_ref).expect("outbox receipt path"),
    )
    .expect("queued receipt value");
    let receipt = crate::node_runtime::parse_control_receipt(&value).expect("queued receipt");
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing provenance evidence")));
}

fn assert_tampered_case(root: &std::path::Path, refs: &CaseRefs) {
    let payload = crate::preserves_rail::record("node-control-install-payload", vec![crate::preserves_rail::string(
        "tampered-provenance",
    )]);
    let payload_ref = super::super::storage::import_artifact(root, &payload).expect("import tampered payload");
    let wrong_artifact_ref =
        super::super::storage::local_ref("node-control-wrong-provenance-artifact", "tampered").expect("wrong ref");
    let wrong_provenance = crate::provenance::synthetic_reviewed_record(&wrong_artifact_ref).expect("wrong provenance");
    let wrong_ref = super::super::storage::import_artifact(root, &wrong_provenance).expect("import wrong provenance");
    let evidence_refs = vec![wrong_ref];
    let request = request_value(&payload_ref, refs, &evidence_refs);
    let dispatch = super::mailbox::submit_and_dispatch(root, &request);
    let receipt =
        crate::node_runtime::parse_control_receipt(&dispatch.control_receipt_value).expect("tampered receipt");
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.diagnostics.iter().any(|diagnostic| diagnostic.contains("no provenance record matches")));
    assert_registry_empty(root);
}
