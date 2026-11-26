//! Ticket and grant import receipt tests.

struct ImportCase {
    sender: std::path::PathBuf,
    policy_refs: Vec<String>,
    ticket: super::super::entitlements::ControlLiveTicket,
    admission: super::super::entitlements::ControlLivePeerAdmission,
}

fn import_case() -> ImportCase {
    let receiver = super::fixtures::temp_dir("node-control-live-import-receiver");
    let sender = super::fixtures::temp_dir("node-control-live-import-sender");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &receiver,
        node_id: "node:live-import",
    })
    .expect("init receiver");
    super::super::running::run_local_fixture_for_tests(&super::super::requests::RunInput { state_root: &receiver })
        .expect("run receiver");
    super::super::bootstrapping::init_local(&super::super::requests::InitInput {
        state_root: &sender,
        node_id: "node:live-import-sender",
    })
    .expect("init sender");
    let policy_refs = vec![super::super::storage::local_ref("node-control-policy", "live-import").expect("policy ref")];
    let ticket =
        super::super::issuance::export_control_live_ticket(&super::super::offers::ControlLiveTicketExportInput {
            state_root: &receiver,
            topic: super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC,
            policy_refs: &policy_refs,
            evidence_refs: &[],
        })
        .expect("export ticket");
    let admission = super::super::peering::admit_control_live_peer(&super::super::offers::ControlLivePeerAdmitInput {
        state_root: &receiver,
        ticket_value: &ticket.value,
        peer_id: "peer:live-import",
        sequence: 1,
        expires_at: Some(4),
        policy_refs: &policy_refs,
        evidence_refs: &[],
    })
    .expect("admit peer");
    ImportCase {
        sender,
        policy_refs,
        ticket,
        admission,
    }
}

fn assert_ticket_imports(case: &ImportCase) {
    let import_input = super::super::offers::ControlLiveTicketImportInput {
        state_root: &case.sender,
        ticket_value: &case.ticket.value,
        peer_admission_value: Some(&case.admission.value),
        expected_node: Some("node:live-import"),
        expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
        expected_endpoint: Some(&case.ticket.live_endpoint_id),
        expected_peer: Some("peer:live-import"),
        as_of_sequence: 2,
    };
    let imported_ticket = super::super::importing::import_control_live_ticket(&import_input).expect("import ticket");
    assert_eq!(imported_ticket.decision, "pass");
    assert_eq!(imported_ticket.imported_refs.len(), 2);
    assert_eq!(
        crate::ledger::artifact_kind(&imported_ticket.receipt_value),
        "node-control-live-ticket-import-receipt"
    );
    super::super::storage::read_ledger_artifact(&case.sender, &case.ticket.ticket_ref).expect("ticket imported");
    super::super::storage::read_ledger_artifact(&case.sender, &case.admission.admission_ref)
        .expect("admission imported");
    assert!(super::super::issuance::parse_control_live_ticket(&imported_ticket.receipt_value).is_err());
    assert!(super::super::peering::parse_control_live_peer_admission(&imported_ticket.receipt_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&imported_ticket.receipt_value)
            .expect("ticket import receipt text")
            .contains("import-receipt-is-not-authority")
    );

    let stale_ticket =
        super::super::importing::import_control_live_ticket(&super::super::offers::ControlLiveTicketImportInput {
            as_of_sequence: 8,
            ..import_input
        })
        .expect("stale ticket import receipt");
    assert_eq!(stale_ticket.decision, "deny");
    assert!(stale_ticket.imported_refs.is_empty());
    assert!(stale_ticket.diagnostics.iter().any(|value| value.contains("expired at sequence")));

    let wrong_topic =
        super::super::importing::import_control_live_ticket(&super::super::offers::ControlLiveTicketImportInput {
            expected_topic: Some("wrong-topic"),
            ..import_input
        })
        .expect("wrong topic ticket import receipt");
    assert_eq!(wrong_topic.decision, "deny");
    assert!(wrong_topic.imported_refs.is_empty());
    assert!(wrong_topic.diagnostics.iter().any(|value| value.contains("wrong-topic")));

    let scope_input = super::super::entitlements::LiveTicketScopeInput {
        ticket: &case.ticket,
        admission: Some(&case.admission),
        expected_node: Some("node:live-import"),
        expected_topic: Some(super::super::settings::DEFAULT_CONTROL_INGRESS_TOPIC),
        expected_endpoint: Some(&case.ticket.live_endpoint_id),
        expected_peer: Some("peer:other-live-import"),
        as_of_sequence: 2,
        required_policy_refs: &case.policy_refs,
    };
    let wrong_peer = super::super::scoping::evaluate_live_ticket_scope(scope_input);
    assert_eq!(wrong_peer.decision, "deny");
    assert!(wrong_peer.diagnostics.iter().any(|value| value.contains("peer:other-live-import")));

    let wrong_policy =
        vec![super::super::storage::local_ref("node-control-policy", "wrong-ticket-policy").expect("wrong policy ref")];
    let policy_denied =
        super::super::scoping::evaluate_live_ticket_scope(super::super::entitlements::LiveTicketScopeInput {
            expected_peer: Some("peer:live-import"),
            required_policy_refs: &wrong_policy,
            ..scope_input
        });
    assert_eq!(policy_denied.decision, "deny");
    assert!(policy_denied.diagnostics.iter().any(|value| value.contains("missing required policy")));
}

