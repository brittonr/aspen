//! Iroh loopback and service delivery tests.

#[tokio::test]
async fn control_live_iroh_loopback_delivers_to_durable_inbox() {
    // r[verify molten.node.cap_std_async_authority]
    let root =
        crate::test_support::process_workspace("node_control_live_iroh").expect("create isolated async node workspace");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:live-ingress",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let policy_refs =
        vec![super::super::storage::local_ref("node-control-policy", "live-ingress").expect("policy ref")];
    let authority_refs =
        super::test_live_authority_refs(&root, "peer:live", "node:live-ingress", "status", &policy_refs)
            .expect("authority grant ref");
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "live-ingress").expect("resource ref")];
    let peer_bootstrap_refs = super::test_live_peer_bootstrap_refs(
        &root,
        "peer:live",
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

    let live =
        super::super::publishing::control_live_iroh_loopback(&super::super::requests::ControlLiveLoopbackInput {
            state_root: &root,
            request_value: &request_value,
            from_peer: "peer:live",
            to_node: "node:live-ingress",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .await
        .expect("live loopback");
    assert!(live.has_enqueued);
    assert_eq!(crate::ledger::artifact_kind(&live.publish_receipt_value), "node-control-live-transport-receipt");
    assert_eq!(crate::ledger::artifact_kind(&live.receive_receipt_value), "node-control-live-transport-receipt");

    let served = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("serve live ingress");
    assert_eq!(served.decision, "pass");
    assert_eq!(served.processed_request_refs.len(), 1);
}

#[test]
fn control_service_delivers_ingress_and_dispatches_through_loop() {
    let root = super::fixtures::temp_dir("node-control-service-ingress");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:service-ingress",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    publish_status_ingress(&root);

    let served = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 4,
        supervisor_policy_value: None,
    })
    .expect("serve ingress");
    assert_eq!(
        served.decision,
        "pass",
        "{}",
        crate::preserves_rail::to_text(&served.service_receipt_value).expect("service receipt text")
    );
    assert_eq!(served.heartbeat_receipt_refs.len(), 1);
    assert_eq!(served.ingress_receipt_refs.len(), 1);
    assert_eq!(served.loop_receipt_refs.len(), 1);
    assert_eq!(served.processed_request_refs.len(), 1);
    assert_eq!(crate::ledger::artifact_kind(&served.service_receipt_value), "node-control-service-run-receipt");
    let state_root = crate::node_state::DirectoryRoot::open(&root).expect("open node state root");
    let control_value = super::super::storage::read_preserves(
        &state_root,
        &super::super::paths::control_outbox_receipt_path(&served.processed_request_refs[0])
            .expect("control receipt path"),
    )
    .expect("read served control receipt");
    let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse served control");
    assert_eq!(control.decision, "pass");
}

fn publish_status_ingress(root: &std::path::Path) {
    let authority_refs =
        vec![super::super::storage::local_ref("node-control-authority", "service-ingress").expect("authority ref")];
    let policy_refs =
        vec![super::super::storage::local_ref("node-control-policy", "service-ingress").expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "service-ingress").expect("resource ref")];
    let peer_bootstrap_refs =
        vec![super::super::storage::local_ref("peer-bootstrap", "peer:service").expect("bootstrap ref")];
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
    let envelope =
        super::super::ingresses::control_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:service",
            to_node: "node:service-ingress",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("ingress envelope");
    super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
        state_root: root,
        envelope_value: &envelope.value,
    })
    .expect("publish ingress");
}

#[test]
fn control_service_duplicate_lock_denies_before_side_effects() {
    let root = super::fixtures::temp_dir("node-control-service-duplicate");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:service-duplicate",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let state_root = crate::node_state::DirectoryRoot::open(&root).expect("open node state root");
    let startup = super::super::leases::current_startup_receipt(&state_root).expect("startup");
    let identity = crate::node_identity::parse_identity(
        &super::super::storage::read_preserves(
            &state_root,
            &crate::node_state::RelativePath::parse(super::super::settings::IDENTITY_FILE).expect("identity path"),
        )
        .expect("identity"),
    )
    .expect("parse identity");
    let service_run_ref =
        super::super::storage::local_ref("node-control-service-run", "already-active").expect("service run ref");
    let lock_value = super::super::services::service_lock_value(&super::super::services::ServiceLockValueInput {
        startup_receipt_ref: &startup.receipt_ref,
        node_id: &identity.node_id,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        service_run_ref: &service_run_ref,
    })
    .expect("service lock");
    super::super::storage::write_preserves(
        &state_root,
        &crate::node_state::RelativePath::parse(super::super::settings::CONTROL_SERVICE_LOCK_FILE)
            .expect("service lock path"),
        &lock_value,
    )
    .expect("write service lock");
    let request = super::super::running::status_request().expect("status request");
    super::super::submission::submit_control_request(&super::super::requests::ControlSubmitInput {
        state_root: &root,
        request_value: &request.value,
    })
    .expect("submit pending request");

    let served = super::super::serving::serve_control(&super::super::requests::ControlServeInput {
        state_root: &root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        max_ticks: 1,
        max_requests_per_tick: 1,
        supervisor_policy_value: None,
    })
    .expect("duplicate service denial");
    assert_eq!(served.decision, "deny");
    assert_eq!(served.ticks, 0);
    assert!(served.processed_request_refs.is_empty());
    assert!(super::super::leases::next_pending_control_request(&root).expect("pending scan").is_some());
    let text = crate::preserves_rail::to_text(&served.service_receipt_value).expect("service receipt text");
    assert!(text.contains("already active"));
}
