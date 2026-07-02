
    fn assert_ack_pass(
        case: &ReconcileCase,
        reconciled: &ControlLiveWorkflowBundleReconcile,
        wrong_envelope: &str,
    ) -> AckPass {
        let delivery = &case.delivery;
        let ack_export = export_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            reconcile_receipt_value: &reconciled.receipt_value,
        })
        .expect("ack export");
        assert_eq!(ack_export.decision, "pass");
        assert_eq!(ack_export.receiver_decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&ack_export.ack.ack_value), "node-control-live-workflow-bundle-ack");
        assert_eq!(
            crate::ledger::artifact_kind(&ack_export.receipt_value),
            "node-control-live-workflow-bundle-ack-export-receipt"
        );
        assert!(parse_control_authority_grant(&ack_export.ack.ack_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&ack_export.ack.ack_value)
                .expect("ack text")
                .contains("ack-bundle-is-not-authority")
        );
        let import_root = temp_dir("node-control-live-workflow-ack-import");
        init_local(&InitInput {
            state_root: &import_root,
            node_id: "node:ack-import",
        })
        .expect("init ack import root");
        let ack_import = import_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckImportInput {
            state_root: &import_root,
            ack_value: &ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("ack import");
        assert_eq!(ack_import.decision, "pass");
        assert!(ack_import.imported_refs.iter().any(|reference| reference == &ack_export.ack.ack_ref));
        assert_eq!(
            crate::ledger::artifact_kind(&ack_import.receipt_value),
            "node-control-live-workflow-bundle-ack-import-receipt"
        );
        assert!(
            crate::preserves_rail::to_text(&ack_import.receipt_value)
                .expect("ack import text")
                .contains("ack-import-is-not-authority")
        );
        read_ledger_artifact(&import_root, &ack_export.ack.ack_ref).expect("ack imported");
        read_ledger_artifact(&import_root, &reconciled.receipt_ref).expect("reconcile imported");
        assert_protocol_pass(case, reconciled, &ack_export.ack.ack_value);
        let wrong_ack_import = import_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckImportInput {
            state_root: &import_root,
            ack_value: &ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(wrong_envelope),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("wrong ack import");
        assert_eq!(wrong_ack_import.decision, "deny");
        assert!(wrong_ack_import.diagnostics.iter().any(|value| value.contains("does not match expected")));
        AckPass { import_root }
    }

    fn assert_protocol_pass(
        case: &ReconcileCase,
        reconciled: &ControlLiveWorkflowBundleReconcile,
        ack_value: &IoValue,
    ) {
        let delivery = &case.delivery;
        let protocol_gate = gate_control_live_workflow_protocol(&ControlLiveWorkflowProtocolGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            receipt_value: &case.gated.receipt_value,
            apply_receipt_value: &case.apply_receipt_value,
            reconcile_receipt_value: &reconciled.receipt_value,
            ack_value,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("workflow protocol gate");
        assert_eq!(protocol_gate.decision, "pass");
        assert_eq!(protocol_gate.operation_count, 6);
        assert_eq!(protocol_gate.message_count, 3);
        assert_eq!(crate::ledger::artifact_kind(&protocol_gate.receipt_value), "protocol-session-gate-receipt");
        assert!(parse_control_authority_grant(&protocol_gate.receipt_value).is_err());
    }

    fn assert_ack_denials(case: &ReconcileCase, denials: &ReconcileDenials, ack: &AckPass) {
        let delivery = &case.delivery;
        let missing_ack_export = export_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: None,
            queue_receipt_value: None,
            control_receipt_value: None,
            reconcile_receipt_value: &denials.missing_receiver.receipt_value,
        })
        .expect("missing ack export");
        assert_eq!(missing_ack_export.decision, "deny");
        assert!(
            missing_ack_export
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"))
        );

        let denied_ack_export = export_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckExportInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&denials.denied_control),
            reconcile_receipt_value: &denials.denied_reconcile.receipt_value,
        })
        .expect("denied ack export");
        assert_eq!(denied_ack_export.decision, "pass");
        assert_eq!(denied_ack_export.receiver_decision, "deny");
        assert!(
            denied_ack_export
                .ack
                .receiver_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("receiver denial propagated"))
        );
        let denied_ack_import = import_control_live_workflow_bundle_ack(&ControlLiveWorkflowBundleAckImportInput {
            state_root: &ack.import_root,
            ack_value: &denied_ack_export.ack.ack_value,
            expected_bundle_ref: Some(&case.exported.bundle.bundle_ref),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("denied ack import");
        assert_eq!(denied_ack_import.decision, "pass");
        assert_eq!(denied_ack_import.receiver_decision, "deny");
        let denied_protocol_gate = gate_control_live_workflow_protocol(&ControlLiveWorkflowProtocolGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            receipt_value: &case.gated.receipt_value,
            apply_receipt_value: &case.apply_receipt_value,
            reconcile_receipt_value: &denials.denied_reconcile.receipt_value,
            ack_value: &denied_ack_export.ack.ack_value,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("denied workflow protocol gate");
        assert_eq!(denied_protocol_gate.decision, "deny");
        assert!(
            denied_protocol_gate
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("ack receiver decision deny"))
        );
    }

    #[test]
    fn control_ingress_denies_missing_authority_before_enqueue() {
        let root = temp_dir("node-control-ingress-deny");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ingress-deny",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let request = status_request().expect("status request");
        let peer_bootstrap_refs = vec![local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];
        let policy_refs = vec![local_ref("node-control-policy", "ingress-deny").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "ingress-deny").expect("resource ref")];
        let envelope = control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request.value,
            from_peer: "peer:operator",
            to_node: "node:ingress-deny",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &[],
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("missing authority envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &root,
            envelope_value: &envelope.value,
        })
        .expect("publish denied ingress");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver denied ingress");
        assert!(!delivered.has_enqueued);
        let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains("authority refs missing"));
        assert!(next_pending_control_request(&root).expect("pending request scan").is_none());
    }

    struct PeerDelivery<'a> {
        root: &'a Path,
        request_value: &'a IoValue,
        from_peer: &'a str,
        to_node: &'a str,
        peer_bootstrap_refs: &'a [String],
        authority_refs: &'a [String],
        policy_refs: &'a [String],
        resource_refs: &'a [String],
        is_expected_enqueued: bool,
        expected_note: Option<&'a str>,
    }

    fn assert_peer_delivery(input: PeerDelivery<'_>) {
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: input.request_value,
            from_peer: input.from_peer,
            to_node: input.to_node,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: input.peer_bootstrap_refs,
            authority_refs: input.authority_refs,
            policy_refs: input.policy_refs,
            resource_refs: input.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: input.root,
            envelope_value: &envelope.value,
        })
        .expect("publish envelope");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: input.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver envelope");
        assert_eq!(delivered.has_enqueued, input.is_expected_enqueued);
        if let Some(expected_note) = input.expected_note {
            let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
            assert!(receipt_text.contains(expected_note));
        }
    }