fn assert_grant_imports(case: &ImportCase) {
    let operations = vec!["status".to_string()];
    let grant_value =
        super::super::issuance::control_authority_grant_value(&super::super::offers::ControlAuthorityGrantInput {
            peer_id: "peer:live-import",
            node_id: "node:live-import",
            operations: &operations,
            target_scope: "*",
            resource_scope: "*",
            epoch: 1,
            expires_at: Some(4),
            policy_refs: &case.policy_refs,
            revocation_refs: &[],
            evidence_refs: &[],
        })
        .expect("grant value");
    let imported_grant = super::super::importing::import_control_authority_grant_checked(
        &super::super::offers::ControlAuthorityGrantImportInput {
            state_root: &case.sender,
            grant_value: &grant_value,
            expected_peer: Some("peer:live-import"),
            expected_node: Some("node:live-import"),
            expected_operations: &operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        },
    )
    .expect("import grant");
    assert_eq!(imported_grant.decision, "pass");
    assert_eq!(imported_grant.imported_refs.len(), 1);
    assert_eq!(
        crate::ledger::artifact_kind(&imported_grant.receipt_value),
        "node-control-authority-grant-import-receipt"
    );
    super::super::storage::read_ledger_artifact(&case.sender, &imported_grant.grant_ref).expect("grant imported");
    assert!(super::super::issuance::parse_control_authority_grant(&imported_grant.receipt_value).is_err());
    assert!(
        crate::preserves_rail::to_text(&imported_grant.receipt_value)
            .expect("grant import receipt text")
            .contains("import-receipt-is-not-authority")
    );

    let bad_operations = vec!["shutdown".to_string()];
    let denied_grant = super::super::importing::import_control_authority_grant_checked(
        &super::super::offers::ControlAuthorityGrantImportInput {
            state_root: &case.sender,
            grant_value: &grant_value,
            expected_peer: Some("peer:live-import"),
            expected_node: Some("node:live-import"),
            expected_operations: &bad_operations,
            expected_target_scope: Some("*"),
            expected_resource_scope: Some("*"),
            as_of_epoch: 2,
        },
    )
    .expect("denied grant import");
    assert_eq!(denied_grant.decision, "deny");
    assert!(denied_grant.imported_refs.is_empty());
    assert!(denied_grant.diagnostics.iter().any(|value| value.contains("operation shutdown")));
}

#[test]
fn control_live_ticket_and_authority_import_receipts_gate_bindings() {
    let case = import_case();
    assert_ticket_imports(&case);
    assert_grant_imports(&case);
}
