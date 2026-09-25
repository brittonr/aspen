
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

fn live_workflow_required_steps(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if input.bundle.is_none() {
        diagnostics.push_item("node-control-live-workflow-bundle-missing".to_string());
    }
    if input.gate.is_none() {
        diagnostics.push_item("node-control-live-workflow-gate-missing".to_string());
    }
    if input.apply.is_none() {
        diagnostics.push_item("node-control-live-workflow-apply-missing".to_string());
    }
    if input.reconcile.is_none() {
        diagnostics.push_item("node-control-live-workflow-reconcile-missing".to_string());
    }
    if input.ack.is_none() {
        diagnostics.push_item("node-control-live-workflow-ack-missing".to_string());
    }
    if input.ack_import.is_none() && input.protocol_gate.is_none() {
        diagnostics.push_item("node-control-live-workflow-terminal-evidence-missing".to_string());
    }
}

fn live_workflow_step_decisions(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if let Some(gate) = input.gate
        && gate.decision != "pass"
    {
        diagnostics.push_item(format!("node-control-live-workflow-gate-decision-{}", gate.decision));
        diagnostics.extend_items(gate.diagnostics.iter().cloned());
    }
    if let Some(apply) = input.apply
        && apply.decision != "pass"
    {
        diagnostics.push_item(format!("node-control-live-workflow-apply-decision-{}", apply.decision));
        diagnostics.extend_items(apply.diagnostics.iter().cloned());
    }
    if let Some(reconcile) = input.reconcile
        && reconcile.decision != "pass"
    {
        diagnostics.push_item(format!(
            "node-control-live-workflow-reconcile-decision-{}",
            reconcile.decision
        ));
        diagnostics.extend_items(reconcile.diagnostics.iter().cloned());
    }
    if let Some(ack) = input.ack {
        if ack.receiver_decision != "pass" {
            diagnostics.push_item(format!("node-control-live-workflow-ack-receiver-decision-{}", ack.receiver_decision));
            diagnostics.extend_items(ack.receiver_diagnostics.iter().cloned());
        }
        diagnostics.extend_items(ack.diagnostics.iter().cloned());
    }
    if let Some(ack_import) = input.ack_import
        && ack_import.decision != "pass"
    {
        diagnostics.push_item(format!(
            "node-control-live-workflow-ack-import-decision-{}",
            ack_import.decision
        ));
        diagnostics.extend_items(ack_import.diagnostics.iter().cloned());
    }
    if let Some(protocol_gate) = input.protocol_gate
        && protocol_gate.decision != "pass"
    {
        diagnostics.push_item(format!(
            "node-control-live-workflow-protocol-gate-decision-{}",
            protocol_gate.decision
        ));
        diagnostics.extend_items(protocol_gate.diagnostics.iter().cloned());
    }
}

fn live_workflow_ordered_links(input: &LiveWorkflowLifecycleInput<'_>, diagnostics: &mut impl crate::bounded::VecSink<String>) {
    if let (Some(bundle), Some(gate)) = (input.bundle, input.gate)
        && gate.bundle_ref != bundle.bundle_ref
    {
        diagnostics.push_item("node-control-live-workflow-gate-bundle-mismatch".to_string());
    }
    if let (Some(bundle), Some(apply)) = (input.bundle, input.apply)
        && apply.bundle_ref != bundle.bundle_ref
    {
        diagnostics.push_item("node-control-live-workflow-apply-bundle-mismatch".to_string());
    }
    if let (Some(gate), Some(apply)) = (input.gate, input.apply) {
        if apply.gate_receipt_ref.as_deref() != Some(gate.receipt_ref.as_str()) {
            diagnostics.push_item("node-control-live-workflow-apply-gate-mismatch".to_string());
        }
        if apply.recomputed_verify_receipt_ref != gate.recomputed_verify_receipt_ref {
            diagnostics.push_item("node-control-live-workflow-apply-verify-mismatch".to_string());
        }
    }
    if let (Some(apply), Some(reconcile)) = (input.apply, input.reconcile) {
        if reconcile.apply_receipt_ref != apply.receipt_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-apply-mismatch".to_string());
        }
        if reconcile.bundle_ref != apply.bundle_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-bundle-mismatch".to_string());
        }
        if reconcile.send_receipt_ref != apply.send_receipt_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-send-mismatch".to_string());
        }
        if reconcile.envelope_ref != apply.envelope_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-envelope-mismatch".to_string());
        }
        if reconcile.operation_ref != apply.operation_ref {
            diagnostics.push_item("node-control-live-workflow-reconcile-operation-mismatch".to_string());
        }
    }
    if let (Some(reconcile), Some(ack)) = (input.reconcile, input.ack) {
        if ack.reconcile_receipt_ref != reconcile.receipt_ref {
            diagnostics.push_item("node-control-live-workflow-ack-reconcile-mismatch".to_string());
        }
        if ack.bundle_ref != reconcile.bundle_ref {
            diagnostics.push_item("node-control-live-workflow-ack-bundle-mismatch".to_string());
        }
        if ack.envelope_ref != reconcile.envelope_ref {
            diagnostics.push_item("node-control-live-workflow-ack-envelope-mismatch".to_string());
        }
        if ack.operation_ref != reconcile.operation_ref {
            diagnostics.push_item("node-control-live-workflow-ack-operation-mismatch".to_string());
        }
        if ack.request_ref != reconcile.request_ref {
            diagnostics.push_item("node-control-live-workflow-ack-request-mismatch".to_string());
        }
    }
    if let (Some(apply), Some(ack)) = (input.apply, input.ack)
        && ack.apply_receipt_ref != apply.receipt_ref
    {
        diagnostics.push_item("node-control-live-workflow-ack-apply-mismatch".to_string());
    }
    if let (Some(ack), Some(ack_import)) = (input.ack, input.ack_import) {
        if ack_import.ack_ref != ack.ack_ref {
            diagnostics.push_item("node-control-live-workflow-ack-import-ack-mismatch".to_string());
        }
        if ack_import.bundle_ref != ack.bundle_ref {
            diagnostics.push_item("node-control-live-workflow-ack-import-bundle-mismatch".to_string());
        }
    }
}
