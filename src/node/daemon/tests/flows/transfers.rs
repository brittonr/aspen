//! Bundle import/export gates and bounded listener send tests.

#[test]
fn control_live_workflow_bundle_import_export_gates_bindings() {
    let case = super::seeding::flow_case();
    super::gatedenials::assert_flow_gate_denials(&case);
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("apply runtime");
    let applied = super::gatedenials::assert_flow_apply_pass(&case, &runtime);
    super::gatedenials::assert_flow_missing_gate(&case, &runtime);
    super::gatedenials::assert_flow_send_denial(&case, &runtime);
    super::mismatches::assert_flow_import_pass(&case);
    let stale_gate = super::mismatches::assert_flow_wrong_topic(&case);
    super::mismatches::assert_flow_stale_gate(&case, &runtime, &stale_gate);
    super::mismatches::assert_flow_wrong_peer(&case);
    super::mismatches::assert_flow_wrong_operation(&case);
    super::forgeries::assert_flow_wrong_grant(&case);
    super::forgeries::assert_flow_receipts_not_grants(&case, &applied);
    super::forgeries::assert_flow_malformed(&case);
}

fn assert_sent(sent: &super::super::transits::ControlLiveSend) {
    assert_eq!(crate::ledger::artifact_kind(&sent.send_receipt_value), "node-control-live-send-receipt");
    assert!(sent.transport_receipt_ref.is_some());
    assert_eq!(
        sent.operation_ref,
        super::super::delivery::parse_control_ingress_envelope(&sent.envelope_value)
            .expect("envelope")
            .operation_ref
    );
}

fn assert_duplicate(
    first: &super::super::transits::ControlLiveSend,
    duplicate: &super::super::transits::ControlLiveSend,
) {
    assert_eq!(duplicate.send_receipt_ref, first.send_receipt_ref);
    assert!(duplicate.duplicate_receipt_ref.is_some());
    assert_eq!(
        crate::ledger::artifact_kind(duplicate.duplicate_receipt_value.as_ref().expect("duplicate receipt")),
        "node-control-live-send-duplicate-receipt"
    );
}

struct ServedCase<'a> {
    root: &'a std::path::Path,
    authority_ref: &'a str,
    ticket_value: &'a preserves::IOValue,
    admission_value: &'a preserves::IOValue,
    send_receipt_value: &'a preserves::IOValue,
    listener: &'a super::super::transits::ControlLiveServe,
}

fn assert_served_case(case: ServedCase<'_>) {
    assert_eq!(case.listener.service.decision, "pass");
    assert_eq!(case.listener.service.processed_request_refs.len(), 1);
    assert_eq!(case.listener.transport_receipt_refs.len(), 1);
    assert!(case.listener.observed_events > 0);
    let authority_value =
        super::super::storage::read_ledger_artifact(case.root, case.authority_ref).expect("authority value");
    let receive_values = case
        .listener
        .transport_receipt_refs
        .iter()
        .map(|reference| {
            super::super::storage::read_ledger_artifact(case.root, reference).expect("receive receipt value")
        })
        .collect::<Vec<_>>();
    let receive_value_refs = receive_values.iter().collect::<Vec<_>>();
    let workflow =
        super::super::attestation::control_live_workflow_receipt(&super::super::bundling::ControlLiveWorkflowInput {
            state_root: Some(case.root),
            receiver_ticket_value: case.ticket_value,
            peer_admission_value: case.admission_value,
            authority_grant_value: &authority_value,
            send_receipt_value: case.send_receipt_value,
            receive_receipt_values: &receive_value_refs,
            listener_receipt_value: Some(&case.listener.listener_receipt_value),
            service_receipt_value: &case.listener.service.service_receipt_value,
        })
        .expect("workflow receipt");
    assert_eq!(workflow.decision, "pass");
    assert_eq!(crate::ledger::artifact_kind(&workflow.receipt_value), "node-control-live-workflow-receipt");
}

pub(super) struct SendMaterial {
    policy_refs: Vec<String>,
    resource_refs: Vec<String>,
    peer_bootstrap_refs: Vec<String>,
    authority_refs: Vec<String>,
    pub(super) admission: super::super::entitlements::ControlLivePeerAdmission,
    request_value: preserves::IOValue,
}

pub(super) fn init_send_case() -> (std::path::PathBuf, crate::node_identity::Identity) {
    let root = super::fixtures::temp_dir("node-control-live-send");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:live-send",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let state_root = crate::node_state::DirectoryRoot::open(&root).expect("open node state root");
    let identity = crate::node_identity::parse_identity(
        &super::super::storage::read_preserves(
            &state_root,
            &crate::node_state::RelativePath::parse(super::super::settings::IDENTITY_FILE).expect("identity path"),
        )
        .expect("identity"),
    )
    .expect("parse identity");
    (root, identity)
}

