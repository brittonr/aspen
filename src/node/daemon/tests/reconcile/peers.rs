//! Missing-authority ingress and peer ticket admission tests.

#[test]
fn control_ingress_denies_missing_authority_before_enqueue() {
    let root = super::fixtures::temp_dir("node-control-ingress-deny");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:ingress-deny",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let request = super::super::running::status_request().expect("status request");
    let peer_bootstrap_refs =
        vec![super::super::storage::local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];
    let policy_refs =
        vec![super::super::storage::local_ref("node-control-policy", "ingress-deny").expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "ingress-deny").expect("resource ref")];
    let envelope =
        super::super::ingresses::control_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: &request.value,
            from_peer: "peer:operator",
            to_node: "node:ingress-deny",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &[],
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("missing authority envelope");
    super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
        state_root: &root,
        envelope_value: &envelope.value,
    })
    .expect("publish denied ingress");
    let delivered =
        super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
            state_root: &root,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver denied ingress");
    assert!(!delivered.has_enqueued);
    let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
    assert!(receipt_text.contains("authority refs missing"));
    assert!(super::super::leases::next_pending_control_request(&root).expect("pending request scan").is_none());
}

struct PeerDelivery<'a> {
    root: &'a std::path::Path,
    request_value: &'a preserves::IOValue,
    from_peer: &'a str,
    to_node: &'a str,
    peer_bootstrap_refs: &'a [String],
    authority_refs: &'a [String],
    policy_refs: &'a [String],
    resource_refs: &'a [String],
    is_expected_enqueued: bool,
    expected_note: Option<&'a str>,
}

fn assert_peer_delivery(input: PeerDelivery<'_>) {
    let envelope =
        super::super::ingresses::control_live_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: input.request_value,
            from_peer: input.from_peer,
            to_node: input.to_node,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: input.peer_bootstrap_refs,
            authority_refs: input.authority_refs,
            policy_refs: input.policy_refs,
            resource_refs: input.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
    super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
        state_root: input.root,
        envelope_value: &envelope.value,
    })
    .expect("publish envelope");
    let delivered =
        super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
            state_root: input.root,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver envelope");
    assert_eq!(delivered.has_enqueued, input.is_expected_enqueued);
    if let Some(expected_note) = input.expected_note {
        let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
        assert!(receipt_text.contains(expected_note));
    }
}

#[test]
fn control_live_peer_ticket_admission_gates_bootstrap() {
    let root = super::fixtures::temp_dir("node-control-live-peer-ticket");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:live-ticket",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let policy_refs = vec![super::super::storage::local_ref("node-control-policy", "live-ticket").expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "live-ticket").expect("resource ref")];
    let peer_bootstrap_refs = super::test_live_peer_bootstrap_refs(
        &root,
        "peer:ticket",
        super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        &policy_refs,
    )
    .expect("peer admission ref");
    let authority_refs =
        super::test_live_authority_refs(&root, "peer:ticket", "node:live-ticket", "status", &policy_refs)
            .expect("authority grant ref");
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
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
