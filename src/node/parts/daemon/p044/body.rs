
fn live_workflow_expected_refs(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-bundle-mismatch",
        input.bundle.map(|bundle| bundle.bundle_ref.as_str()),
        input.expected_bundle_ref,
    );
    let envelope_ref = input.ack.and_then(|ack| ack.envelope_ref.as_deref()).or_else(|| {
        input
            .reconcile
            .and_then(|reconcile| reconcile.envelope_ref.as_deref())
            .or_else(|| input.apply.and_then(|apply| apply.envelope_ref.as_deref()))
    });
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-envelope-mismatch",
        envelope_ref,
        input.expected_envelope_ref,
    );
    let operation_ref = input.ack.and_then(|ack| ack.operation_ref.as_deref()).or_else(|| {
        input
            .reconcile
            .and_then(|reconcile| reconcile.operation_ref.as_deref())
            .or_else(|| input.apply.and_then(|apply| apply.operation_ref.as_deref()))
    });
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-operation-mismatch",
        operation_ref,
        input.expected_operation_ref,
    );
    let request_ref = input
        .ack
        .and_then(|ack| ack.request_ref.as_deref())
        .or_else(|| input.reconcile.and_then(|reconcile| reconcile.request_ref.as_deref()));
    live_workflow_note_expected(
        diagnostics,
        "node-control-live-workflow-expected-request-mismatch",
        request_ref,
        input.expected_request_ref,
    );
}

fn live_workflow_note_expected(
    diagnostics: &mut impl crate::bounded::VecSink<String>,
    diagnostic: &str,
    observed: Option<&str>,
    expected: Option<&str>,
) {
    if let Some(expected) = expected
        && observed != Some(expected)
    {
        diagnostics.push_item(format!("{diagnostic}:{} != {expected}", observed.unwrap_or("none")));
    }
}

