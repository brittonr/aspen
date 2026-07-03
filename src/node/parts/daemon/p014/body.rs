
const LIVE_TICKET_SCOPE_DIAGNOSTIC_CAPACITY: usize = 8;

fn authority_grant_expected_diagnostics(
    input: &LiveWorkflowBundleExpectedInput<'_>,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    authority_grant_import_diagnostics(
        &ControlAuthorityGrantImportInput {
            state_root: Path::new("."),
            grant_value: &authority.value,
            expected_peer: input.expected_peer,
            expected_node: input.expected_node,
            expected_operations: input.expected_operations,
            expected_target_scope: input.expected_target_scope,
            expected_resource_scope: input.expected_resource_scope,
            as_of_epoch: input.as_of_epoch,
        },
        authority,
    )
}

fn live_workflow_bundle_binding_diagnostics(
    ticket: &ControlLiveTicket,
    admission: &ControlLivePeerAdmission,
    authority: &ControlAuthorityGrant,
) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(8);
    if admission.decision != "pass" {
        diagnostics.push(format!(
            "node control live workflow bundle peer admission {} decision {}",
            admission.admission_ref, admission.decision
        ));
    }
    if admission.ticket_ref != ticket.ticket_ref {
        diagnostics.push("node control live workflow bundle admission does not bind ticket".to_string());
    }
    if admission.node_id != ticket.node_id {
        diagnostics.push("node control live workflow bundle admission node does not match ticket".to_string());
    }
    if admission.topic != ticket.topic {
        diagnostics.push("node control live workflow bundle admission topic does not match ticket".to_string());
    }
    if authority.peer_id != admission.peer_id {
        diagnostics.push("node control live workflow bundle authority peer does not match admission".to_string());
    }
    if authority.node_id != ticket.node_id {
        diagnostics.push("node control live workflow bundle authority node does not match ticket".to_string());
    }
    if !authority.revocation_refs.is_empty() {
        diagnostics.push("node control live workflow bundle authority grant has revocation refs".to_string());
    }
    diagnostics
}

fn live_workflow_bundle_receipt_refs(values: &[&IoValue]) -> Result<Vec<String>> {
    let owned_values = values.iter().map(|value| (**value).clone()).collect::<Vec<_>>();
    live_workflow_bundle_receipt_refs_from_values(&owned_values)
}

fn live_workflow_bundle_receipt_refs_from_values(values: &[IoValue]) -> Result<Vec<String>> {
    let mut refs = Vec::with_capacity(values.len());
    for value in values {
        refs.push(crate::preserves_rail::canonical_hash(value)?);
    }
    Ok(refs)
}

fn live_workflow_bundle_receipt_diagnostics(values: &[&IoValue]) -> Vec<String> {
    let mut diagnostics = Vec::with_capacity(values.len());
    for value in values {
        let kind = crate::ledger::artifact_kind(value);
        if !is_live_workflow_bundle_receipt_kind(kind) {
            diagnostics.push(format!("node control live workflow bundle unsupported receipt kind {kind}"));
        }
    }
    diagnostics
}

fn is_live_workflow_bundle_receipt_kind(kind: &str) -> bool {
    matches!(
        kind,
        "node-control-live-ticket-import-receipt"
            | "node-control-authority-grant-import-receipt"
            | "node-control-live-send-receipt"
            | "node-control-live-send-retry-receipt"
            | "node-control-live-send-duplicate-receipt"
            | "node-control-live-workflow-receipt"
            | "node-control-live-workflow-bundle-verify-receipt"
            | "node-control-live-workflow-bundle-gate-receipt"
            | "node-control-live-workflow-bundle-apply-receipt"
            | "node-control-live-workflow-bundle-reconcile-receipt"
            | "node-control-live-transport-receipt"
            | "node-control-live-listener-receipt"
            | "node-control-service-run-receipt"
    )
}

pub fn evaluate_live_workflow_lifecycle(input: LiveWorkflowLifecycleInput<'_>) -> LiveWorkflowLifecycleDecision {
    let mut diagnostics = Vec::with_capacity(LIVE_WORKFLOW_LIFECYCLE_DIAGNOSTIC_CAPACITY);
    live_workflow_required_steps(&input, &mut diagnostics);
    live_workflow_step_decisions(&input, &mut diagnostics);
    live_workflow_ordered_links(&input, &mut diagnostics);
    live_workflow_expected_refs(&input, &mut diagnostics);
    diagnostics.sort();
    diagnostics.dedup();
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    LiveWorkflowLifecycleDecision {
        decision: decision.to_string(),
        diagnostics,
    }
}

fn live_workflow_required_steps(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut Vec<String>) {
    if input.bundle.is_none() {
        diagnostics.push("node-control-live-workflow-bundle-missing".to_string());
    }
    if input.gate.is_none() {
        diagnostics.push("node-control-live-workflow-gate-missing".to_string());
    }
    if input.apply.is_none() {
        diagnostics.push("node-control-live-workflow-apply-missing".to_string());
    }
    if input.reconcile.is_none() {
        diagnostics.push("node-control-live-workflow-reconcile-missing".to_string());
    }
    if input.ack.is_none() {
        diagnostics.push("node-control-live-workflow-ack-missing".to_string());
    }
    if input.ack_import.is_none() && input.protocol_gate.is_none() {
        diagnostics.push("node-control-live-workflow-terminal-evidence-missing".to_string());
    }
}

