
    impl TwoNodeLive {
        async fn shutdown(self) {
            self.peer_live.router.shutdown().await.expect("peer router shutdown");
            self.requester_live.router.shutdown().await.expect("requester router shutdown");
        }
    }

    fn two_node_roots() -> TwoNodeRoots {
        let roots = TwoNodeRoots {
            requester_store: temp_dir("retention-remote-clearance-live-two-node-requester-store"),
            peer_store: temp_dir("retention-remote-clearance-live-two-node-peer-store"),
            requester_node: temp_dir("retention-remote-clearance-live-two-node-requester-node"),
            peer_node: temp_dir("retention-remote-clearance-live-two-node-peer-node"),
        };
        crate::node_daemon::init_local(&crate::node_daemon::InitInput {
            state_root: &roots.requester_node,
            node_id: "requester-node",
        })
        .expect("init requester node");
        crate::node_daemon::init_local(&crate::node_daemon::InitInput {
            state_root: &roots.peer_node,
            node_id: "peer-node",
        })
        .expect("init peer node");
        crate::node_daemon::run_local(&crate::node_daemon::RunInput {
            state_root: &roots.requester_node,
        })
        .expect("run requester node");
        crate::node_daemon::run_local(&crate::node_daemon::RunInput {
            state_root: &roots.peer_node,
        })
        .expect("run peer node");
        roots
    }

    async fn two_node_live() -> TwoNodeLive {
        let roots = two_node_roots();
        let topic = crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC;
        let peer_live = start_bound_live_node(&roots.peer_node, topic).await;
        let requester_live = start_bound_live_node(&roots.requester_node, topic).await;
        let control_policy_refs = vec![fake_ref("two-node-control-policy")];
        let control_resource_refs = vec![fake_ref("two-node-control-resource")];
        let request_evidence = install_live_direction_evidence(&LiveDirectionEvidenceInput {
            sender_root: &roots.requester_node,
            receiver_root: &roots.peer_node,
            receiver_ticket: &peer_live.ticket,
            sender_node_id: "requester-node",
            receiver_node_id: "peer-node",
            topic,
            policy_refs: &control_policy_refs,
        });
        let response_evidence = install_live_direction_evidence(&LiveDirectionEvidenceInput {
            sender_root: &roots.peer_node,
            receiver_root: &roots.requester_node,
            receiver_ticket: &requester_live.ticket,
            sender_node_id: "peer-node",
            receiver_node_id: "requester-node",
            topic,
            policy_refs: &control_policy_refs,
        });
        TwoNodeLive {
            roots,
            topic,
            peer_live,
            requester_live,
            control_policy_refs,
            control_resource_refs,
            request_evidence,
            response_evidence,
        }
    }

    fn two_node_refs(root: &Path) -> TwoNodeRefs {
        let requester_ref = fake_ref("two-node-requester");
        let peer_ref = fake_ref("two-node-peer");
        let object_ref = fake_ref("two-node-object");
        let remote_ref = fake_ref("two-node-remote");
        let policy = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_POLICY,
            label: "two-node-policy",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let authority = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_AUTHORITY,
            label: "two-node-authority",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let support = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_SUPPORTING_EVIDENCE,
            label: "two-node-support",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let index = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_REFERENCE_INDEX,
            label: "two-node-index",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: &[],
        });
        let remote_gc = store_two_node_admission(TwoNodeAdmissionInput {
            root,
            kind: ADMISSION_KIND_REMOTE_GC,
            label: "two-node-remote-gc",
            requester_ref: &requester_ref,
            object_ref: &object_ref,
            remote_refs: std::slice::from_ref(&remote_ref),
        });
        TwoNodeRefs {
            requester_ref,
            peer_ref,
            object_ref,
            remote_ref,
            policy,
            authority,
            support,
            index,
            remote_gc,
        }
    }

    fn store_two_node_admission(input: TwoNodeAdmissionInput<'_>) -> String {
        store_test_admission(TestAdmissionInput {
            root: input.root,
            kind: input.kind,
            label: input.label,
            requester_ref: input.requester_ref,
            object_ref: input.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_refs: input.remote_refs,
            is_reference_index_complete: true,
            is_current: true,
            revoked_refs: &[],
        })
    }

    async fn send_two_node_request(live: &mut TwoNodeLive, refs: &TwoNodeRefs) -> SentRequest {
        let send = send_remote_gc_clearance_live_request(RemoteGcClearanceLiveRequestSendInput {
            root: &live.roots.requester_store,
            requester_node_root: Some(&live.roots.requester_node),
            peer_ticket_value: &live.peer_live.ticket.value,
            requester_node_id: "requester-node",
            peer_node_id: "peer-node",
            topic: live.topic,
            sequence: 1,
            max_attempts: crate::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
            join_timeout_ms: 10_000,
            requester_ref: &refs.requester_ref,
            peer_ref: &refs.peer_ref,
            object_ref: &refs.object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &refs.remote_ref,
            policy_ref: &refs.policy,
            authority_ref: &refs.authority,
            retention_evidence_refs: std::slice::from_ref(&refs.support),
            peer_bootstrap_refs: &live.request_evidence.peer_bootstrap_refs,
            authority_refs: &live.request_evidence.authority_refs,
            policy_refs: &live.control_policy_refs,
            resource_refs: &live.control_resource_refs,
            transport_evidence_refs: &[],
        })
        .await
        .expect("two-node request send");
        let receipt = crate::node_daemon::parse_control_live_send_receipt(&send.send.send_receipt_value)
            .expect("request send receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(send.send.transport_receipt_ref.is_some());
        let receive =
            receive_one_live_ingress(&live.roots.peer_node, live.topic, "peer-node", &mut live.peer_live.topic).await;
        assert!(receive.has_enqueued);
        assert_eq!(receive.envelope_ref, send.send.envelope_ref);
        SentRequest { send, receive }
    }

    async fn send_two_node_response(live: &mut TwoNodeLive, request: &SentRequest) -> SentResponse {
        let peer_response_evidence = vec![fake_ref("two-node-peer-reference-index")];
        let send = send_remote_gc_clearance_live_response(RemoteGcClearanceLiveResponseSendInput {
            root: &live.roots.peer_store,
            peer_node_root: Some(&live.roots.peer_node),
            requester_ticket_value: &live.requester_live.ticket.value,
            request_value: &request.send.request.value,
            peer_node_id: "peer-node",
            requester_node_id: "requester-node",
            topic: live.topic,
            sequence: 1,
            max_attempts: crate::node_daemon::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
            join_timeout_ms: 10_000,
            response_evidence_refs: &peer_response_evidence,
            retained_refs: &[],
            is_current: true,
            revoked_refs: &[],
            response_diagnostics: &[],
            peer_bootstrap_refs: &live.response_evidence.peer_bootstrap_refs,
            authority_refs: &live.response_evidence.authority_refs,
            policy_refs: &live.control_policy_refs,
            resource_refs: &live.control_resource_refs,
            transport_evidence_refs: &[],
        })
        .await
        .expect("two-node response send");
        let receipt = crate::node_daemon::parse_control_live_send_receipt(&send.send.send_receipt_value)
            .expect("response send receipt");
        assert_eq!(receipt.decision, "pass");
        assert!(send.send.transport_receipt_ref.is_some());
        let receive = receive_one_live_ingress(
            &live.roots.requester_node,
            live.topic,
            "requester-node",
            &mut live.requester_live.topic,
        )
        .await;
        assert!(receive.has_enqueued);
        assert_eq!(receive.envelope_ref, send.send.envelope_ref);
        SentResponse { send, receive }
    }

    fn import_two_node_workflow(
        root: &Path,
        refs: &TwoNodeRefs,
        request: &SentRequest,
        response: &SentResponse,
    ) -> RemoteGcClearanceLiveImportWorkflow {
        import_remote_gc_clearance_live_workflow(RemoteGcClearanceLiveImportWorkflowInput {
            root,
            request_value: &request.send.request.value,
            response_value: &response.send.response.value,
            request_control_value: &request.send.control_value,
            request_send_receipt_value: &request.send.send.send_receipt_value,
            request_receive_receipt_value: &request.receive.transport_receipt_value,
            request_ingress_ref: &request.receive.ingress_receipt_ref,
            response_control_value: &response.send.control_value,
            response_send_receipt_value: &response.send.send.send_receipt_value,
            response_receive_receipt_value: &response.receive.transport_receipt_value,
            response_ingress_ref: &response.receive.ingress_receipt_ref,
            expected_peer_ref: Some(&refs.peer_ref),
            expected_remote_ref: Some(&refs.remote_ref),
        })
        .expect("two-node import workflow")
    }

    fn assert_two_node_import(
        imported: &RemoteGcClearanceLiveImportWorkflow,
        request: &SentRequest,
        response: &SentResponse,
    ) {
        assert_eq!(imported.import.decision, "pass");
        assert_eq!(imported.workflow.decision, "pass");
        assert!(imported.workflow.diagnostics.is_empty());
        assert_eq!(
            imported.workflow.request_live_refs[1],
            request.send.send.transport_receipt_ref.clone().expect("request publish receipt")
        );
        assert_eq!(imported.workflow.request_live_refs[2], request.receive.transport_receipt_ref);
        assert_eq!(
            imported.workflow.response_live_refs[1],
            response.send.send.transport_receipt_ref.clone().expect("response publish receipt")
        );
        assert_eq!(imported.workflow.response_live_refs[2], response.receive.transport_receipt_ref);
    }
