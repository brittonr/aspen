
    struct TicketPair {
        requester_root: PathBuf,
        peer_root: PathBuf,
        peer_ticket: crate::node_daemon::ControlLiveTicket,
        requester_ticket: crate::node_daemon::ControlLiveTicket,
    }

    struct NoEndpointCase {
        root: PathBuf,
        nodes: TicketPair,
        requester: String,
        peer: String,
        object: String,
        remote: String,
        policy: String,
        authority: String,
        evidence: String,
    }

    fn ticket_pair() -> TicketPair {
        let requester_root = temp_dir("retention-remote-clearance-live-multihost-requester");
        let peer_root = temp_dir("retention-remote-clearance-live-multihost-peer");
        crate::node_daemon::init_local(&crate::node_daemon::InitInput {
            state_root: &requester_root,
            node_id: "requester-node",
        })
        .expect("init requester node");
        crate::node_daemon::init_local(&crate::node_daemon::InitInput {
            state_root: &peer_root,
            node_id: "peer-node",
        })
        .expect("init peer node");
        let policy = vec![fake_ref("multihost-ticket-policy")];
        let evidence = vec![fake_ref("multihost-ticket-evidence")];
        let peer_ticket =
            crate::node_daemon::export_control_live_ticket(&crate::node_daemon::ControlLiveTicketExportInput {
                state_root: &peer_root,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                policy_refs: &policy,
                evidence_refs: &evidence,
            })
            .expect("peer ticket");
        let requester_ticket =
            crate::node_daemon::export_control_live_ticket(&crate::node_daemon::ControlLiveTicketExportInput {
                state_root: &requester_root,
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                policy_refs: &policy,
                evidence_refs: &evidence,
            })
            .expect("requester ticket");
        TicketPair {
            requester_root,
            peer_root,
            peer_ticket,
            requester_ticket,
        }
    }

    fn no_endpoint_case() -> NoEndpointCase {
        NoEndpointCase {
            root: temp_dir("retention-remote-clearance-live-multihost-send"),
            nodes: ticket_pair(),
            requester: fake_ref("multihost-send-requester"),
            peer: fake_ref("multihost-send-peer"),
            object: fake_ref("multihost-send-object"),
            remote: fake_ref("multihost-send-remote"),
            policy: fake_ref("multihost-send-policy"),
            authority: fake_ref("multihost-send-authority"),
            evidence: fake_ref("multihost-send-evidence"),
        }
    }

    fn case_request(runtime: &tokio::runtime::Runtime, case: &NoEndpointCase) -> RemoteGcClearanceLiveRequestSend {
        runtime
            .block_on(send_remote_gc_clearance_live_request(RemoteGcClearanceLiveRequestSendInput {
                root: &case.root,
                requester_node_root: Some(&case.nodes.requester_root),
                peer_ticket_value: &case.nodes.peer_ticket.value,
                requester_node_id: "requester-node",
                peer_node_id: "peer-node",
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                sequence: 1,
                max_attempts: 1,
                join_timeout_ms: 1,
                requester_ref: &case.requester,
                peer_ref: &case.peer,
                object_ref: &case.object,
                object_kind: "chunk",
                retention_class: CLASS_DURABLE_VALUE,
                action: ACTION_DELETE,
                remote_ref: &case.remote,
                policy_ref: &case.policy,
                authority_ref: &case.authority,
                retention_evidence_refs: std::slice::from_ref(&case.evidence),
                peer_bootstrap_refs: &[],
                authority_refs: &[],
                policy_refs: &[],
                resource_refs: &[],
                transport_evidence_refs: &[],
            }))
            .expect("request send")
    }

    fn case_response(
        runtime: &tokio::runtime::Runtime,
        case: &NoEndpointCase,
        request: &RemoteGcClearanceLiveRequestSend,
    ) -> RemoteGcClearanceLiveResponseSend {
        runtime
            .block_on(send_remote_gc_clearance_live_response(RemoteGcClearanceLiveResponseSendInput {
                root: &case.root,
                peer_node_root: Some(&case.nodes.peer_root),
                requester_ticket_value: &case.nodes.requester_ticket.value,
                request_value: &request.request.value,
                peer_node_id: "peer-node",
                requester_node_id: "requester-node",
                topic: crate::node_daemon::DEFAULT_CONTROL_INGRESS_TOPIC,
                sequence: 1,
                max_attempts: 1,
                join_timeout_ms: 1,
                response_evidence_refs: std::slice::from_ref(&case.evidence),
                retained_refs: &[],
                is_current: true,
                revoked_refs: &[],
                response_diagnostics: &[],
                peer_bootstrap_refs: &[],
                authority_refs: &[],
                policy_refs: &[],
                resource_refs: &[],
                transport_evidence_refs: &[],
            }))
            .expect("response send")
    }

    fn assert_send_denial(value: &IoValue, expected: Option<&str>) {
        let receipt = crate::node_daemon::parse_control_live_send_receipt(value).expect("send receipt");
        assert_eq!(receipt.decision, "deny");
        if let Some(needle) = expected {
            assert!(receipt.diagnostics.iter().any(|value| value.contains(needle)));
        }
    }

    fn assert_summary_contains(value: &IoValue, expected: &str) {
        let summary = summary(value).expect("retention summary");
        assert!(summary.contains(expected), "{summary}");
    }

    fn fixture_material(root: &Path) -> Material {
        let case = live_case(root, "multihost");
        let pair = request_pair(root, &case);
        let (request_control, response_control) = control_values(&pair);
        Material {
            case,
            request_value: pair.request_value,
            response_value: pair.response_value,
            request_control,
            response_control,
            traffic: traffic_values(),
        }
    }

    fn import_with(root: &Path, material: &Material, request_receive: &IoValue) -> RemoteGcClearanceLiveImportWorkflow {
        import_remote_gc_clearance_live_workflow(RemoteGcClearanceLiveImportWorkflowInput {
            root,
            request_value: &material.request_value,
            response_value: &material.response_value,
            request_control_value: &material.request_control,
            request_send_receipt_value: &material.traffic.request_send,
            request_receive_receipt_value: request_receive,
            request_ingress_ref: &material.traffic.request_ingress,
            response_control_value: &material.response_control,
            response_send_receipt_value: &material.traffic.response_send,
            response_receive_receipt_value: &material.traffic.response_receive,
            response_ingress_ref: &material.traffic.response_ingress,
            expected_peer_ref: Some(&material.case.peer),
            expected_remote_ref: Some(&material.case.remote),
        })
        .expect("workflow import")
    }

    fn assert_import_pass(root: &Path, material: &Material) -> String {
        let imported = import_with(root, material, &material.traffic.request_receive);
        assert_eq!(imported.import.decision, "pass");
        assert_eq!(imported.workflow.decision, "pass");
        assert_eq!(imported.workflow.request_live_refs.len(), 4);
        imported.import.clearance_ref.clone().expect("clearance stored")
    }

    fn assert_wrong_receive(root: &Path, material: &Material, wrong_request_receive: &IoValue) {
        let workflow = import_with(root, material, wrong_request_receive);
        assert_eq!(workflow.import.decision, "pass");
        assert_eq!(workflow.workflow.decision, "deny");
        assert!(
            workflow
                .workflow
                .diagnostics
                .iter()
                .any(|value| value.contains("remote-clearance-live-request-receive-not-receive"))
        );
        assert!(
            workflow
                .workflow
                .diagnostics
                .iter()
                .any(|value| value == "remote-clearance-live-request-receive-wrong-envelope")
        );
        assert!(
            workflow
                .workflow
                .diagnostics
                .iter()
                .any(|value| value == "remote-clearance-live-request-receive-wrong-ingress")
        );
    }

    fn store_passing_plan_fixture(root: &std::path::Path, label: &str) -> TestPlanFixture {
        let requester_ref = fake_ref(&format!("{label}-requester"));
        let object_ref = fake_ref(&format!("{label}-object"));
        let peer_ref = fake_ref(&format!("{label}-peer"));
        let remote_ref = fake_ref(&format!("{label}-remote"));
        let remote_refs = std::slice::from_ref(&remote_ref);
        let [policy, authority, support, index, remote_gc] =
            seed_set(root, label, &requester_ref, &object_ref, remote_refs);
        let remote_clearance = store_test_remote_clearance(TestRemoteClearanceInput {
            root,
            label: &format!("{label}-clearance"),
            requester_ref: &requester_ref,
            peer_ref: &peer_ref,
            object_ref: &object_ref,
            object_kind: "chunk",
            retention_class: CLASS_DURABLE_VALUE,
            action: ACTION_DELETE,
            remote_ref: &remote_ref,
            policy_ref: &policy,
            authority_ref: &authority,
            is_current: true,
            revoked_refs: &[],
            retained_refs: &[],
        });
        TestPlanFixture {
            requester_ref: requester_ref.clone(),
            object_ref,
            evidence: DestructiveEvidence {
                requester_ref: Some(requester_ref),
                policy_refs: vec![policy],
                authority_refs: vec![authority],
                evidence_refs: vec![support],
                retained_refs: Vec::new(),
                remote_peer_refs: vec![peer_ref],
                remote_refs: vec![remote_ref],
                reference_index_refs: vec![index],
                remote_gc_refs: vec![remote_gc],
                remote_clearance_refs: vec![remote_clearance],
                is_reference_index_complete: true,
            },
        }
    }

    fn fake_ref(label: &str) -> String {
        crate::preserves_rail::canonical_hash(&crate::preserves_rail::record("retention-test-ref", vec![
            crate::preserves_rail::string(label),
        ]))
        .expect("fake ref")
    }

    fn store_file_count(dir: &Path) -> usize {
        if !dir.exists() {
            return 0;
        }
        fs::read_dir(dir).expect("read store dir").filter_map(std::result::Result::ok).count()
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
