
    #[test]
    fn control_live_peer_ticket_admission_gates_bootstrap() {
        let root = temp_dir("node-control-live-peer-ticket");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-ticket",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-ticket").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "live-ticket").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:ticket", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
        let authority_refs = test_live_authority_refs(&root, "peer:ticket", "node:live-ticket", "status", &policy_refs)
            .expect("authority grant ref");
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &authority_refs,
                policy_refs: &policy_refs,
                resource_refs: &resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        assert_peer_delivery(PeerDelivery {
            root: &root,
            request_value: &request_value,
            from_peer: "peer:ticket",
            to_node: "node:live-ticket",
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            is_expected_enqueued: true,
            expected_note: None,
        });
        assert_peer_delivery(PeerDelivery {
            root: &root,
            request_value: &request_value,
            from_peer: "peer:other-ticket",
            to_node: "node:live-ticket",
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            is_expected_enqueued: false,
            expected_note: Some("peer peer:ticket does not match peer:other-ticket"),
        });
    }

    struct ImportCase {
        sender: PathBuf,
        policy_refs: Vec<String>,
        ticket: ControlLiveTicket,
        admission: ControlLivePeerAdmission,
    }

    fn import_case() -> ImportCase {
        let receiver = temp_dir("node-control-live-import-receiver");
        let sender = temp_dir("node-control-live-import-sender");
        init_local(&InitInput {
            state_root: &receiver,
            node_id: "node:live-import",
        })
        .expect("init receiver");
        run_local(&RunInput { state_root: &receiver }).expect("run receiver");
        init_local(&InitInput {
            state_root: &sender,
            node_id: "node:live-import-sender",
        })
        .expect("init sender");
        let policy_refs = vec![local_ref("node-control-policy", "live-import").expect("policy ref")];
        let ticket = export_control_live_ticket(&ControlLiveTicketExportInput {
            state_root: &receiver,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("export ticket");
        let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: &receiver,
            ticket_value: &ticket.value,
            peer_id: "peer:live-import",
            sequence: 1,
            expires_at: Some(4),
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("admit peer");
        ImportCase {
            sender,
            policy_refs,
            ticket,
            admission,
        }
    }

    fn assert_ticket_imports(case: &ImportCase) {
        let imported_ticket = import_control_live_ticket(&ControlLiveTicketImportInput {
            state_root: &case.sender,
            ticket_value: &case.ticket.value,
            peer_admission_value: Some(&case.admission.value),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 2,
        })
        .expect("import ticket");
        assert_eq!(imported_ticket.decision, "pass");
        assert_eq!(imported_ticket.imported_refs.len(), 2);
        assert_eq!(
            crate::ledger::artifact_kind(&imported_ticket.receipt_value),
            "node-control-live-ticket-import-receipt"
        );
        read_ledger_artifact(&case.sender, &case.ticket.ticket_ref).expect("ticket imported");
        read_ledger_artifact(&case.sender, &case.admission.admission_ref).expect("admission imported");
        assert!(parse_control_live_ticket(&imported_ticket.receipt_value).is_err());
        assert!(parse_control_live_peer_admission(&imported_ticket.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&imported_ticket.receipt_value)
                .expect("ticket import receipt text")
                .contains("import-receipt-is-not-authority")
        );

        let stale_ticket = import_control_live_ticket(&ControlLiveTicketImportInput {
            state_root: &case.sender,
            ticket_value: &case.ticket.value,
            peer_admission_value: Some(&case.admission.value),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 8,
        })
        .expect("stale ticket import receipt");
        assert_eq!(stale_ticket.decision, "deny");
        assert!(stale_ticket.imported_refs.is_empty());
        assert!(stale_ticket.diagnostics.iter().any(|value| value.contains("expired at sequence")));

        let wrong_topic = import_control_live_ticket(&ControlLiveTicketImportInput {
            state_root: &case.sender,
            ticket_value: &case.ticket.value,
            peer_admission_value: Some(&case.admission.value),
            expected_node: Some("node:live-import"),
            expected_topic: Some("wrong-topic"),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 2,
        })
        .expect("wrong topic ticket import receipt");
        assert_eq!(wrong_topic.decision, "deny");
        assert!(wrong_topic.imported_refs.is_empty());
        assert!(wrong_topic.diagnostics.iter().any(|value| value.contains("wrong-topic")));

        let wrong_peer = evaluate_live_ticket_scope(LiveTicketScopeInput {
            ticket: &case.ticket,
            admission: Some(&case.admission),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:other-live-import"),
            as_of_sequence: 2,
            required_policy_refs: &case.policy_refs,
        });
        assert_eq!(wrong_peer.decision, "deny");
        assert!(wrong_peer.diagnostics.iter().any(|value| value.contains("peer:other-live-import")));

        let wrong_policy = vec![local_ref("node-control-policy", "wrong-ticket-policy").expect("wrong policy ref")];
        let policy_denied = evaluate_live_ticket_scope(LiveTicketScopeInput {
            ticket: &case.ticket,
            admission: Some(&case.admission),
            expected_node: Some("node:live-import"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&case.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-import"),
            as_of_sequence: 2,
            required_policy_refs: &wrong_policy,
        });
        assert_eq!(policy_denied.decision, "deny");
        assert!(policy_denied.diagnostics.iter().any(|value| value.contains("missing required policy")));
    }

    fn assert_grant_imports(case: &ImportCase) {
        let operations = vec!["status".to_string()];
        let grant_value = control_authority_grant_value(&ControlAuthorityGrantInput {
            peer_id: "peer:live-import",
            node_id: "node:live-import",
            operations: &operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(4),
            policy_refs: &case.policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("grant value");
        let imported_grant = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
            state_root: &case.sender,
            grant_value: &grant_value,
            expected_peer: Some("peer:live-import"),
            expected_node: Some("node:live-import"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        })
        .expect("import grant");
        assert_eq!(imported_grant.decision, "pass");
        assert_eq!(imported_grant.imported_refs.len(), 1);
        assert_eq!(
            crate::ledger::artifact_kind(&imported_grant.receipt_value),
            "node-control-authority-grant-import-receipt"
        );
        read_ledger_artifact(&case.sender, &imported_grant.grant_ref).expect("grant imported");
        assert!(parse_control_authority_grant(&imported_grant.receipt_value).is_err());
        assert!(
            crate::preserves_rail::to_text(&imported_grant.receipt_value)
                .expect("grant import receipt text")
                .contains("import-receipt-is-not-authority")
        );

        let bad_operations = vec!["shutdown".to_string()];
        let denied_grant = import_control_authority_grant_checked(&ControlAuthorityGrantImportInput {
            state_root: &case.sender,
            grant_value: &grant_value,
            expected_peer: Some("peer:live-import"),
            expected_node: Some("node:live-import"),
            expected_operations: &bad_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        })
        .expect("denied grant import");
        assert_eq!(denied_grant.decision, "deny");
        assert!(denied_grant.imported_refs.is_empty());
        assert!(denied_grant.diagnostics.iter().any(|value| value.contains("operation shutdown")));
    }

    #[test]
    fn control_live_ticket_and_authority_import_receipts_gate_bindings() {
        let case = import_case();
        assert_ticket_imports(&case);
        assert_grant_imports(&case);
    }

    struct FlowSeed {
        bundle_sender: PathBuf,
        operations: Vec<String>,
        ticket: ControlLiveTicket,
        admission: ControlLivePeerAdmission,
        authority_value: IoValue,
        receipt_values: Vec<IoValue>,
        authority_import_ref: String,
    }

    struct FlowCase {
        bundle_sender: PathBuf,
        operations: Vec<String>,
        ticket: ControlLiveTicket,
        admission: ControlLivePeerAdmission,
        authority_import_ref: String,
        exported: ControlLiveWorkflowBundleExport,
        verified: ControlLiveWorkflowBundleVerify,
        gated: ControlLiveWorkflowBundleGate,
    }

    struct FlowImports {
        receipt_values: Vec<IoValue>,
        authority_import_ref: String,
    }

    struct FlowApplyInput<'a> {
        state_root: &'a Path,
        receipt_value: Option<&'a IoValue>,
        request_value: Option<&'a IoValue>,
        is_send_requested: bool,
        sequence: u64,
        expect_message: &'a str,
    }

    fn init_flow_root(label: &str, node_id: &str) -> PathBuf {
        let root = temp_dir(label);
        init_local(&InitInput {
            state_root: &root,
            node_id,
        })
        .expect("init flow root");
        root
    }

    fn flow_roots() -> (PathBuf, PathBuf, PathBuf) {
        let receiver = init_flow_root("node-control-live-workflow-bundle-receiver", "node:live-bundle");
        run_local(&RunInput { state_root: &receiver }).expect("run receiver");
        let staging = init_flow_root("node-control-live-workflow-bundle-staging", "node:live-bundle-staging");
        let sender = init_flow_root("node-control-live-workflow-bundle-sender", "node:live-bundle-sender");
        (receiver, staging, sender)
    }

    fn flow_ticket(root: &Path, policy_refs: &[String]) -> ControlLiveTicket {
        export_control_live_ticket(&ControlLiveTicketExportInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs,
            evidence_refs: &[],
        })
        .expect("export ticket")
    }

    fn flow_admission(root: &Path, ticket: &ControlLiveTicket, policy_refs: &[String]) -> ControlLivePeerAdmission {
        admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: root,
            ticket_value: &ticket.value,
            peer_id: "peer:live-bundle",
            sequence: 1,
            expires_at: Some(8),
            policy_refs,
            evidence_refs: &[],
        })
        .expect("admit peer")
    }

    fn flow_authority_value(policy_refs: &[String], operations: &[String]) -> IoValue {
        control_authority_grant_value(&ControlAuthorityGrantInput {
            peer_id: "peer:live-bundle",
            node_id: "node:live-bundle",
            operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(8),
            policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("authority grant value")
    }
