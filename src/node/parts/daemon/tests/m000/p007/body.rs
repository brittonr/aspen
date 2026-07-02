
    fn assert_flow_import_pass(case: &FlowCase) {
        let imported = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
            state_root: &case.bundle_sender,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("import bundle");
        assert_eq!(imported.decision, "pass");
        assert!(imported.imported_refs.iter().any(|reference| reference == &case.exported.bundle.bundle_ref));
        read_ledger_artifact(&case.bundle_sender, &case.ticket.ticket_ref).expect("bundle imported ticket");
        read_ledger_artifact(&case.bundle_sender, &case.admission.admission_ref).expect("bundle imported admission");
        read_ledger_artifact(&case.bundle_sender, &case.authority_import_ref).expect("bundle imported authority");
        assert!(parse_control_authority_grant(&imported.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&imported.receipt_value)
                .expect("import receipt text")
                .contains("bundle-import-is-not-authority")
        );
    }

    fn assert_flow_wrong_topic(case: &FlowCase) -> ControlLiveWorkflowBundleGate {
        let root = init_flow_root("node-control-live-workflow-bundle-wrong-topic", "node:live-bundle-wrong-topic");
        let wrong_topic = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
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
        })
        .expect("wrong topic receipt");
        assert_eq!(wrong_topic.decision, "deny");
        assert!(wrong_topic.imported_refs.is_empty());
        assert!(wrong_topic.diagnostics.iter().any(|value| value.contains("wrong-topic")));
        assert!(read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
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
        })
        .expect("wrong topic verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("wrong-topic")));
        let stale_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: Some(&wrong_verify.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("stale verify gate receipt");
        assert_eq!(stale_gate.decision, "deny");
        assert!(stale_gate.diagnostics.iter().any(|value| value.contains("does not match recomputed")));
        stale_gate
    }

    fn assert_flow_stale_gate(
        case: &FlowCase,
        runtime: &tokio::runtime::Runtime,
        stale_gate: &ControlLiveWorkflowBundleGate,
    ) {
        let root =
            init_flow_root("node-control-live-workflow-bundle-apply-stale-gate", "node:live-bundle-apply-stale-gate");
        let receipt = run_flow_apply(runtime, case, FlowApplyInput {
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
        assert!(read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
    }

    fn assert_flow_wrong_peer(case: &FlowCase) {
        let root = init_flow_root("node-control-live-workflow-bundle-wrong-peer", "node:live-bundle-wrong-peer");
        let wrong_peer = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong peer receipt");
        assert_eq!(wrong_peer.decision, "deny");
        assert!(wrong_peer.imported_refs.is_empty());
        assert!(wrong_peer.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong peer verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("peer:other-live-bundle")));
    }

    fn assert_flow_wrong_operation(case: &FlowCase) {
        let root =
            init_flow_root("node-control-live-workflow-bundle-wrong-operation", "node:live-bundle-wrong-operation");
        let wrong_operations = vec!["shutdown".to_string()];
        let wrong_operation = import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
            state_root: &root,
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &wrong_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong operation receipt");
        assert_eq!(wrong_operation.decision, "deny");
        assert!(wrong_operation.imported_refs.is_empty());
        assert!(wrong_operation.diagnostics.iter().any(|value| value.contains("operation shutdown")));
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &case.exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &wrong_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong operation verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("operation shutdown")));
    }

    fn assert_flow_wrong_grant(case: &FlowCase) {
        let wrong_grant_ref = local_ref("authority-grant", "wrong-live-bundle").expect("wrong grant ref");
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
            crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(Vec::<IoValue>::new())]),
        ]);
        let wrong_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &wrong_bundle,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &case.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("wrong grant verify receipt");
        assert_eq!(wrong_verify.decision, "deny");
        assert!(wrong_verify.diagnostics.iter().any(|value| value.contains("authority grant ref mismatch")));
    }

    fn assert_ref_not_grant(root: &Path, authority_ref: &str, sequence: u64) {
        let authority_refs = vec![authority_ref.to_string()];
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("authority request");
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:live-bundle",
            to_node: "node:live-bundle",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence,
            peer_bootstrap_refs: &[],
            authority_refs: &authority_refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("authority envelope");
        let diagnostics = live_send_authority_grant_diagnostics(root, &envelope).expect("authority diagnostics");
        assert!(diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(diagnostics.iter().any(|value| value.contains("authority delegation missing admitted grant")));
    }

    fn assert_flow_receipts_not_grants(case: &FlowCase, applied: &ControlLiveWorkflowBundleApply) {
        import_artifact(&case.bundle_sender, &case.verified.receipt_value).expect("import verify receipt");
        assert_ref_not_grant(&case.bundle_sender, &case.verified.receipt_ref, 3);
        import_artifact(&case.bundle_sender, &case.gated.receipt_value).expect("import gate receipt");
        assert_ref_not_grant(&case.bundle_sender, &case.gated.receipt_ref, 4);
        assert_ref_not_grant(&case.bundle_sender, &applied.receipt_ref, 5);
    }