pub(super) fn send_material(
    root: &std::path::Path,
    ticket: &super::super::entitlements::ControlLiveTicket,
) -> SendMaterial {
    let policy_refs = vec![super::super::storage::local_ref("node-control-policy", "live-send").expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "live-send").expect("resource ref")];
    let admission = super::super::peering::admit_control_live_peer(&super::super::offers::ControlLivePeerAdmitInput {
        state_root: root,
        ticket_value: &ticket.value,
        peer_id: "peer:external-send",
        sequence: 1,
        expires_at: None,
        policy_refs: &policy_refs,
        evidence_refs: &[],
    })
    .expect("peer admission");
    let peer_bootstrap_refs = vec![admission.admission_ref.clone()];
    let authority_refs =
        super::test_live_authority_refs(root, "peer:external-send", "node:live-send", "status", &policy_refs)
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
    SendMaterial {
        policy_refs,
        resource_refs,
        peer_bootstrap_refs,
        authority_refs,
        admission,
        request_value,
    }
}

pub(super) fn build_send_input<'a>(
    root: &'a std::path::Path,
    ticket: &'a super::super::entitlements::ControlLiveTicket,
    material: &'a SendMaterial,
) -> super::super::offers::ControlLiveSendInput<'a> {
    super::super::offers::ControlLiveSendInput {
        state_root: Some(root),
        request_value: &material.request_value,
        receiver_ticket_value: &ticket.value,
        from_peer: "peer:external-send",
        sequence: 1,
        expected_operation_ref: None,
        expected_receiver_node: None,
        expected_topic: None,
        expected_endpoint: None,
        topology_profile: None,
        transport_profile: None,
        max_attempts: super::super::settings::DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS,
        peer_bootstrap_refs: &material.peer_bootstrap_refs,
        authority_refs: &material.authority_refs,
        policy_refs: &material.policy_refs,
        resource_refs: &material.resource_refs,
        evidence_refs: &[],
        join_timeout_ms: 10_000,
    }
}

fn build_listener_input(root: &std::path::Path) -> super::super::requests::ControlLiveServeInput<'_> {
    super::super::requests::ControlLiveServeInput {
        state_root: root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_events: 8,
        event_timeout_ms: 1_000,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    }
}

#[test]
fn send_reaches_bounded_listener() {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().expect("runtime");
    runtime.block_on(async {
        let (root, identity) = init_send_case();
        let state_root = crate::node_state::DirectoryRoot::open(&root).expect("open node state root");
        let lookup = iroh::address_lookup::memory::MemoryLookup::new();
        let receiver_endpoint = super::super::endpoints::live_gossip_endpoint(
            &lookup,
            Some(
                super::super::endpoints::stable_live_endpoint_secret(&state_root, &identity).expect("transport secret"),
            ),
        )
        .await
        .expect("receiver endpoint");
        let receiver_addr = receiver_endpoint.addr();
        let live_ticket = super::super::endpoints::live_ticket_for_bound_endpoint(
            &state_root,
            &identity,
            super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            &receiver_addr,
        )
        .expect("live ticket");
        lookup.add_endpoint_info(receiver_addr);
        let receiver_gossip = iroh_gossip::Gossip::builder().spawn(receiver_endpoint.clone());
        let receiver_router = iroh::protocol::Router::builder(receiver_endpoint)
            .accept(iroh_gossip::ALPN, receiver_gossip.clone())
            .spawn();
        let mut receiver_topic = receiver_gossip
            .subscribe(
                super::super::endpoints::control_live_topic_id(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
                Vec::new(),
            )
            .await
            .expect("receiver subscribe");
        let material = send_material(&root, &live_ticket);
        let send_input = build_send_input(&root, &live_ticket, &material);
        let sent = super::super::sending::send_control_live_ingress(&send_input).await.expect("live send");
        assert_sent(&sent);
        let duplicate =
            super::super::sending::send_control_live_ingress(&send_input).await.expect("duplicate live send");
        assert_duplicate(&sent, &duplicate);
        let listener_input = build_listener_input(&root);
        let listener = super::super::endpoints::serve_node_control_live_listener_with_topic(
            &state_root,
            &listener_input,
            &mut receiver_topic,
            super::super::endpoints::EndpointIdentityInput {
                node_id: &identity.node_id,
                logical_endpoint_id: &identity.endpoint_id,
                bound_endpoint_id: &live_ticket.live_endpoint_id,
            },
        )
        .await
        .expect("listener drain");
        receiver_router.shutdown().await.expect("receiver shutdown");
        assert_served_case(ServedCase {
            root: &root,
            authority_ref: &material.authority_refs[0],
            ticket_value: &live_ticket.value,
            admission_value: &material.admission.value,
            send_receipt_value: &sent.send_receipt_value,
            listener: &listener,
        });
    });
}
