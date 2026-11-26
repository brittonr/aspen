//! Live flow fixtures: roots, tickets, grants, export, verify, and gate.

struct FlowSeed {
    bundle_sender: std::path::PathBuf,
    operations: Vec<String>,
    ticket: super::super::entitlements::ControlLiveTicket,
    admission: super::super::entitlements::ControlLivePeerAdmission,
    authority_value: preserves::IOValue,
    receipt_values: Vec<preserves::IOValue>,
    authority_import_ref: String,
}

pub(super) struct FlowCase {
    pub(super) bundle_sender: std::path::PathBuf,
    pub(super) operations: Vec<String>,
    pub(super) ticket: super::super::entitlements::ControlLiveTicket,
    pub(super) admission: super::super::entitlements::ControlLivePeerAdmission,
    pub(super) authority_import_ref: String,
    pub(super) exported: super::super::artifacts::ControlLiveWorkflowBundleExport,
    pub(super) verified: super::super::artifacts::ControlLiveWorkflowBundleVerify,
    pub(super) gated: super::super::artifacts::ControlLiveWorkflowBundleGate,
}

struct FlowImports {
    receipt_values: Vec<preserves::IOValue>,
    authority_import_ref: String,
}

pub(super) struct FlowApplyInput<'a> {
    pub(super) state_root: &'a std::path::Path,
    pub(super) receipt_value: Option<&'a preserves::IOValue>,
    pub(super) request_value: Option<&'a preserves::IOValue>,
    pub(super) is_send_requested: bool,
    pub(super) sequence: u64,
    pub(super) expect_message: &'a str,
}

pub(super) fn init_flow_root(label: &str, node_id: &str) -> std::path::PathBuf {
    let root = super::fixtures::temp_dir(label);
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &root,
        node_id,
    })
    .expect("init flow root");
    root
}

fn flow_roots() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let receiver = init_flow_root("node-control-live-workflow-bundle-receiver", "node:live-bundle");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &receiver })
        .expect("run receiver");
    let staging = init_flow_root("node-control-live-workflow-bundle-staging", "node:live-bundle-staging");
    let sender = init_flow_root("node-control-live-workflow-bundle-sender", "node:live-bundle-sender");
    (receiver, staging, sender)
}

fn flow_ticket(root: &std::path::Path, policy_refs: &[String]) -> super::super::entitlements::ControlLiveTicket {
    super::super::issuance::export_control_live_ticket(&super::super::offers::ControlLiveTicketExportInput {
        state_root: root,
        topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
        policy_refs,
        evidence_refs: &[],
    })
    .expect("export ticket")
}

fn flow_admission(
    root: &std::path::Path,
    ticket: &super::super::entitlements::ControlLiveTicket,
    policy_refs: &[String],
) -> super::super::entitlements::ControlLivePeerAdmission {
    super::super::peering::admit_control_live_peer(&super::super::offers::ControlLivePeerAdmitInput {
        state_root: root,
        ticket_value: &ticket.value,
        peer_id: "peer:live-bundle",
        sequence: 1,
        expires_at: Some(8),
        policy_refs,
        evidence_refs: &[],
    })
    .expect("admit peer")
}

fn flow_authority_value(policy_refs: &[String], operations: &[String]) -> preserves::IOValue {
    super::super::issuance::control_authority_grant_value(&super::super::offers::ControlAuthorityGrantInput {
        peer_id: "peer:live-bundle",
        node_id: "node:live-bundle",
        operations,
        target_scope: "*",
        resource_scope: "*",
        epoch: 1,
        expires_at: Some(8),
        policy_refs,
        revocation_refs: &[],
        evidence_refs: &[],
    })
    .expect("authority grant value")
}

fn flow_imports(
    staging: &std::path::Path,
    ticket: &super::super::entitlements::ControlLiveTicket,
    admission: &super::super::entitlements::ControlLivePeerAdmission,
    authority_value: &preserves::IOValue,
    operations: &[String],
) -> FlowImports {
    let ticket_import =
        super::super::importing::import_control_live_ticket(&super::super::offers::ControlLiveTicketImportInput {
            state_root: staging,
            ticket_value: &ticket.value,
            peer_admission_value: Some(&admission.value),
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            as_of_sequence: 2,
        })
        .expect("ticket import");
    let authority_import = super::super::importing::import_control_authority_grant_checked(
        &super::super::offers::ControlAuthorityGrantImportInput {
            state_root: staging,
            grant_value: authority_value,
            expected_peer: Some("peer:live-bundle"),
            expected_node: Some("node:live-bundle"),
            expected_operations: operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        },
    )
    .expect("authority import");
    FlowImports {
        receipt_values: vec![ticket_import.receipt_value, authority_import.receipt_value],
        authority_import_ref: authority_import.grant_ref,
    }
}