pub fn evaluate_live_ticket_scope(input: LiveTicketScopeInput<'_>) -> LiveTicketScopeDecision {
    let mut diagnostics = Vec::with_capacity(LIVE_TICKET_SCOPE_DIAGNOSTIC_CAPACITY);
    if let Some(expected) = input.expected_node
        && input.ticket.node_id != expected
    {
        diagnostics.push(format!(
            "node control live ticket import node {} does not match expected {expected}",
            input.ticket.node_id
        ));
    }
    if let Some(expected) = input.expected_topic
        && input.ticket.topic != expected
    {
        diagnostics.push(format!(
            "node control live ticket import topic {} does not match expected {expected}",
            input.ticket.topic
        ));
    }
    if let Some(expected) = input.expected_endpoint
        && input.ticket.live_endpoint_id != expected
    {
        diagnostics.push(format!(
            "node control live ticket import endpoint {} does not match expected {expected}",
            input.ticket.live_endpoint_id
        ));
    }
    for required_policy_ref in input.required_policy_refs {
        if !input.ticket.policy_refs.iter().any(|policy_ref| policy_ref == required_policy_ref) {
            diagnostics.push(format!(
                "node control live ticket import missing required policy {required_policy_ref}"
            ));
        }
    }
    if let Some(admission) = input.admission {
        diagnostics.extend(live_ticket_admission_scope_diagnostics(input, admission));
    } else if let Some(expected) = input.expected_peer {
        diagnostics.push(format!(
            "node control live ticket import missing peer admission for expected peer {expected}"
        ));
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    LiveTicketScopeDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn live_ticket_import_diagnostics(
    input: &ControlLiveTicketImportInput<'_>,
    ticket: &ControlLiveTicket,
    admission: Option<&ControlLivePeerAdmission>,
) -> Vec<String> {
    evaluate_live_ticket_scope(LiveTicketScopeInput {
        ticket,
        admission,
        expected_node: input.expected_node,
        expected_topic: input.expected_topic,
        expected_endpoint: input.expected_endpoint,
        expected_peer: input.expected_peer,
        as_of_sequence: input.as_of_sequence,
        required_policy_refs: &[],
    })
    .diagnostics
}

fn live_ticket_admission_scope_diagnostics(
    input: LiveTicketScopeInput<'_>,
    admission: &ControlLivePeerAdmission,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(LIVE_TICKET_SCOPE_DIAGNOSTIC_CAPACITY);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.ticket_ref != input.ticket.ticket_ref {
        diagnostics.push(format!(
            "node control live peer admission {} ticket {} does not match ticket {}",
            admission.admission_ref, admission.ticket_ref, input.ticket.ticket_ref
        ));
    }
    if admission.node_id != input.ticket.node_id {
        diagnostics.push(format!(
            "node control live peer admission {} node {} does not match ticket node {}",
            admission.admission_ref, admission.node_id, input.ticket.node_id
        ));
    }
    if admission.topic != input.ticket.topic {
        diagnostics.push(format!(
            "node control live peer admission {} topic {} does not match ticket topic {}",
            admission.admission_ref, admission.topic, input.ticket.topic
        ));
    }
    for required_policy_ref in input.required_policy_refs {
        if !admission.policy_refs.iter().any(|policy_ref| policy_ref == required_policy_ref) {
            diagnostics.push(format!(
                "node control live peer admission {} missing required policy {required_policy_ref}",
                admission.admission_ref
            ));
        }
    }
    if let Some(expected) = input.expected_peer
        && admission.peer_id != expected
    {
        diagnostics.push(format!(
            "node control live peer admission {} peer {} does not match expected {expected}",
            admission.admission_ref, admission.peer_id
        ));
    }
    if admission.sequence > input.as_of_sequence {
        diagnostics.push(format!(
            "node control live peer admission {} is not valid until sequence {}",
            admission.admission_ref, admission.sequence
        ));
    }
    if let Some(expires_at) = admission.expires_at
        && expires_at < input.as_of_sequence
    {
        diagnostics.push(format!(
            "node control live peer admission {} expired at sequence {expires_at}",
            admission.admission_ref
        ));
    }
    diagnostics
}

fn authority_grant_import_diagnostics(
    input: &ControlAuthorityGrantImportInput<'_>,
    grant: &ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if let Some(expected) = input.expected_peer
        && grant.peer_id != expected
    {
        diagnostics.push(format!(
            "node control authority grant import peer {} does not match expected {expected}",
            grant.peer_id
        ));
    }
    if let Some(expected) = input.expected_node
        && grant.node_id != expected
    {
        diagnostics.push(format!(
            "node control authority grant import node {} does not match expected {expected}",
            grant.node_id
        ));
    }
    for operation in input.expected_operations {
        if !grant.operations.iter().any(|candidate| candidate == "*" || candidate == operation) {
            diagnostics.push(format!("node control authority grant import does not allow operation {operation}"));
        }
    }
    if let Some(expected) = input.expected_target_scope
        && grant.target_scope != "*"
        && grant.target_scope != expected
    {
        diagnostics.push(format!(
            "node control authority grant import target scope {} does not cover expected {expected}",
            grant.target_scope
        ));
    }
    if let Some(expected) = input.expected_resource_scope
        && grant.resource_scope != "*"
        && grant.resource_scope != expected
    {
        diagnostics.push(format!(
            "node control authority grant import resource scope {} does not cover expected {expected}",
            grant.resource_scope
        ));
    }
    if grant.epoch > input.as_of_epoch {
        diagnostics.push(format!("node control authority grant import is not valid until epoch {}", grant.epoch));
    }
    if let Some(expires_at) = grant.expires_at
        && expires_at < input.as_of_epoch
    {
        diagnostics.push(format!("node control authority grant import expired at epoch {expires_at}"));
    }
    if !grant.revocation_refs.is_empty() {
        diagnostics.push("node control authority grant import has revocation refs".to_string());
    }
    diagnostics
}

fn live_ticket_import_receipt_value(input: &LiveTicketImportReceiptValueInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    let binding_status = if input.decision == "pass" { "pass" } else { "fail" };
    Ok(crate::preserves_rail::record("node-control-live-ticket-import-receipt-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LIVE_TICKET_IMPORT_RECEIPT_SCHEMA),
        crate::preserves_rail::record("decision", vec![crate::preserves_rail::string(input.decision)]),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(&())?)]),
        crate::preserves_rail::record("ticket", vec![crate::preserves_rail::string(&input.ticket.ticket_ref)]),
        crate::preserves_rail::record("node", vec![crate::preserves_rail::string(&input.ticket.node_id)]),
        crate::preserves_rail::record("topic", vec![crate::preserves_rail::string(&input.ticket.topic)]),
        crate::preserves_rail::record("endpoint", vec![crate::preserves_rail::string(&input.ticket.live_endpoint_id)]),
        crate::preserves_rail::record("peer-admission", vec![optional_string(input.peer_admission_ref)]),
        crate::preserves_rail::record("peer", vec![optional_string(input.peer_id)]),
        crate::preserves_rail::record("as-of-sequence", vec![crate::preserves_rail::string(
            input.as_of_sequence.to_string(),
        )]),
        crate::preserves_rail::record("imported", vec![crate::preserves_rail::sequence(
            input.imported_refs.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("diagnostics", vec![crate::preserves_rail::sequence(
            input.diagnostics.iter().map(crate::preserves_rail::string).collect(),
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-kind-version"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("ticket-topic-endpoint-bound"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("peer-admission-kind-version"),
                crate::preserves_rail::string(binding_status),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("import-receipt-is-not-authority"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("provenance-still-required"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
        live_profile_ref_records(None, None),
        live_effective_transport_optional_record(None, None),
    ]))
}
