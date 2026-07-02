
    fn reconcile_delivery() -> ReconcileDelivery {
        let seed = reconcile_seed();
        let (request_value, request) = reconcile_request(&seed);
        let (envelope, delivered) = deliver_reconcile_envelope(&seed, &request_value);
        let (queue_value, control_value, control_receipt_ref) = dispatched_reconcile(&seed, &delivered);
        ReconcileDelivery {
            root: seed.root,
            request,
            envelope,
            delivered,
            queue_value,
            control_value,
            control_receipt_ref,
            policy_refs: seed.policy_refs,
            operations: vec!["status".to_string()],
        }
    }

    fn export_reconcile_bundle(delivery: &ReconcileDelivery) -> ControlLiveWorkflowBundleExport {
        let ticket = export_control_live_ticket(&ControlLiveTicketExportInput {
            state_root: &delivery.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &delivery.policy_refs,
            evidence_refs: &[],
        })
        .expect("export reconcile ticket");
        let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: &delivery.root,
            ticket_value: &ticket.value,
            peer_id: "peer:reconcile",
            sequence: 1,
            expires_at: None,
            policy_refs: &delivery.policy_refs,
            evidence_refs: &[],
        })
        .expect("admit reconcile peer");
        let authority_value = control_authority_grant_value(&ControlAuthorityGrantInput {
            peer_id: "peer:reconcile",
            node_id: "node:reconcile",
            operations: &delivery.operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: None,
            policy_refs: &delivery.policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("reconcile authority value");
        let receipt_values: Vec<&IoValue> = Vec::new();
        let exported = export_control_live_workflow_bundle(&ControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &ticket.value,
            peer_admission_value: &admission.value,
            authority_grant_value: &authority_value,
            receipt_values: &receipt_values,
        })
        .expect("export reconcile workflow bundle");
        assert_eq!(exported.decision, "pass");
        exported
    }

    fn gate_reconcile_bundle(
        exported: &ControlLiveWorkflowBundleExport,
        expected: &LiveWorkflowBundleExpectedInput<'_>,
    ) -> (ControlLiveWorkflowBundleVerify, ControlLiveWorkflowBundleGate) {
        let verified = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: expected.expected_node,
            expected_topic: expected.expected_topic,
            expected_endpoint: expected.expected_endpoint,
            expected_peer: expected.expected_peer,
            expected_operations: expected.expected_operations,
            expected_target_scope: expected.expected_target_scope,
            expected_resource_scope: expected.expected_resource_scope,
            as_of_sequence: expected.as_of_sequence,
            as_of_epoch: expected.as_of_epoch,
        })
        .expect("verify reconcile workflow bundle");
        assert_eq!(verified.decision, "pass");
        let gated = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&verified.receipt_value),
            require_verify_receipt: true,
            expected_node: expected.expected_node,
            expected_topic: expected.expected_topic,
            expected_endpoint: expected.expected_endpoint,
            expected_peer: expected.expected_peer,
            expected_operations: expected.expected_operations,
            expected_target_scope: expected.expected_target_scope,
            expected_resource_scope: expected.expected_resource_scope,
            as_of_sequence: expected.as_of_sequence,
            as_of_epoch: expected.as_of_epoch,
        })
        .expect("gate reconcile workflow bundle");
        assert_eq!(gated.decision, "pass");
        (verified, gated)
    }

    fn apply_reconcile_value(
        delivery: &ReconcileDelivery,
        exported: &ControlLiveWorkflowBundleExport,
        verified: &ControlLiveWorkflowBundleVerify,
        gated: &ControlLiveWorkflowBundleGate,
        expected: &LiveWorkflowBundleExpectedInput<'_>,
    ) -> IoValue {
        let imported_refs = Vec::new();
        let diagnostics = Vec::new();
        live_workflow_bundle_apply_receipt_value(&LiveWorkflowBundleApplyReceiptValueInput {
            decision: "pass",
            state_root: &delivery.root,
            bundle_ref: &exported.bundle.bundle_ref,
            gate_receipt_ref: Some(&gated.receipt_ref),
            recomputed_verify_receipt_ref: &verified.receipt_ref,
            import_receipt_ref: None,
            imported_refs: &imported_refs,
            mode: "dry-run",
            envelope_ref: Some(&delivery.envelope.envelope_ref),
            operation_ref: Some(&delivery.envelope.operation_ref),
            send_receipt_ref: None,
            expected,
            diagnostics: &diagnostics,
        })
        .expect("apply receipt")
    }

    fn reconcile_case() -> ReconcileCase {
        let delivery = reconcile_delivery();
        let expected = reconcile_expected(&delivery.operations);
        let exported = export_reconcile_bundle(&delivery);
        let (verified, gated) = gate_reconcile_bundle(&exported, &expected);
        let apply_receipt_value = apply_reconcile_value(&delivery, &exported, &verified, &gated, &expected);
        ReconcileCase {
            delivery,
            exported,
            gated,
            apply_receipt_value,
        }
    }

    fn assert_reconcile_pass(case: &ReconcileCase) -> ControlLiveWorkflowBundleReconcile {
        let delivery = &case.delivery;
        let reconciled = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("reconcile");
        assert_eq!(reconciled.decision, "pass");
        assert_eq!(
            crate::ledger::artifact_kind(&reconciled.receipt_value),
            "node-control-live-workflow-bundle-reconcile-receipt"
        );
        assert_eq!(reconciled.ingress_receipt_ref.as_deref(), Some(delivery.delivered.ingress_receipt_ref.as_str()));
        assert_eq!(reconciled.control_receipt_ref.as_deref(), Some(delivery.control_receipt_ref.as_str()));
        assert!(parse_control_authority_grant(&reconciled.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&reconciled.receipt_value)
                .expect("reconcile text")
                .contains("reconcile-receipt-is-not-authority")
        );
        import_artifact(&delivery.root, &reconciled.receipt_value).expect("import reconcile receipt");
        assert_reconcile_not_authority(case, &reconciled);
        reconciled
    }

    fn assert_reconcile_not_authority(case: &ReconcileCase, reconciled: &ControlLiveWorkflowBundleReconcile) {
        let refs = vec![reconciled.receipt_ref.clone()];
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &refs,
                policy_refs: &[],
                resource_refs: &[],
                evidence_refs: &[],
            })
            .expect("reconcile authority request");
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 2,
            peer_bootstrap_refs: &[],
            authority_refs: &refs,
            policy_refs: &[],
            resource_refs: &[],
            evidence_refs: &[],
        })
        .expect("reconcile authority envelope");
        let diagnostics = live_send_authority_grant_diagnostics(&case.delivery.root, &envelope)
            .expect("reconcile authority diagnostics");
        assert!(diagnostics.iter().any(|value| value.contains("is not a grant")));
        assert!(diagnostics.iter().any(|value| value.contains("authority delegation missing admitted grant")));
    }

    fn assert_reconcile_denials(case: &ReconcileCase) -> ReconcileDenials {
        let delivery = &case.delivery;
        let missing_receiver = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: None,
            queue_receipt_value: None,
            control_receipt_value: None,
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("missing receiver reconcile");
        assert_eq!(missing_receiver.decision, "deny");
        assert!(
            missing_receiver
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("requires receiver ingress receipt"))
        );

        let wrong_envelope = local_ref("node-control-envelope", "wrong-reconcile").expect("wrong envelope");
        let wrong_reconcile = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&delivery.control_value),
            expected_envelope_ref: Some(&wrong_envelope),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("wrong envelope reconcile");
        assert_eq!(wrong_reconcile.decision, "deny");
        assert!(wrong_reconcile.diagnostics.iter().any(|diagnostic| diagnostic.contains("does not match expected")));

        let denied_control = crate::node_runtime::control_deny_receipt_value(
            &delivery.request,
            &local_ref("node-startup", "reconcile-deny").expect("startup ref"),
            "receiver denial propagated",
        )
        .expect("denied control");
        let denied_reconcile = reconcile_control_live_workflow_bundle(&ControlLiveWorkflowBundleReconcileInput {
            apply_receipt_value: &case.apply_receipt_value,
            send_receipt_value: None,
            ingress_receipt_value: Some(&delivery.delivered.ingress_receipt_value),
            queue_receipt_value: Some(&delivery.queue_value),
            control_receipt_value: Some(&denied_control),
            expected_envelope_ref: Some(&delivery.envelope.envelope_ref),
            expected_operation_ref: Some(&delivery.envelope.operation_ref),
            expected_request_ref: Some(&delivery.delivered.request_ref),
        })
        .expect("denied reconcile");
        assert_eq!(denied_reconcile.decision, "deny");
        assert!(
            denied_reconcile
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.contains("receiver denial propagated"))
        );

        ReconcileDenials {
            missing_receiver,
            denied_control,
            denied_reconcile,
            wrong_envelope,
        }
    }
