
    fn assert_flow_malformed(case: &FlowCase) {
        let root = init_flow_root("node-control-live-workflow-bundle-malformed", "node:live-bundle-malformed");
        let malformed =
            crate::preserves_rail::record("node-control-live-workflow-bundle-v1", vec![crate::preserves_rail::string(
                crate::preserves_rail::NODE_CONTROL_LIVE_WORKFLOW_BUNDLE_SCHEMA,
            )]);
        assert!(
            import_control_live_workflow_bundle(&ControlLiveWorkflowBundleImportInput {
                state_root: &root,
                bundle_value: &malformed,
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
            .is_err()
        );
        let malformed_verify = verify_control_live_workflow_bundle(&ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &malformed,
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
        .expect("malformed verify receipt");
        assert_eq!(malformed_verify.decision, "deny");
        assert!(malformed_verify.diagnostics.iter().any(|value| value.contains("parse failed")));
        let malformed_gate = gate_control_live_workflow_bundle(&ControlLiveWorkflowBundleGateInput {
            bundle_value: &malformed,
            verify_receipt_value: Some(&malformed_verify.receipt_value),
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
        .expect("malformed gate receipt");
        assert_eq!(malformed_gate.decision, "deny");
        assert!(malformed_gate.diagnostics.iter().any(|value| value.contains("parse failed")));
    }

    #[test]
    fn control_live_workflow_bundle_import_export_gates_bindings() {
        let case = flow_case();
        assert_flow_gate_denials(&case);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("apply runtime");
        let applied = assert_flow_apply_pass(&case, &runtime);
        assert_flow_missing_gate(&case, &runtime);
        assert_flow_send_denial(&case, &runtime);
        assert_flow_import_pass(&case);
        let stale_gate = assert_flow_wrong_topic(&case);
        assert_flow_stale_gate(&case, &runtime, &stale_gate);
        assert_flow_wrong_peer(&case);
        assert_flow_wrong_operation(&case);
        assert_flow_wrong_grant(&case);
        assert_flow_receipts_not_grants(&case, &applied);
        assert_flow_malformed(&case);
    }

    fn assert_sent(sent: &ControlLiveSend) {
        assert_eq!(crate::ledger::artifact_kind(&sent.send_receipt_value), "node-control-live-send-receipt");
        assert!(sent.transport_receipt_ref.is_some());
        assert_eq!(
            sent.operation_ref,
            parse_control_ingress_envelope(&sent.envelope_value).expect("envelope").operation_ref
        );
    }

    fn assert_duplicate(first: &ControlLiveSend, duplicate: &ControlLiveSend) {
        assert_eq!(duplicate.send_receipt_ref, first.send_receipt_ref);
        assert!(duplicate.duplicate_receipt_ref.is_some());
        assert_eq!(
            crate::ledger::artifact_kind(duplicate.duplicate_receipt_value.as_ref().expect("duplicate receipt")),
            "node-control-live-send-duplicate-receipt"
        );
    }

    struct ServedCase<'a> {
        root: &'a std::path::Path,
        authority_ref: &'a str,
        ticket_value: &'a IoValue,
        admission_value: &'a IoValue,
        send_receipt_value: &'a IoValue,
        listener: &'a ControlLiveServe,
    }

    fn assert_served_case(case: ServedCase<'_>) {
        assert_eq!(case.listener.service.decision, "pass");
        assert_eq!(case.listener.service.processed_request_refs.len(), 1);
        assert_eq!(case.listener.transport_receipt_refs.len(), 1);
        assert!(case.listener.observed_events > 0);
        let authority_value = read_ledger_artifact(case.root, case.authority_ref).expect("authority value");
        let receive_values = case
            .listener
            .transport_receipt_refs
            .iter()
            .map(|reference| read_ledger_artifact(case.root, reference).expect("receive receipt value"))
            .collect::<Vec<_>>();
        let receive_value_refs = receive_values.iter().collect::<Vec<_>>();
        let workflow = control_live_workflow_receipt(&ControlLiveWorkflowInput {
            state_root: Some(case.root),
            receiver_ticket_value: case.ticket_value,
            peer_admission_value: case.admission_value,
            authority_grant_value: &authority_value,
            send_receipt_value: case.send_receipt_value,
            receive_receipt_values: &receive_value_refs,
            listener_receipt_value: Some(&case.listener.listener_receipt_value),
            service_receipt_value: &case.listener.service.service_receipt_value,
        })
        .expect("workflow receipt");
        assert_eq!(workflow.decision, "pass");
        assert_eq!(crate::ledger::artifact_kind(&workflow.receipt_value), "node-control-live-workflow-receipt");
    }

    struct SendMaterial {
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
        admission: ControlLivePeerAdmission,
        request_value: IoValue,
    }

    fn init_send_case() -> (std::path::PathBuf, crate::node_identity::Identity) {
        let root = temp_dir("node-control-live-send");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-send",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let state_root = crate::node_state::NodeStateRoot::open(&root).expect("open node state root");
        let identity = crate::node_identity::parse_identity(
            &read_preserves(
                &state_root,
                &crate::node_state::NodeStatePath::parse(IDENTITY_FILE).expect("identity path"),
            )
            .expect("identity"),
        )
        .expect("parse identity");
        (root, identity)
    }

    fn send_material(root: &std::path::Path, ticket: &ControlLiveTicket) -> SendMaterial {
        let policy_refs = vec![local_ref("node-control-policy", "live-send").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "live-send").expect("resource ref")];
        let admission = admit_control_live_peer(&ControlLivePeerAdmitInput {
            state_root: root,
            ticket_value: &ticket.value,
            peer_id: "peer:external-send",
            sequence: 1,
            expires_at: None,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("peer admission");
        let peer_bootstrap_refs = vec![admission.admission_ref.clone()];
        let authority_refs =
            test_live_authority_refs(root, "peer:external-send", "node:live-send", "status", &policy_refs)
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
        SendMaterial {
            policy_refs,
            resource_refs,
            peer_bootstrap_refs,
            authority_refs,
            admission,
            request_value,
        }
    }

    fn build_send_input<'a>(
        root: &'a std::path::Path,
        ticket: &'a ControlLiveTicket,
        material: &'a SendMaterial,
    ) -> ControlLiveSendInput<'a> {
        ControlLiveSendInput {
            state_root: Some(root),
            request_value: &material.request_value,
            receiver_ticket_value: &ticket.value,
            from_peer: "peer:external-send",
            sequence: 1,
            expected_operation_ref: None,
            expected_receiver_node: None,
            expected_topic: None,
            expected_endpoint: None,
            topology_profile: None,
            transport_profile: None,
            max_attempts: DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
            peer_bootstrap_refs: &material.peer_bootstrap_refs,
            authority_refs: &material.authority_refs,
            policy_refs: &material.policy_refs,
            resource_refs: &material.resource_refs,
            evidence_refs: &[],
            join_timeout_ms: 10_000,
        }
    }

    fn build_listener_input(root: &std::path::Path) -> ControlLiveServeInput<'_> {
        ControlLiveServeInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            max_events: 8,
            event_timeout_ms: 1_000,
            max_requests_per_tick: 1,
            supervisor_policy_value: None,
        }
    }

    #[test]
    fn send_reaches_bounded_listener() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        runtime.block_on(async {
            let (root, identity) = init_send_case();
            let lookup = iroh::address_lookup::memory::MemoryLookup::new();
            let receiver_endpoint = live_gossip_endpoint(&lookup, Some(stable_live_endpoint_secret(&identity)))
                .await
                .expect("receiver endpoint");
            let receiver_addr = receiver_endpoint.addr();
            let state_root = crate::node_state::NodeStateRoot::open(&root).expect("open node state root");
            let live_ticket =
                live_ticket_for_bound_endpoint(&state_root, &identity, DEFAULT_CONTROL_INGRESS_TOPIC, &receiver_addr)
                    .expect("live ticket");
            lookup.add_endpoint_info(receiver_addr);
            let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
            let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
                .accept(iroh_gossip::ALPN, receiver_gossip.clone())
                .spawn();
            let mut receiver_topic = receiver_gossip
                .subscribe(control_live_topic_id(DEFAULT_CONTROL_INGRESS_TOPIC), Vec::new())
                .await
                .expect("receiver subscribe");
            let material = send_material(&root, &live_ticket);
            let send_input = build_send_input(&root, &live_ticket, &material);
            let sent = send_control_live_ingress(&send_input).await.expect("live send");
            assert_sent(&sent);
            let duplicate = send_control_live_ingress(&send_input).await.expect("duplicate live send");
            assert_duplicate(&sent, &duplicate);
            let listener_input = build_listener_input(&root);
            let listener = serve_node_control_live_listener_with_topic(
                &state_root,
                &listener_input,
                &mut receiver_topic,
                &identity.node_id,
                &identity.endpoint_id,
                &live_ticket.live_endpoint_id,
            )
            .await
            .expect("listener drain");
            receiver_router.shutdown().await.expect("receiver shutdown");
            assert_served_case(ServedCase {
                root: &root,
                authority_ref: &material.authority_refs[0],
                ticket_value: &live_ticket.value,
                admission_value: &material.admission.value,
                send_receipt_value: &sent.send_receipt_value,
                listener: &listener,
            });
        });
    }

    struct DenyCase<'a> {
        name: &'a str,
        grant_peer: Option<&'a str>,
        grant_node: &'a str,
        grant_operations: &'a [&'a str],
        target_ref: Option<&'a str>,
        target_scope: &'a str,
        resource_scope: &'a str,
        epoch: u64,
        expires_at: Option<u64>,
        is_revoked: bool,
        sequence: u64,
        expected: &'a str,
    }

    struct DenyCaseRefs {
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
    }
