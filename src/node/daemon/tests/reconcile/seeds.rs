//! Reconcile fixtures: seeds, requests, and deliveries.

#[test]
fn control_live_workflow_bundle_reconcile_binds_receiver_evidence() {
    let case = super::verdicts::reconcile_case();
    let reconciled = super::verdicts::assert_reconcile_pass(&case);
    let denials = super::rejections::assert_reconcile_denials(&case);
    let ack = super::acceptance::assert_ack_pass(&case, &reconciled, &denials.wrong_envelope);
    super::refusals::assert_ack_denials(&case, &denials, &ack);
}

struct ReconcileSeed {
    root: std::path::PathBuf,
    policy_refs: Vec<String>,
    resource_refs: Vec<String>,
    peer_bootstrap_refs: Vec<String>,
    authority_refs: Vec<String>,
}

pub(super) struct ReconcileDelivery {
    pub(super) root: std::path::PathBuf,
    pub(super) request: crate::node_runtime::ControlRequest,
    pub(super) envelope: super::super::transits::ControlIngressEnvelope,
    pub(super) delivered: super::super::transits::ControlIngressDeliver,
    pub(super) queue_value: preserves::IOValue,
    pub(super) control_value: preserves::IOValue,
    pub(super) control_receipt_ref: String,
    pub(super) policy_refs: Vec<String>,
    pub(super) operations: Vec<String>,
}

pub(super) struct ReconcileCase {
    pub(super) delivery: ReconcileDelivery,
    pub(super) exported: super::super::artifacts::ControlLiveWorkflowBundleExport,
    pub(super) gated: super::super::artifacts::ControlLiveWorkflowBundleGate,
    pub(super) apply_receipt_value: preserves::IOValue,
}

pub(super) struct ReconcileDenials {
    pub(super) missing_receiver: super::super::artifacts::ControlLiveWorkflowBundleReconcile,
    pub(super) denied_control: preserves::IOValue,
    pub(super) denied_reconcile: super::super::artifacts::ControlLiveWorkflowBundleReconcile,
    pub(super) wrong_envelope: String,
}

pub(super) struct AckPass {
    pub(super) import_root: std::path::PathBuf,
}

pub(super) fn reconcile_expected<'a>(
    operations: &'a [String],
) -> super::super::intake::LiveWorkflowBundleExpectedInput<'a> {
    super::super::intake::LiveWorkflowBundleExpectedInput {
        expected_node: Some("node:reconcile"),
        expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
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
    let root = super::fixtures::temp_dir("node-control-live-workflow-reconcile");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:reconcile",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let policy_refs = vec![super::super::storage::local_ref("node-control-policy", "reconcile").expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "reconcile").expect("resource ref")];
    let peer_bootstrap_refs = super::test_live_peer_bootstrap_refs(
        &root,
        "peer:reconcile",
        super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        &policy_refs,
    )
    .expect("peer bootstrap");
    let authority_refs =
        super::test_live_authority_refs(&root, "peer:reconcile", "node:reconcile", "status", &policy_refs)
            .expect("authority refs");
    ReconcileSeed {
        root,
        policy_refs,
        resource_refs,
        peer_bootstrap_refs,
        authority_refs,
    }
}

fn reconcile_request(seed: &ReconcileSeed) -> (preserves::IOValue, crate::node_runtime::ControlRequest) {
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
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
    request_value: &preserves::IOValue,
) -> (super::super::transits::ControlIngressEnvelope, super::super::transits::ControlIngressDeliver) {
    let envelope =
        super::super::ingresses::control_live_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value,
            from_peer: "peer:reconcile",
            to_node: "node:reconcile",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &seed.peer_bootstrap_refs,
            authority_refs: &seed.authority_refs,
            policy_refs: &seed.policy_refs,
            resource_refs: &seed.resource_refs,
            evidence_refs: &[],
        })
        .expect("live envelope");
    super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
        state_root: &seed.root,
        envelope_value: &envelope.value,
    })
    .expect("publish envelope");
    let delivered =
        super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
            state_root: &seed.root,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
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

fn dispatched_reconcile(
    seed: &ReconcileSeed,
    delivered: &super::super::transits::ControlIngressDeliver,
) -> (preserves::IOValue, preserves::IOValue, String) {
    super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: &seed.root,
        max_requests: 1,
    })
    .expect("dispatch request");
    let state_root = crate::node_state::DirectoryRoot::open(&seed.root).expect("open node state root");
    let queue_value = super::super::storage::read_preserves(
        &state_root,
        &super::super::paths::queue_receipt_path(&delivered.request_ref).expect("queue receipt path"),
    )
    .expect("queue receipt");
    let control_value = super::super::storage::read_preserves(
        &state_root,
        &super::super::paths::control_outbox_receipt_path(&delivered.request_ref).expect("control receipt path"),
    )
    .expect("control receipt");
    let control = crate::node_runtime::parse_control_receipt(&control_value).expect("parse control");
    assert_eq!(control.decision, "pass");
    (queue_value, control_value, control.receipt_ref)
}

pub(super) fn reconcile_delivery() -> ReconcileDelivery {
    let seed = reconcile_seed();
    let (request_value, request) = reconcile_request(&seed);
    let (envelope, delivered) = deliver_reconcile_envelope(&seed, &request_value);
    let (queue_value, control_value, control_receipt_ref) = dispatched_reconcile(&seed, &delivered);
    ReconcileDelivery {
        root: seed.root,
        request,
        envelope,
        delivered,
        queue_value,
        control_value,
        control_receipt_ref,
        policy_refs: seed.policy_refs,
        operations: vec!["status".to_string()],
    }
}
