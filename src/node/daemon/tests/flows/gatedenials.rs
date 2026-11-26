//! Live flow gate, apply, and send assertions.

pub(super) fn assert_flow_gate_denials(case: &super::seeding::FlowCase) {
    let missing_verify_gate = super::super::verifying::gate_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: None,
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
    .expect("missing verify gate receipt");
    assert_eq!(missing_verify_gate.decision, "deny");
    assert!(
        missing_verify_gate
            .diagnostics
            .iter()
            .any(|value| value.contains("requires a current verify receipt"))
    );
    let malformed_verify_gate = super::super::verifying::gate_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: Some(&case.exported.bundle.bundle_value),
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
    .expect("malformed verify gate receipt");
    assert_eq!(malformed_verify_gate.decision, "deny");
    assert!(malformed_verify_gate.diagnostics.iter().any(|value| value.contains("verify receipt parse failed")));
}

pub(super) fn run_flow_apply(
    runtime: &tokio::runtime::Runtime,
    case: &super::seeding::FlowCase,
    input: super::seeding::FlowApplyInput<'_>,
) -> super::super::artifacts::ControlLiveWorkflowBundleApply {
    runtime
        .block_on(super::super::applying::apply_control_live_workflow_bundle(
            &super::super::bundling::ControlLiveWorkflowBundleApplyInput {
                state_root: input.state_root,
                bundle_value: &case.exported.bundle.bundle_value,
                receipt_value: input.receipt_value,
                is_gate_receipt_required: true,
                request_value: input.request_value,
                should_send: input.is_send_requested,
                from_peer: None,
                sequence: input.sequence,
                expected_operation_ref: None,
                expected_node: Some("node:live-bundle"),
                expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
                expected_endpoint: Some(&case.ticket.live_endpoint_id),
                expected_peer: Some("peer:live-bundle"),
                expected_operations: &case.operations,
                expected_target_scope: Some("*"),
                expected_resource_scope: Some("*"),
                as_of_sequence: 2,
                as_of_epoch: 2,
                peer_bootstrap_refs: &[],
                authority_refs: &[],
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
                topology_profile: None,
                transport_profile: None,
                max_attempts: super::super::settings::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
                join_timeout_ms: 10_000,
            },
        ))
        .expect(input.expect_message)
}

pub(super) fn assert_flow_apply_pass(
    case: &super::seeding::FlowCase,
    runtime: &tokio::runtime::Runtime,
) -> super::super::artifacts::ControlLiveWorkflowBundleApply {
    let applied = run_flow_apply(runtime, case, super::seeding::FlowApplyInput {
        state_root: &case.bundle_sender,
        receipt_value: Some(&case.gated.receipt_value),
        request_value: None,
        is_send_requested: false,
        sequence: 1,
        expect_message: "apply bundle",
    });
    assert_eq!(applied.decision, "pass");
    assert_eq!(
        crate::ledger::artifact_kind(&applied.receipt_value),
        "node-control-live-workflow-bundle-apply-receipt"
    );
    assert!(applied.import_receipt_ref.is_some());
    assert!(applied.imported_refs.iter().any(|reference| reference == &case.exported.bundle.bundle_ref));
    assert!(super::super::issuance::parse_control_authority_grant(&applied.receipt_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&applied.receipt_value)
            .expect("apply receipt text")
            .contains("apply-receipt-is-not-authority")
    );
    super::super::storage::read_ledger_artifact(&case.bundle_sender, &case.exported.bundle.bundle_ref)
        .expect("apply imported bundle");
    applied
}

pub(super) fn assert_flow_missing_gate(case: &super::seeding::FlowCase, runtime: &tokio::runtime::Runtime) {
    let root = super::seeding::init_flow_root(
        "node-control-live-workflow-bundle-apply-missing-gate",
        "node:live-bundle-apply-missing-gate",
    );
    let receipt = run_flow_apply(runtime, case, super::seeding::FlowApplyInput {
        state_root: &root,
        receipt_value: None,
        request_value: None,
        is_send_requested: false,
        sequence: 1,
        expect_message: "missing gate apply receipt",
    });
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.imported_refs.is_empty());
    assert!(receipt.diagnostics.iter().any(|value| value.contains("requires a current gate receipt")));
    assert!(super::super::storage::read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
}

pub(super) fn assert_flow_send_denial(case: &super::seeding::FlowCase, runtime: &tokio::runtime::Runtime) {
    let root =
        super::seeding::init_flow_root("node-control-live-workflow-bundle-apply-send", "node:live-bundle-apply-send");
    let authority_refs = vec![case.exported.bundle.authority_grant_ref.clone()];
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "status",
        target_ref: None,
        payload_ref: None,
        authority_refs: &authority_refs,
        policy_refs: &[],
        resource_refs: &[],
        evidence_refs: &[],
    })
    .expect("apply send request");
    let receipt = run_flow_apply(runtime, case, super::seeding::FlowApplyInput {
        state_root: &root,
        receipt_value: Some(&case.gated.receipt_value),
        request_value: Some(&request_value),
        is_send_requested: true,
        sequence: 7,
        expect_message: "apply send receipt",
    });
    assert_eq!(receipt.decision, "deny");
    assert!(receipt.import_receipt_ref.is_some());
    assert!(receipt.send_receipt_ref.is_some());
    assert!(receipt.diagnostics.iter().any(|value| value.contains("no endpoint addresses")));
    assert!(receipt.send_receipt_value.is_some());
}
