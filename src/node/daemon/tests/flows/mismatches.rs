//! Live flow import and wrong-binding assertions.

pub(super) fn assert_flow_import_pass(case: &super::seeding::FlowCase) {
    let imported = super::super::intake::import_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleImportInput {
            state_root: &case.bundle_sender,
            bundle_value: &case.exported.bundle.bundle_value,
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
    .expect("import bundle");
    assert_eq!(imported.decision, "pass");
    assert!(imported.imported_refs.iter().any(|reference| reference == &case.exported.bundle.bundle_ref));
    super::super::storage::read_ledger_artifact(&case.bundle_sender, &case.ticket.ticket_ref)
        .expect("bundle imported ticket");
    super::super::storage::read_ledger_artifact(&case.bundle_sender, &case.admission.admission_ref)
        .expect("bundle imported admission");
    super::super::storage::read_ledger_artifact(&case.bundle_sender, &case.authority_import_ref)
        .expect("bundle imported authority");
    assert!(super::super::issuance::parse_control_authority_grant(&imported.receipt_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&imported.receipt_value)
            .expect("import receipt text")
            .contains("bundle-import-is-not-authority")
    );
}

pub(super) fn assert_flow_wrong_topic(
    case: &super::seeding::FlowCase,
) -> super::super::artifacts::ControlLiveWorkflowBundleGate {
    let root =
        super::seeding::init_flow_root("node-control-live-workflow-bundle-wrong-topic", "node:live-bundle-wrong-topic");
    let wrong_topic = super::super::intake::import_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some("wrong-topic"),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("wrong topic receipt");
    assert_eq!(wrong_topic.decision, "deny");
    assert!(wrong_topic.imported_refs.is_empty());
    assert!(wrong_topic.diagnostics.iter().any(|value| value.contains("wrong-topic")));
    assert!(super::super::storage::read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
    let wrong_verify = super::super::verifying::verify_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some("wrong-topic"),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("wrong topic verify receipt");
    assert_eq!(wrong_verify.decision, "deny");
    assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("wrong-topic")));
    let stale_gate = super::super::verifying::gate_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: Some(&wrong_verify.receipt_value),
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
    .expect("stale verify gate receipt");
    assert_eq!(stale_gate.decision, "deny");
    assert!(stale_gate.diagnostics.iter().any(|value| value.contains("does not match recomputed")));
    stale_gate
}

pub(super) fn assert_flow_stale_gate(
    case: &super::seeding::FlowCase,
    runtime: &tokio::runtime::Runtime,
    stale_gate: &super::super::artifacts::ControlLiveWorkflowBundleGate,
) {
    let root = super::seeding::init_flow_root(
        "node-control-live-workflow-bundle-apply-stale-gate",
        "node:live-bundle-apply-stale-gate",
    );
    let receipt = super::gatedenials::run_flow_apply(runtime, case, super::seeding::FlowApplyInput {
        state_root: &root,
        receipt_value: Some(&stale_gate.receipt_value),
        request_value: None,
        is_send_requested: false,
        sequence: 1,
        expect_message: "stale gate apply receipt",
    });
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.imported_refs.is_empty());
    assert!(receipt.diagnostics.iter().any(|value| value.contains("decision deny")));
    assert!(super::super::storage::read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
}

pub(super) fn assert_flow_wrong_peer(case: &super::seeding::FlowCase) {
    let root =
        super::seeding::init_flow_root("node-control-live-workflow-bundle-wrong-peer", "node:live-bundle-wrong-peer");
    let wrong_peer = super::super::intake::import_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("wrong peer receipt");
    assert_eq!(wrong_peer.decision, "deny");
    assert!(wrong_peer.imported_refs.is_empty());
    assert!(wrong_peer.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));
    let wrong_verify = super::super::verifying::verify_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("wrong peer verify receipt");
    assert_eq!(wrong_verify.decision, "deny");
    assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));
}

pub(super) fn assert_flow_wrong_operation(case: &super::seeding::FlowCase) {
    let root = super::seeding::init_flow_root(
        "node-control-live-workflow-bundle-wrong-operation",
        "node:live-bundle-wrong-operation",
    );
    let wrong_operations = vec!["shutdown".to_string()];
    let wrong_operation = super::super::intake::import_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &wrong_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("wrong operation receipt");
    assert_eq!(wrong_operation.decision, "deny");
    assert!(wrong_operation.imported_refs.is_empty());
    assert!(wrong_operation.diagnostics.iter().any(|value| value.contains("operation shutdown")));
    let wrong_verify = super::super::verifying::verify_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &wrong_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("wrong operation verify receipt");
    assert_eq!(wrong_verify.decision, "deny");
    assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("operation shutdown")));
}
