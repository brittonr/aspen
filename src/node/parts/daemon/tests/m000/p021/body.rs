
    fn assert_transport_ref_does_not_bootstrap(
        root: &Path,
        policy_refs: &[String],
        resource_refs: &[String],
        authority_refs: &[String],
        transport_ref: &str,
    ) {
        let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs: &[],
        })
        .expect("bootstrap request");
        let peer_bootstrap_refs = vec![transport_ref.to_string()];
        let delivered = deliver_live_probe(
            root,
            &request_value,
            &peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            2,
        );
        assert!(!delivered.has_enqueued);
        let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains("transport evidence, not bootstrap authority"), "{receipt_text}");
    }

    fn assert_transport_ref_does_not_authorize(
        root: &Path,
        policy_refs: &[String],
        resource_refs: &[String],
        transport_ref: &str,
    ) {
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(root, "peer:transport", DEFAULT_CONTROL_INGRESS_TOPIC, policy_refs)
                .expect("peer admission ref");
        let authority_refs = vec![transport_ref.to_string()];
        let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
            operation: "status",
            target_ref: None,
            payload_ref: None,
            authority_refs: &authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs: &[],
        })
        .expect("authority request");
        let delivered = deliver_live_probe(
            root,
            &request_value,
            &peer_bootstrap_refs,
            &authority_refs,
            policy_refs,
            resource_refs,
            3,
        );
        assert!(!delivered.has_enqueued);
        let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains("transport evidence, not authority"), "{receipt_text}");
    }

    fn deliver_live_probe(
        root: &Path,
        request_value: &IoValue,
        peer_bootstrap_refs: &[String],
        authority_refs: &[String],
        policy_refs: &[String],
        resource_refs: &[String],
        sequence: u64,
    ) -> ControlIngressDeliver {
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value,
            from_peer: "peer:transport",
            to_node: "node:transport-proof",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs: &[],
        })
        .expect("live probe envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: root,
            envelope_value: &envelope.value,
        })
        .expect("publish live probe");
        deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver live probe")
    }

    #[tokio::test]
    async fn control_live_serve_listener_loopback_dispatches_through_service() {
        let root = temp_dir("node-control-live-listener");
        init_local(&InitInput {
            state_root: &root,
            node_id: "node:live-listener",
        })
        .expect("init node");
        run_local(&RunInput { state_root: &root }).expect("run node");
        let policy_refs = vec![local_ref("node-control-policy", "live-listener").expect("policy ref")];
        let authority_refs =
            test_live_authority_refs(&root, "peer:listener", "node:live-listener", "status", &policy_refs)
                .expect("authority grant ref");
        let resource_refs = vec![local_ref("node-control-resource", "live-listener").expect("resource ref")];
        let peer_bootstrap_refs =
            test_live_peer_bootstrap_refs(&root, "peer:listener", DEFAULT_CONTROL_INGRESS_TOPIC, &policy_refs)
                .expect("peer admission ref");
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

        let loopback = control_live_serve_listener_loopback(&ControlLiveServeLoopbackInput {
            state_root: &root,
            request_value: &request_value,
            from_peer: "peer:listener",
            to_node: "node:live-listener",
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
            max_requests_per_tick: 1,
        })
        .await
        .expect("live listener loopback");
        assert_eq!(
            crate::ledger::artifact_kind(&loopback.listener.listener_receipt_value),
            "node-control-live-listener-receipt"
        );
        assert_eq!(loopback.listener.service.decision, "pass");
        assert_eq!(loopback.listener.service.processed_request_refs.len(), 1);
        assert_eq!(loopback.listener.transport_receipt_refs.len(), 1);
        assert!(loopback.listener.observed_events >= 1);
    }
