
    fn assert_peer_delivery(input: PeerDelivery<'_>) {
        let envelope = control_live_ingress_envelope(&ControlIngressEnvelopeInput {
            request_value: input.request_value,
            from_peer: input.from_peer,
            to_node: input.to_node,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: input.peer_bootstrap_refs,
            authority_refs: input.authority_refs,
            policy_refs: input.policy_refs,
            resource_refs: input.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
        publish_control_ingress(&ControlIngressPublishInput {
            state_root: input.root,
            envelope_value: &envelope.value,
        })
        .expect("publish envelope");
        let delivered = deliver_control_ingress(&ControlIngressDeliverInput {
            state_root: input.root,
            topic: DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver envelope");
        assert_eq!(delivered.has_enqueued, input.is_expected_enqueued);
        if let Some(expected_note) = input.expected_note {
            let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
            assert!(receipt_text.contains(expected_note));
        }
    }
