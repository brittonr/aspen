
    #[test]
    fn remote_clearance_live_loopback_imports_peer_clearance_for_destructive_admission() {
        let root = temp_dir("retention-remote-clearance-live-loopback");
        let requester_node_root = temp_dir("retention-live-requester-node");
        let peer_node_root = temp_dir("retention-live-peer-node");
        let requester_node_id = "retention-live-requester";
        let peer_node_id = "retention-live-peer";
        crate::node_daemon::init_local(&crate::node_daemon::InitInput {
            state_root: &requester_node_root,
            node_id: requester_node_id,
        })
        .expect("init requester node");
        crate::node_daemon::init_local(&crate::node_daemon::InitInput {
            state_root: &peer_node_root,
            node_id: peer_node_id,
        })
        .expect("init peer node");
        let request_live = live_direction_refs(&peer_node_root, requester_node_id, "request");
        let response_live = live_direction_refs(&requester_node_root, peer_node_id, "response");
        let case = live_case(&root, "live");
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        let live = runtime
            .block_on(run_remote_gc_clearance_live_loopback(RemoteGcClearanceLiveLoopbackInput {
                root: &root,
                requester_node_root: &requester_node_root,
                peer_node_root: &peer_node_root,
                requester_node_id,
                peer_node_id,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                request_sequence: 1,
                response_sequence: 1,
                requester_ref: &case.requester,
                peer_ref: &case.peer,
                object_ref: &case.object,
                object_kind: "chunk",
                retention_class: CLASS_DURABLE_VALUE,
                action: ACTION_DELETE,
                remote_ref: &case.remote,
                policy_ref: &case.policy,
                authority_ref: &case.authority,
                retention_evidence_refs: std::slice::from_ref(&case.support),
                response_evidence_refs: &[fake_ref("live-peer-evidence")],
                retained_refs: &[],
                is_current: true,
                revoked_refs: &[],
                response_diagnostics: &[],
                request_peer_bootstrap_refs: &request_live.peer_bootstrap_refs,
                request_authority_refs: &request_live.authority_refs,
                request_policy_refs: &request_live.policy_refs,
                request_resource_refs: &request_live.resource_refs,
                request_transport_evidence_refs: &request_live.evidence_refs,
                response_peer_bootstrap_refs: &response_live.peer_bootstrap_refs,
                response_authority_refs: &response_live.authority_refs,
                response_policy_refs: &response_live.policy_refs,
                response_resource_refs: &response_live.resource_refs,
                response_transport_evidence_refs: &response_live.evidence_refs,
            }))
            .expect("live loopback");
        assert_eq!(live.workflow.decision, "pass");
        assert_eq!(live.import.decision, "pass");
        let clearance_ref = live.import.clearance_ref.clone().expect("live clearance imported");
        assert_case_pass(&root, &case, clearance_ref);
    }

    #[test]
    fn remote_clearance_live_multihost_request_and_response_send_write_artifacts_on_denied_transport() {
        let case = no_endpoint_case();
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");

        let request = case_request(&runtime, &case);
        assert_eq!(request.request.peer_ref, case.peer);
        assert_send_denial(&request.send.send_receipt_value, Some("ticket has no endpoint addresses"));

        let response = case_response(&runtime, &case, &request);
        assert_eq!(response.response.request_ref, request.request.request_ref);
        assert_send_denial(&response.send.send_receipt_value, None);
    }

    #[test]
    fn remote_clearance_live_multihost_import_workflow_binds_explicit_send_receive_evidence() {
        let root = temp_dir("retention-remote-clearance-live-multihost");
        let material = fixture_material(&root);
        let clearance_ref = assert_import_pass(&root, &material);
        let wrong_request_receive = fake_live_transport_receipt(
            "publish",
            "wrong-peer-node",
            "wrong-request-envelope",
            &fake_ref("wrong-request-ingress"),
        );
        assert_wrong_receive(&root, &material, &wrong_request_receive);
        assert_case_pass(&root, &material.case, clearance_ref);
    }

    #[test]
    fn remote_clearance_live_multihost_two_node_happy_path_uses_real_receive_evidence() {
        let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
        runtime.block_on(async {
            let mut live = two_node_live().await;
            let refs = two_node_refs(&live.roots.requester_store);
            let request = send_two_node_request(&mut live, &refs).await;
            let response = send_two_node_response(&mut live, &request).await;
            let imported = import_two_node_workflow(&live.roots.requester_store, &refs, &request, &response);
            assert_two_node_import(&imported, &request, &response);
            assert_two_node_admission(&live.roots.requester_store, refs, imported);
            live.shutdown().await;
        });
    }

    #[test]
    fn remote_clearance_live_workflow_denies_retained_wrong_peer_and_tampered_response() {
        let root = temp_dir("retention-remote-clearance-live-deny");
        let requester_ref = fake_ref("live-deny-requester");
        let peer_ref = fake_ref("live-deny-peer");
        let object_ref = fake_ref("live-deny-object");
        let remote_ref = fake_ref("live-deny-remote");
        let policy = fake_ref("live-deny-policy");
        let authority = fake_ref("live-deny-authority");
        let request = store_remote_gc_clearance_request(&root, &RemoteGcClearanceRequestInput {
            requester_ref: &requester_ref,
            peer_ref: &peer_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &remote_ref,
            policy_ref: &policy,
            authority_ref: &authority,
            evidence_refs: &[],
        })
        .expect("store live deny request");
        let live_refs = fake_live_refs("retained");

        assert_retained(&root, &request, &remote_ref, &live_refs);
        assert_tampered(&root, &request, &peer_ref, &remote_ref, &live_refs);
    }

    fn assert_retained(root: &Path, request: &RemoteGcClearanceRequest, remote_ref: &str, live_refs: &[String]) {
        let retained_ref = fake_ref("live-deny-retained");
        let response = store_remote_gc_clearance_response(RemoteGcClearanceResponseInput {
            root,
            request_value: &request.value,
            evidence_refs: &[],
            retained_refs: std::slice::from_ref(&retained_ref),
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store retained response");
        let wrong_peer_import = import_remote_gc_clearance_response(RemoteGcClearanceImportInput {
            root,
            request_value: &request.value,
            response_value: &response.value,
            expected_peer_ref: Some(&fake_ref("wrong-live-peer")),
            expected_remote_ref: Some(remote_ref),
        })
        .expect("wrong peer import");
        let retained_workflow = remote_gc_clearance_live_workflow_value(&RemoteGcClearanceLiveWorkflowValueInput {
            request_value: &request.value,
            response_value: &response.value,
            import_value: &wrong_peer_import.value,
            request_control_ref: &live_refs[0],
            request_publish_ref: &live_refs[1],
            request_receive_ref: &live_refs[2],
            request_ingress_ref: &live_refs[3],
            response_control_ref: &live_refs[4],
            response_publish_ref: &live_refs[5],
            response_receive_ref: &live_refs[6],
            response_ingress_ref: &live_refs[7],
            transport_diagnostics: &[],
        })
        .expect("retained live workflow value");
        let retained = parse_remote_gc_clearance_live_workflow(&retained_workflow).expect("parse retained live");
        assert_eq!(retained.decision, "deny");
        assert!(retained.diagnostics.iter().any(|diagnostic| diagnostic == "remote-clearance-retained"));
        assert!(
            retained
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic == "remote-clearance-expected-peer-mismatch")
        );
    }

    fn assert_tampered(
        root: &Path,
        request: &RemoteGcClearanceRequest,
        peer_ref: &str,
        remote_ref: &str,
        live_refs: &[String],
    ) {
        let tampered_response =
            crate::preserves_rail::record("not-a-remote-clearance-response", vec![crate::preserves_rail::string(
                "tampered",
            )]);
        let tampered_import = import_remote_gc_clearance_response(RemoteGcClearanceImportInput {
            root,
            request_value: &request.value,
            response_value: &tampered_response,
            expected_peer_ref: Some(peer_ref),
            expected_remote_ref: Some(remote_ref),
        })
        .expect("tampered import");
        let tampered_workflow = remote_gc_clearance_live_workflow_value(&RemoteGcClearanceLiveWorkflowValueInput {
            request_value: &request.value,
            response_value: &tampered_response,
            import_value: &tampered_import.value,
            request_control_ref: &live_refs[0],
            request_publish_ref: &live_refs[1],
            request_receive_ref: &live_refs[2],
            request_ingress_ref: &live_refs[3],
            response_control_ref: &live_refs[4],
            response_publish_ref: &live_refs[5],
            response_receive_ref: &live_refs[6],
            response_ingress_ref: &live_refs[7],
            transport_diagnostics: &[],
        })
        .expect("tampered live workflow value");
        let tampered = parse_remote_gc_clearance_live_workflow(&tampered_workflow).expect("parse tampered live");
        assert_eq!(tampered.decision, "deny");
        assert!(
            tampered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.starts_with("remote-clearance-live-tampered-response"))
        );
    }

    struct LiveNodeHarness {
        ticket: crate::node_daemon::ControlLiveTicket,
        topic: iroh_gossip::api::GossipTopic,
        router: iroh::protocol::Router,
    }

    struct LiveDirectionEvidenceInput<'a> {
        sender_root: &'a Path,
        receiver_root: &'a Path,
        receiver_ticket: &'a crate::node_daemon::ControlLiveTicket,
        sender_node_id: &'a str,
        receiver_node_id: &'a str,
        topic: &'a str,
        policy_refs: &'a [String],
    }

    struct LiveDirectionEvidence {
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
    }

    struct TwoNodeRoots {
        requester_store: PathBuf,
        peer_store: PathBuf,
        requester_node: PathBuf,
        peer_node: PathBuf,
    }

    struct TwoNodeLive {
        roots: TwoNodeRoots,
        topic: &'static str,
        peer_live: LiveNodeHarness,
        requester_live: LiveNodeHarness,
        control_policy_refs: Vec<String>,
        control_resource_refs: Vec<String>,
        request_evidence: LiveDirectionEvidence,
        response_evidence: LiveDirectionEvidence,
    }

    struct TwoNodeRefs {
        requester_ref: String,
        peer_ref: String,
        object_ref: String,
        remote_ref: String,
        policy: String,
        authority: String,
        support: String,
        index: String,
        remote_gc: String,
    }

    struct TwoNodeAdmissionInput<'a> {
        root: &'a Path,
        kind: &'a str,
        label: &'a str,
        requester_ref: &'a str,
        object_ref: &'a str,
        remote_refs: &'a [String],
    }

    struct SentRequest {
        send: RemoteGcClearanceLiveRequestSend,
        receive: crate::node_daemon::ControlLiveIngressReceive,
    }

    struct SentResponse {
        send: RemoteGcClearanceLiveResponseSend,
        receive: crate::node_daemon::ControlLiveIngressReceive,
    }
