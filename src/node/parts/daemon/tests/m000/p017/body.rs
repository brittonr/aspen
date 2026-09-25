
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
