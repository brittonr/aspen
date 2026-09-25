
pub fn control_live_workflow_receipt(input: &ControlLiveWorkflowInput<'_>) -> Result<ControlLiveWorkflowReceipt> {
    let state_root = input.state_root.map(crate::node_state::NodeStateRoot::open).transpose()?;
    if let Some(path) = input.state_root {
        validate_state_root(path)?;
    }
    if let Some(state_root) = state_root.as_ref() {
        ensure_state_layout(state_root)?;
    }
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let admission = parse_control_live_peer_admission(input.peer_admission_value)?;
    let authority = parse_control_authority_grant(input.authority_grant_value)?;
    let send = parse_control_live_send_receipt(input.send_receipt_value)?;
    let service_receipt_ref = service_run_receipt_ref(input.service_receipt_value)?;
    let checks = FlowChecks {
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        send: &send,
        service_receipt_ref: &service_receipt_ref,
    };
    let mut diagnostics = Vec::with_capacity(input.receive_receipt_values.len().saturating_add(8));
    checks.note_bindings(&mut diagnostics);
    let refs = checks.collect_refs(input, &mut diagnostics)?;
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let receipt_value = live_workflow_receipt_value(&LiveWorkflowReceiptValueInput {
        decision,
        ticket: &ticket,
        admission: &admission,
        authority: &authority,
        send: &send,
        receive_receipt_refs: &refs.receive_receipt_refs,
        listener_receipt_ref: refs.listener_receipt_ref.as_deref(),
        service_receipt_ref: &service_receipt_ref,
        diagnostics: &diagnostics,
    })?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    if let Some(state_root) = state_root.as_ref() {
        import_flow_values(state_root, input, &receipt_ref, &receipt_value)?;
    }
    Ok(ControlLiveWorkflowReceipt {
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}
