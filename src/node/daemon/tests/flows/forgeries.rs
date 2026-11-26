//! Live flow grant forgery and malformed input assertions.

pub(super) fn assert_flow_wrong_grant(case: &super::seeding::FlowCase) {
    let wrong_grant_ref =
        super::super::storage::local_ref("authority-grant", "wrong-live-bundle").expect("wrong grant ref");
    let wrong_bundle = crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA),
        crate::preserves_rail::record("ticket", vec![case.exported.bundle.ticket_value.clone()]),
        crate::preserves_rail::record("peer-admission", vec![case.exported.bundle.peer_admission_value.clone()]),
        crate::preserves_rail::record("authority-grant", vec![case.exported.bundle.authority_grant_value.clone()]),
        crate::preserves_rail::record("receipts", vec![crate::preserves_rail::sequence(
            case.exported.bundle.receipt_values.clone(),
        )]),
        crate::preserves_rail::record("ticket-ref", vec![crate::preserves_rail::string(
            &case.exported.bundle.ticket_ref,
        )]),
        crate::preserves_rail::record("peer-admission-ref", vec![crate::preserves_rail::string(
            &case.exported.bundle.peer_admission_ref,
        )]),
        crate::preserves_rail::record("authority-grant-ref", vec![crate::preserves_rail::string(&wrong_grant_ref)]),
        crate::preserves_rail::record("receipt-refs", vec![crate::preserves_rail::sequence(
            case.exported.bundle.receipt_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![
            crate::preserves_rail::sequence(Vec::<preserves::IOValue>::new()),
        ]),
    ]);
    let wrong_verify = super::super::verifying::verify_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &wrong_bundle,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("wrong grant verify receipt");
    assert_eq!(wrong_verify.decision, "deny");
    assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("authority grant ref mismatch")));
}

fn assert_ref_not_grant(root: &std::path::Path, authority_ref: &str, sequence: u64) {
    let authority_refs = vec![authority_ref.to_string()];
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "status",
        target_ref: None,
        payload_ref: None,
        authority_refs: &authority_refs,
        policy_refs: &[],
        resource_refs: &[],
        evidence_refs: &[],
    })
    .expect("authority request");
    let envelope =
        super::super::ingresses::control_live_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:live-bundle",
            to_node: "node:live-bundle",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence,
            peer_bootstrap_refs: &[],
            authority_refs: &authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("authority envelope");
    let state_root = crate::node_state::DirectoryRoot::open(root).expect("open node state root");
    let diagnostics = super::super::admissibility::live_send_authority_grant_diagnostics(&state_root, &envelope)
        .expect("authority diagnostics");
    assert!(diagnostics.iter().any(|value| value.contains("is not a grant")));
    assert!(diagnostics.iter().any(|value| value.contains("authority delegation missing admitted grant")));
}

pub(super) fn assert_flow_receipts_not_grants(
    case: &super::seeding::FlowCase,
    applied: &super::super::artifacts::ControlLiveWorkflowBundleApply,
) {
    super::super::storage::import_artifact(&case.bundle_sender, &case.verified.receipt_value)
        .expect("import verify receipt");
    assert_ref_not_grant(&case.bundle_sender, &case.verified.receipt_ref, 3);
    super::super::storage::import_artifact(&case.bundle_sender, &case.gated.receipt_value)
        .expect("import gate receipt");
    assert_ref_not_grant(&case.bundle_sender, &case.gated.receipt_ref, 4);
    assert_ref_not_grant(&case.bundle_sender, &applied.receipt_ref, 5);
}

pub(super) fn assert_flow_malformed(case: &super::seeding::FlowCase) {
    let root =
        super::seeding::init_flow_root("node-control-live-workflow-bundle-malformed", "node:live-bundle-malformed");
    let malformed =
        crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![crate::preserves_rail::string(
            crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
        )]);
    assert!(
        super::super::intake::import_control_live_workflow_bundle(
            &super::super::bundling::ControlLiveWorkflowBundleImportInput {
                state_root: &root,
                bundle_value: &malformed,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&case.ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &case.operations,
                expected_target_scope: Some("*"),
                expected_resource_scope: Some("*"),
                as_of_sequence: 2,
                as_of_epoch: 2,
            }
        )
        .is_err()
    );
    let malformed_verify = super::super::verifying::verify_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &malformed,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("malformed verify receipt");
    assert_eq!(malformed_verify.decision, "deny");
    assert!(malformed_verify.diagnostics.iter().any(|value| value.contains("parse failed")));
    let malformed_gate = super::super::verifying::gate_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleGateInput {
            bundle_value: &malformed,
            verify_receipt_value: Some(&malformed_verify.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("malformed gate receipt");
    assert_eq!(malformed_gate.decision, "deny");
    assert!(malformed_gate.diagnostics.iter().any(|value| value.contains("parse failed")));
}
