//! Duplicate and denied live send receipts.

#[derive(Debug)]
pub(super) struct DeniedLiveSendInput<'a> {
    pub(super) input: &'a super::offers::ControlLiveSendInput<'a>,
    pub(super) state_root: Option<&'a crate::node_state::DirectoryRoot>,
    pub(super) ticket: &'a super::entitlements::ControlLiveTicket,
    pub(super) envelope: super::transits::ControlIngressEnvelope,
    pub(super) diagnostics: Vec<String>,
    pub(super) retry_receipt_refs: Vec<String>,
    pub(super) retry_receipt_values: Vec<preserves::IOValue>,
}

pub(super) fn duplicate_control_live_send(
    input: &super::offers::ControlLiveSendInput<'_>,
    state_root: &crate::node_state::DirectoryRoot,
    ticket: &super::entitlements::ControlLiveTicket,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<Option<super::transits::ControlLiveSend>> {
    let (transport_receipt_value, transport_receipt_ref, send_receipt_ref) =
        pass_send_receipt_refs(input, ticket, envelope)?;
    let send_path = super::paths::control_live_send_receipt_path(&send_receipt_ref)?;
    if !state_root.try_exists(&send_path)? {
        return Ok(None);
    }
    let prior_send_value = super::storage::read_preserves(state_root, &send_path)?;
    let prior_send = super::receipts::parse_control_live_send_receipt(&prior_send_value)?;
    if prior_send.receipt_ref != send_receipt_ref {
        return Err(crate::error::Failure::invalid_harness("node control live send prior receipt path is stale"));
    }
    if prior_send.decision != "pass" || prior_send.envelope_ref != envelope.envelope_ref {
        return Ok(None);
    }
    let diagnostics = vec![format!(
        "node control live send duplicate operation {} reused prior send receipt {send_receipt_ref}",
        envelope.operation_ref
    )];
    let duplicate_receipt_value =
        super::sends::live_send_duplicate_receipt_value(&super::sends::LiveSendDuplicateReceiptValueInput {
            from_peer: input.from_peer,
            ticket,
            envelope,
            prior_send_receipt_ref: &send_receipt_ref,
            diagnostics: &diagnostics,
        })?;
    let duplicate_receipt_ref = crate::preserves_rail::canonical_hash(&duplicate_receipt_value)?;
    super::storage::write_preserves(
        state_root,
        &super::paths::control_live_send_duplicate_receipt_path(&duplicate_receipt_ref)?,
        &duplicate_receipt_value,
    )?;
    super::storage::import_artifact(state_root, &duplicate_receipt_value)?;
    Ok(Some(super::transits::ControlLiveSend {
        envelope_ref: envelope.envelope_ref.clone(),
        envelope_value: envelope.value.clone(),
        operation_ref: envelope.operation_ref.clone(),
        receiver_ticket_ref: ticket.ticket_ref.clone(),
        receiver_endpoint_id: ticket.live_endpoint_id.clone(),
        transport_receipt_ref: Some(transport_receipt_ref),
        transport_receipt_value: Some(transport_receipt_value),
        retry_receipt_refs: Vec::new(),
        retry_receipt_values: Vec::new(),
        duplicate_receipt_ref: Some(duplicate_receipt_ref),
        duplicate_receipt_value: Some(duplicate_receipt_value),
        send_receipt_ref,
        send_receipt_value: prior_send_value,
    }))
}

/// The transport receipt and the send receipt ref that a passing first send of this envelope would
/// have recorded.
fn pass_send_receipt_refs(
    input: &super::offers::ControlLiveSendInput<'_>,
    ticket: &super::entitlements::ControlLiveTicket,
    envelope: &super::transits::ControlIngressEnvelope,
) -> crate::error::Result<(preserves::IOValue, String, String)> {
    let transport_receipt_value =
        super::transports::live_transport_receipt_value(&super::transports::LiveTransportReceiptValueInput {
            operation: "publish",
            decision: "pass",
            node_id: input.from_peer,
            delivered_from: None,
            envelope,
            ingress_receipt_ref: None,
            topology_profile_ref: super::tuning::selected_topology_profile_ref(input),
            transport_profile_ref: super::tuning::selected_transport_profile_ref(input),
            effective_max_attempts: Some(super::tuning::effective_live_send_max_attempts(input)),
            effective_join_timeout_ms: Some(super::tuning::effective_live_send_join_timeout_ms(input)),
            diagnostics: &[],
        })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&transport_receipt_value)?;
    let send_receipt_value = super::sends::live_send_receipt_value(&super::sends::LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.from_peer,
        ticket,
        envelope,
        transport_receipt_ref: Some(&transport_receipt_ref),
        topology_profile_ref: super::tuning::selected_topology_profile_ref(input),
        transport_profile_ref: super::tuning::selected_transport_profile_ref(input),
        effective_max_attempts: super::tuning::effective_live_send_max_attempts(input),
        effective_join_timeout_ms: super::tuning::effective_live_send_join_timeout_ms(input),
        diagnostics: &[],
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    Ok((transport_receipt_value, transport_receipt_ref, send_receipt_ref))
}

pub(super) fn denied_control_live_send_with_diagnostics(
    denied: DeniedLiveSendInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveSend> {
    let send_receipt_value = super::sends::live_send_receipt_value(&super::sends::LiveSendReceiptValueInput {
        decision: "deny",
        from_peer: denied.input.from_peer,
        ticket: denied.ticket,
        envelope: &denied.envelope,
        transport_receipt_ref: None,
        topology_profile_ref: super::tuning::selected_topology_profile_ref(denied.input),
        transport_profile_ref: super::tuning::selected_transport_profile_ref(denied.input),
        effective_max_attempts: super::tuning::effective_live_send_max_attempts(denied.input),
        effective_join_timeout_ms: super::tuning::effective_live_send_join_timeout_ms(denied.input),
        diagnostics: &denied.diagnostics,
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = denied.state_root {
        super::storage::import_artifact(state_root, denied.input.receiver_ticket_value)?;
        super::writers::write_ingress_envelope_and_verify(state_root, &denied.ticket.topic, &denied.envelope)?;
        super::storage::import_artifact(state_root, &denied.envelope.value)?;
        super::storage::write_preserves(
            state_root,
            &super::paths::control_live_send_receipt_path(&send_receipt_ref)?,
            &send_receipt_value,
        )?;
        super::storage::import_artifact(state_root, &send_receipt_value)?;
    }
    Ok(super::transits::ControlLiveSend {
        envelope_ref: denied.envelope.envelope_ref,
        envelope_value: denied.envelope.value,
        operation_ref: denied.envelope.operation_ref,
        receiver_ticket_ref: denied.ticket.ticket_ref.clone(),
        receiver_endpoint_id: denied.ticket.live_endpoint_id.clone(),
        transport_receipt_ref: None,
        transport_receipt_value: None,
        retry_receipt_refs: denied.retry_receipt_refs,
        retry_receipt_values: denied.retry_receipt_values,
        duplicate_receipt_ref: None,
        duplicate_receipt_value: None,
        send_receipt_ref,
        send_receipt_value,
    })
}
