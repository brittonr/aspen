
    fn flow_imports(
        staging: &Path,
        ticket: &ControlLiveTicket,
        admission: &ControlLivePeerAdmission,
        authority_value: &IoValue,
        operations: &[String],
    ) -> FlowImports {
        let ticket_import = import_control_live_ticket(&ControlLiveTicketImportInput {
            state_root: staging,
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            as_of_sequence: 2,
        })
        .expect("ticket import");
        let authority_import = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
            state_root: staging,
            grant_value: authority_value,
            expected_peer: Some("peer:live-bundle"),
            expected_node: Some("node:live-bundle"),
            expected_operations: operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        })
        .expect("authority import");
        FlowImports {
            receipt_values: vec![ticket_import.receipt_value, authority_import.receipt_value],
            authority_import_ref: authority_import.grant_ref,
        }
    }

    fn flow_seed() -> FlowSeed {
        let (receiver, staging, bundle_sender) = flow_roots();
        let policy_refs = vec![local_ref("node-control-policy", "live-bundle").expect("policy ref")];
        let operations = vec!["status".to_string()];
        let ticket = flow_ticket(&receiver, &policy_refs);
        let admission = flow_admission(&receiver, &ticket, &policy_refs);
        let authority_value = flow_authority_value(&policy_refs, &operations);
        let imports = flow_imports(&staging, &ticket, &admission, &authority_value, &operations);
        FlowSeed {
            bundle_sender,
            operations,
            ticket,
            admission,
            authority_value,
            receipt_values: imports.receipt_values,
            authority_import_ref: imports.authority_import_ref,
        }
    }

    fn export_flow(seed: &FlowSeed) -> ControlLiveWorkflowBundleExport {
        let receipt_values = seed.receipt_values.iter().collect::<Vec<_>>();
        export_control_live_workflow_bundle(&ControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &seed.ticket.value,
            peer_admission_value: &seed.admission.value,
            authority_grant_value: &seed.authority_value,
            receipt_values: &receipt_values,
        })
        .expect("export bundle")
    }

    fn verify_flow(seed: &FlowSeed, exported: &ControlLiveWorkflowBundleExport) -> ControlLiveWorkflowBundleVerify {
        verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&seed.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &seed.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("verify bundle")
    }

    fn gate_flow(
        seed: &FlowSeed,
        exported: &ControlLiveWorkflowBundleExport,
        verified: &ControlLiveWorkflowBundleVerify,
    ) -> ControlLiveWorkflowBundleGate {
        gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&verified.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&seed.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &seed.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        })
        .expect("gate bundle")
    }

    fn flow_case() -> FlowCase {
        let seed = flow_seed();
        let exported = export_flow(&seed);
        assert_eq!(exported.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&exported.bundle.bundle_value), "node-control-live-workflow-bundle");
        assert!(parse_control_authority_grant(&exported.bundle.bundle_value).is_err());
        let verified = verify_flow(&seed, &exported);
        assert_eq!(verified.decision, "pass");
        assert_eq!(
            crate::ledger::artifact_kind(&verified.receipt_value),
            "node-control-live-workflow-bundle-verify-receipt"
        );
        assert!(parse_control_authority_grant(&verified.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&verified.receipt_value)
                .expect("verify receipt text")
                .contains("verify-receipt-is-not-authority")
        );
        let gated = gate_flow(&seed, &exported, &verified);
        assert_eq!(gated.decision, "pass");
        assert_eq!(
            crate::ledger::artifact_kind(&gated.receipt_value),
            "node-control-live-workflow-bundle-gate-receipt"
        );
        assert_eq!(gated.verify_receipt_ref.as_deref(), Some(verified.receipt_ref.as_str()));
        assert!(parse_control_authority_grant(&gated.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&gated.receipt_value)
                .expect("gate receipt text")
                .contains("gate-receipt-is-not-authority")
        );
        FlowCase {
            bundle_sender: seed.bundle_sender,
            operations: seed.operations,
            ticket: seed.ticket,
            admission: seed.admission,
            authority_import_ref: seed.authority_import_ref,
            exported,
            verified,
            gated,
        }
    }

    fn assert_flow_gate_denials(case: &FlowCase) {
        let missing_verify_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: None,
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
        .expect("missing verify gate receipt");
        assert_eq!(missing_verify_gate.decision, "deny");
        assert!(
            missing_verify_gate
                .diagnostics
                .iter()
                .any(|value| value.contains("requires a current verify receipt"))
        );
        let malformed_verify_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &case.exported.bundle.bundle_value,
            verify_receipt_value: Some(&case.exported.bundle.bundle_value),
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
        .expect("malformed verify gate receipt");
        assert_eq!(malformed_verify_gate.decision, "deny");
        assert!(malformed_verify_gate.diagnostics.iter().any(|value| value.contains("verify receipt parse failed")));
    }

    fn run_flow_apply(
        runtime: &tokio::runtime::Runtime,
        case: &FlowCase,
        input: FlowApplyInput<'_>,
    ) -> ControlLiveWorkflowBundleApply {
        runtime
            .block_on(apply_control_live_workflow_bundle(&ControlLiveWorkflowBundleApplyInput {
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
                expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
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
                max_attempts: DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
                join_timeout_ms: 10_000,
            }))
            .expect(input.expect_message)
    }

    fn assert_flow_apply_pass(case: &FlowCase, runtime: &tokio::runtime::Runtime) -> ControlLiveWorkflowBundleApply {
        let applied = run_flow_apply(runtime, case, FlowApplyInput {
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
        assert!(parse_control_authority_grant(&applied.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&applied.receipt_value)
                .expect("apply receipt text")
                .contains("apply-receipt-is-not-authority")
        );
        read_ledger_artifact(&case.bundle_sender, &case.exported.bundle.bundle_ref).expect("apply imported bundle");
        applied
    }

    fn assert_flow_missing_gate(case: &FlowCase, runtime: &tokio::runtime::Runtime) {
        let root = init_flow_root(
            "node-control-live-workflow-bundle-apply-missing-gate",
            "node:live-bundle-apply-missing-gate",
        );
        let receipt = run_flow_apply(runtime, case, FlowApplyInput {
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
        assert!(read_ledger_artifact(&root, &case.exported.bundle.bundle_ref).is_err());
    }

    fn assert_flow_send_denial(case: &FlowCase, runtime: &tokio::runtime::Runtime) {
        let root = init_flow_root("node-control-live-workflow-bundle-apply-send", "node:live-bundle-apply-send");
        let authority_refs = vec![case.exported.bundle.authority_grant_ref.clone()];
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
            .expect("apply send request");
        let receipt = run_flow_apply(runtime, case, FlowApplyInput {
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
