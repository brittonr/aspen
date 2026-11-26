//! Transport receipt non-authority and listener loopback tests.

pub(super) fn transport_receipt_ref(
    root: &std::path::Path,
    policy_refs: &[String],
    resource_refs: &[String],
    authority_refs: &[String],
) -> String {
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "status",
        target_ref: None,
        payload_ref: None,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs: &[],
    })
    .expect("transport seed request");
    let envelope =
        super::super::ingresses::control_live_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:transport",
            to_node: "node:transport-proof",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &[],
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs: &[],
        })
        .expect("transport seed envelope");
    let transport_value = super::super::transports::live_transport_receipt_value(
        &super::super::transports::LiveTransportReceiptValueInput {
            operation: "receive",
            decision: "pass",
            node_id: "node:transport-proof",
            delivered_from: Some("peer:transport"),
            envelope: &envelope,
            ingress_receipt_ref: None,
            topology_profile_ref: None,
            transport_profile_ref: None,
            effective_max_attempts: None,
            effective_join_timeout_ms: None,
            diagnostics: &[],
        },
    )
    .expect("transport receipt");
    super::super::storage::import_artifact(root, &transport_value).expect("import transport receipt")
}

pub(super) fn assert_transport_ref_does_not_bootstrap(
    root: &std::path::Path,
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
    let delivered =
        deliver_live_probe(root, &request_value, &peer_bootstrap_refs, authority_refs, policy_refs, resource_refs, 2);
    assert!(!delivered.has_enqueued);
    let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
    assert!(receipt_text.contains("transport evidence, not bootstrap authority"), "{receipt_text}");
}

pub(super) fn assert_transport_ref_does_not_authorize(
    root: &std::path::Path,
    policy_refs: &[String],
    resource_refs: &[String],
    transport_ref: &str,
) {
    let peer_bootstrap_refs = super::test_live_peer_bootstrap_refs(
        root,
        "peer:transport",
        super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        policy_refs,
    )
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
    let delivered =
        deliver_live_probe(root, &request_value, &peer_bootstrap_refs, &authority_refs, policy_refs, resource_refs, 3);
    assert!(!delivered.has_enqueued);
    let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
    assert!(receipt_text.contains("transport evidence, not authority"), "{receipt_text}");
}

fn deliver_live_probe(
    root: &std::path::Path,
    request_value: &preserves::IOValue,
    peer_bootstrap_refs: &[String],
    authority_refs: &[String],
    policy_refs: &[String],
    resource_refs: &[String],
    sequence: u64,
) -> super::super::transits::ControlIngressDeliver {
    let envelope =
        super::super::ingresses::control_live_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value,
            from_peer: "peer:transport",
            to_node: "node:transport-proof",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence,
            peer_bootstrap_refs,
            authority_refs,
            policy_refs,
            resource_refs,
            evidence_refs: &[],
        })
        .expect("live probe envelope");
    super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
        state_root: root,
        envelope_value: &envelope.value,
    })
    .expect("publish live probe");
    super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
        state_root: root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        envelope_ref: &envelope.envelope_ref,
    })
    .expect("deliver live probe")
}

#[tokio::test]
async fn control_live_serve_listener_loopback_dispatches_through_service() {
    let root = super::fixtures::temp_dir("node-control-live-listener");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:live-listener",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let policy_refs =
        vec![super::super::storage::local_ref("node-control-policy", "live-listener").expect("policy ref")];
    let authority_refs =
        super::test_live_authority_refs(&root, "peer:listener", "node:live-listener", "status", &policy_refs)
            .expect("authority grant ref");
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "live-listener").expect("resource ref")];
    let peer_bootstrap_refs = super::test_live_peer_bootstrap_refs(
        &root,
        "peer:listener",
        super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        &policy_refs,
    )
    .expect("peer admission ref");
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

    let loopback = super::super::listening::control_live_serve_listener_loopback(
        &super::super::requests::ControlLiveServeLoopbackInput {
            state_root: &root,
            request_value: &request_value,
            from_peer: "peer:listener",
            to_node: "node:live-listener",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
            max_requests_per_tick: 1,
        },
    )
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
