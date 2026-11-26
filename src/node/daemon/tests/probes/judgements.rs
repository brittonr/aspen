//! Authority delegation denial assertions.

fn denied_case_refs(root: &std::path::Path, case: &super::denycases::DenyCase<'_>) -> super::denycases::DenyCaseRefs {
    let policy_refs = vec![super::super::storage::local_ref("node-control-policy", case.name).expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", case.name).expect("resource ref")];
    let peer_bootstrap_refs = super::test_live_peer_bootstrap_refs(
        root,
        "peer:case",
        super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        &policy_refs,
    )
    .expect("peer admission ref");
    let authority_refs = if let Some(grant_peer) = case.grant_peer {
        let operations = case.grant_operations.iter().map(|operation| (*operation).to_string()).collect::<Vec<_>>();
        let revocation_refs = if case.is_revoked {
            vec![super::super::storage::local_ref("node-control-revocation", case.name).expect("revocation ref")]
        } else {
            Vec::new()
        };
        let grant_value =
            super::super::issuance::control_authority_grant_value(&super::super::offers::ControlAuthorityGrantInput {
                peer_id: grant_peer,
                node_id: case.grant_node,
                operations: &operations,
                target_scope: case.target_scope,
                resource_scope: case.resource_scope,
                epoch: case.epoch,
                expires_at: case.expires_at,
                policy_refs: &policy_refs,
                revocation_refs: &revocation_refs,
                evidence_refs: &[],
            })
            .expect("authority grant value");
        vec![
            super::super::issuance::import_control_authority_grant(root, &grant_value)
                .expect("import authority grant")
                .grant_ref,
        ]
    } else {
        vec![super::super::storage::local_ref("node-control-authority", case.name).expect("authority ref")]
    };
    super::denycases::DenyCaseRefs {
        policy_refs,
        resource_refs,
        peer_bootstrap_refs,
        authority_refs,
    }
}

fn assert_denied_case(case: &super::denycases::DenyCase<'_>) {
    let root = super::fixtures::temp_dir(&format!("node-control-live-authority-{}", case.name));
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:live-authority",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let refs = denied_case_refs(&root, case);
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "status",
        target_ref: case.target_ref,
        payload_ref: None,
        authority_refs: &refs.authority_refs,
        policy_refs: &refs.policy_refs,
        resource_refs: &refs.resource_refs,
        evidence_refs: &[],
    })
    .expect("status request");
    let envelope =
        super::super::ingresses::control_live_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:case",
            to_node: "node:live-authority",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: case.sequence,
            peer_bootstrap_refs: &refs.peer_bootstrap_refs,
            authority_refs: &refs.authority_refs,
            policy_refs: &refs.policy_refs,
            resource_refs: &refs.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
    super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
        state_root: &root,
        envelope_value: &envelope.value,
    })
    .expect("publish live envelope");
    let delivered =
        super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
            state_root: &root,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver live envelope");
    assert!(!delivered.has_enqueued, "{} enqueued", case.name);
    let receipt_text = crate::preserves_rail::to_text(&delivered.ingress_receipt_value).expect("receipt text");
    assert!(receipt_text.contains(case.expected), "{} receipt: {receipt_text}", case.name);
    assert!(super::super::leases::next_pending_control_request(&root).expect("pending request scan").is_none());
}

#[test]
fn control_live_authority_delegation_fails_closed() {
    for case in super::denycases::DENY_CASES {
        assert_denied_case(case);
    }
}

#[test]
fn control_live_transport_receipts_do_not_bootstrap_or_authorize() {
    let root = super::fixtures::temp_dir("node-control-live-transport-is-not-authority");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:transport-proof",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let policy_refs =
        vec![super::super::storage::local_ref("node-control-policy", "transport-proof").expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "transport-proof").expect("resource ref")];
    let valid_authority_refs =
        super::test_live_authority_refs(&root, "peer:transport", "node:transport-proof", "status", &policy_refs)
            .expect("authority grant ref");
    let transport_ref =
        super::probes::transport_receipt_ref(&root, &policy_refs, &resource_refs, &valid_authority_refs);

    super::probes::assert_transport_ref_does_not_bootstrap(
        &root,
        &policy_refs,
        &resource_refs,
        &valid_authority_refs,
        &transport_ref,
    );
    super::probes::assert_transport_ref_does_not_authorize(&root, &policy_refs, &resource_refs, &transport_ref);
}
