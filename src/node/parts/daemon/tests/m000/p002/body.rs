
    fn assert_enqueued_then_denied(root: &Path, envelope: &ControlIngressEnvelope) {
        let published = publish_control_ingress(&ControlIngressPublishInput {
            state_root: root,
            envelope_value: &envelope.value,
        })
        .expect("publish ingress");
        assert_eq!(crate::ledger::artifact_kind(&published.receipt_value), "node-control-ingress-receipt");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver ingress");
        assert!(delivered.has_enqueued);
        assert!(delivered.queue_receipt_ref.is_some());

        let duplicate = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("duplicate ingress");
        assert!(!duplicate.has_enqueued);
        assert!(duplicate.idempotency_receipt_ref.is_some());

        let loop_result = run_control_loop(&ControlLoopInput {
            state_root: root,
            max_requests: 1,
        })
        .expect("dispatch ingress request");
        assert_eq!(loop_result.processed_request_refs.len(), 1);
        let state_root = crate::node_state::NodeStateRoot::open(root).expect("open node state root");
        let control_value = read_preserves(
            &state_root,
            &control_outbox_receipt_path(&delivered.request_ref).expect("outbox receipt path"),
        )
        .expect("read ingress dispatch receipt");
        let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse control receipt");
        assert_eq!(control.decision, "deny");
        assert!(control.diagnostics.iter().any(|diagnostic| diagnostic.contains("provenance evidence refs missing")));
    }

    #[test]
    fn control_ingress_denies_tampered_materialized_envelope_ref() {
        // r[verify molten.runtime_spine.canonical_content_refs.node_control]
        // r[verify molten.runtime_spine.canonical_content_refs.negative_tests]
        let pair = materialized_ingress_pair();
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &pair.root,
            envelope_value: &pair.first.value,
        })
        .expect("publish first");
        let state_root = crate::node_state::NodeStateRoot::open(&pair.root).expect("open node state root");
        write_preserves(
            &state_root,
            &control_ingress_envelope_path(DEFAULT_CONTROL_INGRESS_TOPIC, &pair.first.envelope_ref)
                .expect("ingress envelope path"),
            &pair.second.value,
        )
        .expect("tamper materialized envelope");
        let denied = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &pair.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &pair.first.envelope_ref,
        })
        .expect_err("materialized ref mismatch denied");
        assert!(denied.to_string().contains("materialized envelope ref"));
    }

    struct MaterializedIngressPair {
        root: PathBuf,
        first: ControlIngressEnvelope,
        second: ControlIngressEnvelope,
    }

    struct MaterializedIngressRefs {
        authority_refs: Vec<String>,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
    }

    const FIRST_MATERIALIZED_ENVELOPE_SEQUENCE: u64 = 1;
    const SECOND_MATERIALIZED_ENVELOPE_SEQUENCE: u64 = 2;

    fn materialized_ingress_pair() -> MaterializedIngressPair {
        let root = initialized_materialized_ingress_root();
        let refs = materialized_ingress_refs();
        let request_value = materialized_request_value(&root, &refs);
        MaterializedIngressPair {
            first: materialized_envelope(&request_value, &refs, FIRST_MATERIALIZED_ENVELOPE_SEQUENCE),
            second: materialized_envelope(&request_value, &refs, SECOND_MATERIALIZED_ENVELOPE_SEQUENCE),
            root,
        }
    }

    fn initialized_materialized_ingress_root() -> PathBuf {
        let root = temp_dir("node-control-ingress-materialized-ref");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:ingress-materialized",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        root
    }

    fn materialized_ingress_refs() -> MaterializedIngressRefs {
        MaterializedIngressRefs {
            authority_refs: vec![local_ref("node-control-authority", "materialized").expect("authority ref")],
            policy_refs: vec![local_ref("node-control-policy", "materialized").expect("policy ref")],
            resource_refs: vec![local_ref("node-control-resource", "materialized").expect("resource ref")],
            peer_bootstrap_refs: vec![local_ref("peer-bootstrap", "peer:materialized").expect("bootstrap ref")],
        }
    }

    fn materialized_request_value(root: &Path, refs: &MaterializedIngressRefs) -> IoValue {
        let payload_value =
            crate::preserves_rail::record("node-control-ingress-payload", vec![crate::preserves_rail::string(
                "materialized",
            )]);
        let payload_ref = import_artifact(root, &payload_value).expect("import payload");
        crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "install",
            target_ref: None,
            payload_ref: Some(&payload_ref),
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs: &[],
        })
        .expect("request")
    }

    fn materialized_envelope(
        request_value: &IoValue,
        refs: &MaterializedIngressRefs,
        sequence: u64,
    ) -> ControlIngressEnvelope {
        control_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value,
            from_peer: "peer:materialized",
            to_node: "node:ingress-materialized",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence,
            peer_bootstrap_refs: &refs.peer_bootstrap_refs,
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs: &[],
        })
        .expect("materialized envelope")
    }

    #[test]
    fn control_live_workflow_bundle_reconcile_binds_receiver_evidence() {
        let case = reconcile_case();
        let reconciled = assert_reconcile_pass(&case);
        let denials = assert_reconcile_denials(&case);
        let ack = assert_ack_pass(&case, &reconciled, &denials.wrong_envelope);
        assert_ack_denials(&case, &denials, &ack);
    }

    struct ReconcileSeed {
        root: PathBuf,
        policy_refs: Vec<String>,
        resource_refs: Vec<String>,
        peer_bootstrap_refs: Vec<String>,
        authority_refs: Vec<String>,
    }

    struct ReconcileDelivery {
        root: PathBuf,
        request: crate::node_runtime::ControlRequest,
        envelope: ControlIngressEnvelope,
        delivered: ControlIngressDeliver,
        queue_value: IoValue,
        control_value: IoValue,
        control_receipt_ref: String,
        policy_refs: Vec<String>,
        operations: Vec<String>,
    }

    struct ReconcileCase {
        delivery: ReconcileDelivery,
        exported: ControlLiveWorkflowBundleExport,
        gated: ControlLiveWorkflowBundleGate,
        apply_receipt_value: IoValue,
    }

    struct ReconcileDenials {
        missing_receiver: ControlLiveWorkflowBundleReconcile,
        denied_control: IoValue,
        denied_reconcile: ControlLiveWorkflowBundleReconcile,
        wrong_envelope: String,
    }

    struct AckPass {
        import_root: PathBuf,
    }

    fn reconcile_expected<'a>(operations: &'a [String]) -> LiveWorkflowBundleExpectedInput<'a> {
        LiveWorkflowBundleExpectedInput {
            expected_node: Some("node:reconcile"),
            expected_topic: Some(DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: None,
            expected_peer: Some("peer:reconcile"),
            expected_operations: operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 1,
            as_of_epoch: 1,
        }
    }

    fn reconcile_seed() -> ReconcileSeed {
        let root = temp_dir("node-control-live-workflow-reconcile");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:reconcile",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "reconcile").expect("policy ref")];
        let resource_refs = vec![local_ref("node-control-resource", "reconcile").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:reconcile", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer bootstrap");
        let authority_refs =
            test_live_authority_refs(&root, "peer:reconcile", "node:reconcile", "status", &policy_refs)
                .expect("authority refs");
        ReconcileSeed {
            root,
            policy_refs,
            resource_refs,
            peer_bootstrap_refs,
            authority_refs,
        }
    }

    fn reconcile_request(seed: &ReconcileSeed) -> (IoValue, crate::node_runtime::ControlRequest) {
        let request_value =
            crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
                operation: "status",
                target_ref: None,
                payload_ref: None,
                authority_refs: &seed.authority_refs,
                policy_refs: &seed.policy_refs,
                resource_refs: &seed.resource_refs,
                evidence_refs: &[],
            })
            .expect("status request");
        let request = crate::node_runtime::parse_control_request(&request_value).expect("request");
        (request_value, request)
    }

    fn deliver_reconcile_envelope(
        seed: &ReconcileSeed,
        request_value: &IoValue,
    ) -> (ControlIngressEnvelope, ControlIngressDeliver) {
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &seed.peer_bootstrap_refs,
            authority_refs: &seed.authority_refs,
            policy_refs: &seed.policy_refs,
            resource_refs: &seed.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: &seed.root,
            envelope_value: &envelope.value,
        })
        .expect("publish envelope");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: &seed.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver envelope");
        assert!(
            delivered.has_enqueued,
            "{}",
            crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("ingress receipt text")
        );
        (envelope, delivered)
    }

    fn dispatched_reconcile(seed: &ReconcileSeed, delivered: &ControlIngressDeliver) -> (IoValue, IoValue, String) {
        run_control_loop(&ControlLoopInput {
            state_root: &seed.root,
            max_requests: 1,
        })
        .expect("dispatch request");
        let state_root = crate::node_state::NodeStateRoot::open(&seed.root).expect("open node state root");
        let queue_value = read_preserves(
            &state_root,
            &queue_receipt_path(&delivered.request_ref).expect("queue receipt path"),
        )
        .expect("queue receipt");
        let control_value = read_preserves(
            &state_root,
            &control_outbox_receipt_path(&delivered.request_ref).expect("control receipt path"),
        )
        .expect("control receipt");
        let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse control");
        assert_eq!(control.decision, "pass");
        (queue_value, control_value, control.receipt_ref)
    }