fn live_workflow_step_decisions(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut Vec<String>) {
    if let Some(gate) = input.gate
        && gate.decision != "pass"
    {
        diagnostics.push(format!("node-control-live-workflow-gate-decision-{}", gate.decision));
        diagnostics.extend(gate.diagnostics.iter().cloned());
    }
    if let Some(apply) = input.apply
        && apply.decision != "pass"
    {
        diagnostics.push(format!("node-control-live-workflow-apply-decision-{}", apply.decision));
        diagnostics.extend(apply.diagnostics.iter().cloned());
    }
    if let Some(reconcile) = input.reconcile
        && reconcile.decision != "pass"
    {
        diagnostics.push(format!(
            "node-control-live-workflow-reconcile-decision-{}",
            reconcile.decision
        ));
        diagnostics.extend(reconcile.diagnostics.iter().cloned());
    }
    if let Some(ack) = input.ack {
        if ack.receiver_decision != "pass" {
            diagnostics.push(format!("node-control-live-workflow-ack-receiver-decision-{}", ack.receiver_decision));
            diagnostics.extend(ack.receiver_diagnostics.iter().cloned());
        }
        diagnostics.extend(ack.diagnostics.iter().cloned());
    }
    if let Some(ack_import) = input.ack_import
        && ack_import.decision != "pass"
    {
        diagnostics.push(format!(
            "node-control-live-workflow-ack-import-decision-{}",
            ack_import.decision
        ));
        diagnostics.extend(ack_import.diagnostics.iter().cloned());
    }
    if let Some(protocol_gate) = input.protocol_gate
        && protocol_gate.decision != "pass"
    {
        diagnostics.push(format!(
            "node-control-live-workflow-protocol-gate-decision-{}",
            protocol_gate.decision
        ));
        diagnostics.extend(protocol_gate.diagnostics.iter().cloned());
    }
}

fn live_workflow_ordered_links(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut Vec<String>) {
    if let (Some(bundle), Some(gate)) = (input.bundle, input.gate)
        && gate.bundle_ref != bundle.bundle_ref
    {
        diagnostics.push("node-control-live-workflow-gate-bundle-mismatch".to_string());
    }
    if let (Some(bundle), Some(apply)) = (input.bundle, input.apply)
        && apply.bundle_ref != bundle.bundle_ref
    {
        diagnostics.push("node-control-live-workflow-apply-bundle-mismatch".to_string());
    }
    if let (Some(gate), Some(apply)) = (input.gate, input.apply) {
        if apply.gate_receipt_ref.as_deref() != Some(gate.receipt_ref.as_str()) {
            diagnostics.push("node-control-live-workflow-apply-gate-mismatch".to_string());
        }
        if apply.recomputed_verify_receipt_ref != gate.recomputed_verify_receipt_ref {
            diagnostics.push("node-control-live-workflow-apply-verify-mismatch".to_string());
        }
    }
    if let (Some(apply), Some(reconcile)) = (input.apply, input.reconcile) {
        if reconcile.apply_receipt_ref != apply.receipt_ref {
            diagnostics.push("node-control-live-workflow-reconcile-apply-mismatch".to_string());
        }
        if reconcile.bundle_ref != apply.bundle_ref {
            diagnostics.push("node-control-live-workflow-reconcile-bundle-mismatch".to_string());
        }
        if reconcile.send_receipt_ref != apply.send_receipt_ref {
            diagnostics.push("node-control-live-workflow-reconcile-send-mismatch".to_string());
        }
        if reconcile.envelope_ref != apply.envelope_ref {
            diagnostics.push("node-control-live-workflow-reconcile-envelope-mismatch".to_string());
        }
        if reconcile.operation_ref != apply.operation_ref {
            diagnostics.push("node-control-live-workflow-reconcile-operation-mismatch".to_string());
        }
    }
    if let (Some(reconcile), Some(ack)) = (input.reconcile, input.ack) {
        if ack.reconcile_receipt_ref != reconcile.receipt_ref {
            diagnostics.push("node-control-live-workflow-ack-reconcile-mismatch".to_string());
        }
        if ack.bundle_ref != reconcile.bundle_ref {
            diagnostics.push("node-control-live-workflow-ack-bundle-mismatch".to_string());
        }
        if ack.envelope_ref != reconcile.envelope_ref {
            diagnostics.push("node-control-live-workflow-ack-envelope-mismatch".to_string());
        }
        if ack.operation_ref != reconcile.operation_ref {
            diagnostics.push("node-control-live-workflow-ack-operation-mismatch".to_string());
        }
        if ack.request_ref != reconcile.request_ref {
            diagnostics.push("node-control-live-workflow-ack-request-mismatch".to_string());
        }
    }
    if let (Some(apply), Some(ack)) = (input.apply, input.ack)
        && ack.apply_receipt_ref != apply.receipt_ref
    {
        diagnostics.push("node-control-live-workflow-ack-apply-mismatch".to_string());
    }
    if let (Some(ack), Some(ack_import)) = (input.ack, input.ack_import) {
        if ack_import.ack_ref != ack.ack_ref {
            diagnostics.push("node-control-live-workflow-ack-import-ack-mismatch".to_string());
        }
        if ack_import.bundle_ref != ack.bundle_ref {
            diagnostics.push("node-control-live-workflow-ack-import-bundle-mismatch".to_string());
        }
    }
}

fn live_workflow_expected_refs(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut Vec<String>) {
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
    diagnostics: &mut Vec<String>,
    diagnostic: &str,
    observed: Option<&str>,
    expected: Option<&str>,
) {
    if let Some(expected) = expected
        && observed != Some(expected)
    {
        diagnostics.push(format!("{diagnostic}:{} != {expected}", observed.unwrap_or("none")));
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
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            input.state_root,
        )?)]),
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
    ]))
}
