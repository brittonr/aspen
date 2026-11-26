//! Ingress enqueue and materialized envelope tests.

#[test]
fn control_ingress_enqueues_once_and_preserves_provenance_gate() {
    let root = super::fixtures::temp_dir("node-control-ingress");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:ingress",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    let authority_refs =
        vec![super::super::storage::local_ref("node-control-authority", "ingress").expect("authority ref")];
    let policy_refs = vec![super::super::storage::local_ref("node-control-policy", "ingress").expect("policy ref")];
    let resource_refs =
        vec![super::super::storage::local_ref("node-control-resource", "ingress").expect("resource ref")];
    let peer_bootstrap_refs =
        vec![super::super::storage::local_ref("peer-bootstrap", "peer:operator").expect("bootstrap ref")];

    let payload_value =
        crate::preserves_rail::record("node-control-ingress-payload", vec![crate::preserves_rail::string(
            "missing-provenance",
        )]);
    let payload_ref = super::super::storage::import_artifact(&root, &payload_value).expect("import payload");
    let request_value = crate::node_runtime::control_request_value(&crate::node_runtime::ControlRequestValueInput {
        operation: "install",
        target_ref: None,
        payload_ref: Some(&payload_ref),
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &[],
    })
    .expect("install request");
    let envelope =
        super::super::ingresses::control_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: "peer:operator",
            to_node: "node:ingress",
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            sequence: 1,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &[],
        })
        .expect("ingress envelope");
    assert_enqueued_then_denied(&root, &envelope);
}

fn assert_enqueued_then_denied(root: &std::path::Path, envelope: &super::super::transits::ControlIngressEnvelope) {
    let published =
        super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
            state_root: root,
            envelope_value: &envelope.value,
        })
        .expect("publish ingress");
    assert_eq!(crate::ledger::artifact_kind(&published.receipt_value), "node-control-ingress-receipt");
    let delivered =
        super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
            state_root: root,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("deliver ingress");
    assert!(delivered.has_enqueued);
    assert!(delivered.queue_receipt_ref.is_some());

    let duplicate =
        super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
            state_root: root,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            envelope_ref: &envelope.envelope_ref,
        })
        .expect("duplicate ingress");
    assert!(!duplicate.has_enqueued);
    assert!(duplicate.idempotency_receipt_ref.is_some());

    let loop_result = super::super::submission::run_control_loop(&super::super::requests::ControlLoopInput {
        state_root: root,
        max_requests: 1,
    })
    .expect("dispatch ingress request");
    assert_eq!(loop_result.processed_request_refs.len(), 1);
    let state_root = crate::node_state::DirectoryRoot::open(root).expect("open node state root");
    let control_value = super::super::storage::read_preserves(
        &state_root,
        &super::super::paths::control_outbox_receipt_path(&delivered.request_ref).expect("outbox receipt path"),
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
    super::super::delivery::publish_control_ingress(&super::super::requests::ControlIngressPublishInput {
        state_root: &pair.root,
        envelope_value: &pair.first.value,
    })
    .expect("publish first");
    let state_root = crate::node_state::DirectoryRoot::open(&pair.root).expect("open node state root");
    super::super::storage::write_preserves(
        &state_root,
        &super::super::paths::control_ingress_envelope_path(
            super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            &pair.first.envelope_ref,
        )
        .expect("ingress envelope path"),
        &pair.second.value,
    )
    .expect("tamper materialized envelope");
    let denied = super::super::delivery::deliver_control_ingress(&super::super::requests::ControlIngressDeliverInput {
        state_root: &pair.root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        envelope_ref: &pair.first.envelope_ref,
    })
    .expect_err("materialized ref mismatch denied");
    assert!(denied.to_string().contains("materialized envelope ref"));
}

struct MaterializedIngressPair {
    root: std::path::PathBuf,
    first: super::super::transits::ControlIngressEnvelope,
    second: super::super::transits::ControlIngressEnvelope,
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

fn initialized_materialized_ingress_root() -> std::path::PathBuf {
    let root = super::fixtures::temp_dir("node-control-ingress-materialized-ref");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id: "node:ingress-materialized",
    })
    .expect("init node");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &root })
        .expect("run node");
    root
}

fn materialized_ingress_refs() -> MaterializedIngressRefs {
    MaterializedIngressRefs {
        authority_refs: vec![
            super::super::storage::local_ref("node-control-authority", "materialized").expect("authority ref"),
        ],
        policy_refs: vec![super::super::storage::local_ref("node-control-policy", "materialized").expect("policy ref")],
        resource_refs: vec![
            super::super::storage::local_ref("node-control-resource", "materialized").expect("resource ref"),
        ],
        peer_bootstrap_refs: vec![
            super::super::storage::local_ref("peer-bootstrap", "peer:materialized").expect("bootstrap ref"),
        ],
    }
}

fn materialized_request_value(root: &std::path::Path, refs: &MaterializedIngressRefs) -> preserves::IOValue {
    let payload_value =
        crate::preserves_rail::record("node-control-ingress-payload", vec![crate::preserves_rail::string(
            "materialized",
        )]);
    let payload_ref = super::super::storage::import_artifact(root, &payload_value).expect("import payload");
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
    request_value: &preserves::IOValue,
    refs: &MaterializedIngressRefs,
    sequence: u64,
) -> super::super::transits::ControlIngressEnvelope {
    super::super::ingresses::control_ingress_envelope(&super::super::requests::ControlIngressEnvelopeInput {
        request_value,
        from_peer: "peer:materialized",
        to_node: "node:ingress-materialized",
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        sequence,
        peer_bootstrap_refs: &refs.peer_bootstrap_refs,
        authority_refs: &refs.authority_refs,
        policy_refs: &refs.policy_refs,
        resource_refs: &refs.resource_refs,
        evidence_refs: &[],
    })
    .expect("materialized envelope")
}