fn flow_seed() -> FlowSeed {
    let (receiver, staging, bundle_sender) = flow_roots();
    let policy_refs = vec![super::super::storage::local_ref("node-control-policy", "live-bundle").expect("policy ref")];
    let operations = vec!["status".to_string()];
    let ticket = flow_ticket(&receiver, &policy_refs);
    let admission = flow_admission(&receiver, &ticket, &policy_refs);
    let authority_value = flow_authority_value(&policy_refs, &operations);
    let imports = flow_imports(&staging, &ticket, &admission, &authority_value, &operations);
    FlowSeed {
        bundle_sender,
        operations,
        ticket,
        admission,
        authority_value,
        receipt_values: imports.receipt_values,
        authority_import_ref: imports.authority_import_ref,
    }
}

fn export_flow(seed: &FlowSeed) -> super::super::artifacts::ControlLiveWorkflowBundleExport {
    let receipt_values = seed.receipt_values.iter().collect::<Vec<_>>();
    super::super::exporting::export_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleExportInput {
            receiver_ticket_value: &seed.ticket.value,
            peer_admission_value: &seed.admission.value,
            authority_grant_value: &seed.authority_value,
            receipt_values: &receipt_values,
        },
    )
    .expect("export bundle")
}

fn verify_flow(
    seed: &FlowSeed,
    exported: &super::super::artifacts::ControlLiveWorkflowBundleExport,
) -> super::super::artifacts::ControlLiveWorkflowBundleVerify {
    super::super::verifying::verify_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleVerifyInput {
            bundle_value: &exported.bundle.bundle_value,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&seed.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &seed.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("verify bundle")
}

fn gate_flow(
    seed: &FlowSeed,
    exported: &super::super::artifacts::ControlLiveWorkflowBundleExport,
    verified: &super::super::artifacts::ControlLiveWorkflowBundleVerify,
) -> super::super::artifacts::ControlLiveWorkflowBundleGate {
    super::super::verifying::gate_control_live_workflow_bundle(
        &super::super::bundling::ControlLiveWorkflowBundleGateInput {
            bundle_value: &exported.bundle.bundle_value,
            verify_receipt_value: Some(&verified.receipt_value),
            require_verify_receipt: true,
            expected_node: Some("node:live-bundle"),
            expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
            expected_endpoint: Some(&seed.ticket.live_endpoint_id),
            expected_peer: Some("peer:live-bundle"),
            expected_operations: &seed.operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_sequence: 2,
            as_of_epoch: 2,
        },
    )
    .expect("gate bundle")
}

pub(super) fn flow_case() -> FlowCase {
    let seed = flow_seed();
    let exported = export_flow(&seed);
    assert_eq!(exported.decision, "pass");
    assert_eq!(crate::ledger::artifact_kind(&exported.bundle.bundle_value), "node-control-live-workflow-bundle");
    assert!(super::super::issuance::parse_control_authority_grant(&exported.bundle.bundle_value).is_err());
    let verified = verify_flow(&seed, &exported);
    assert_eq!(verified.decision, "pass");
    assert_eq!(
        crate::ledger::artifact_kind(&verified.receipt_value),
        "node-control-live-workflow-bundle-verify-receipt"
    );
    assert!(super::super::issuance::parse_control_authority_grant(&verified.receipt_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&verified.receipt_value)
            .expect("verify receipt text")
            .contains("verify-receipt-is-not-authority")
    );
    let gated = gate_flow(&seed, &exported, &verified);
    assert_eq!(gated.decision, "pass");
    assert_eq!(crate::ledger::artifact_kind(&gated.receipt_value), "node-control-live-workflow-bundle-gate-receipt");
    assert_eq!(gated.verify_receipt_ref.as_deref(), Some(verified.receipt_ref.as_str()));
    assert!(super::super::issuance::parse_control_authority_grant(&gated.receipt_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&gated.receipt_value)
            .expect("gate receipt text")
            .contains("gate-receipt-is-not-authority")
    );
    FlowCase {
        bundle_sender: seed.bundle_sender,
        operations: seed.operations,
        ticket: seed.ticket,
        admission: seed.admission,
        authority_import_ref: seed.authority_import_ref,
        exported,
        verified,
        gated,
    }
}
