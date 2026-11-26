//! Live send orchestration and its finish step.

pub async fn send_control_live_ingress(
    input: &super::offers::ControlLiveSendInput<'_>,
) -> crate::error::Result<super::transits::ControlLiveSend> {
    let state_root = input.state_root.map(crate::node_state::DirectoryRoot::open).transpose()?;
    send_control_live_ingress_with_root(input, state_root.as_ref()).await
}

pub(super) async fn send_control_live_ingress_with_root(
    input: &super::offers::ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::DirectoryRoot>,
) -> crate::error::Result<super::transits::ControlLiveSend> {
    super::vetting::validate_send_input(input, state_root)?;
    let ticket = super::issuance::parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = super::vetting::send_envelope(input, &ticket)?;
    if let Some(operation_ref) = input.expected_operation_ref
        && operation_ref != envelope.operation_ref
    {
        let diagnostics = vec![format!(
            "node control live send operation-id {operation_ref} does not match derived {}",
            envelope.operation_ref
        )];
        return super::duplicates::denied_control_live_send_with_diagnostics(super::duplicates::DeniedLiveSendInput {
            input,
            state_root,
            ticket: &ticket,
            envelope,
            diagnostics,
            retry_receipt_refs: Vec::new(),
            retry_receipt_values: Vec::new(),
        });
    }
    let receiver_addr = match super::vetting::send_receiver_addr(input, state_root, &ticket, &envelope)? {
        Ok(addr) => addr,
        Err(diagnostics) => {
            return super::duplicates::denied_control_live_send_with_diagnostics(
                super::duplicates::DeniedLiveSendInput {
                    input,
                    state_root,
                    ticket: &ticket,
                    envelope,
                    diagnostics,
                    retry_receipt_refs: Vec::new(),
                    retry_receipt_values: Vec::new(),
                },
            );
        }
    };
    if let Some(state_root) = state_root
        && let Some(duplicate) = super::duplicates::duplicate_control_live_send(input, state_root, &ticket, &envelope)?
    {
        return Ok(duplicate);
    }
    let retries = super::backoff::publish_with_retries(input, state_root, &receiver_addr, &ticket, &envelope).await?;
    let Some(published) = retries.published else {
        return super::duplicates::denied_control_live_send_with_diagnostics(super::duplicates::DeniedLiveSendInput {
            input,
            state_root,
            ticket: &ticket,
            envelope,
            diagnostics: retries.diagnostics,
            retry_receipt_refs: retries.retry_receipt_refs,
            retry_receipt_values: retries.retry_receipt_values,
        });
    };
    finish_send(FinishSendInput {
        input,
        state_root,
        ticket: &ticket,
        envelope,
        published,
        retry_receipt_refs: retries.retry_receipt_refs,
        retry_receipt_values: retries.retry_receipt_values,
    })
}

#[derive(Debug)]
pub(super) struct SendRetryOutcome {
    pub(super) published: Option<super::transits::ControlLiveIngressPublish>,
    pub(super) retry_receipt_refs: Vec<String>,
    pub(super) retry_receipt_values: Vec<preserves::IOValue>,
    pub(super) diagnostics: Vec<String>,
}

#[derive(Debug)]
struct FinishSendInput<'a> {
    input: &'a super::offers::ControlLiveSendInput<'a>,
    state_root: Option<&'a crate::node_state::DirectoryRoot>,
    ticket: &'a super::entitlements::ControlLiveTicket,
    envelope: super::transits::ControlIngressEnvelope,
    published: super::transits::ControlLiveIngressPublish,
    retry_receipt_refs: Vec<String>,
    retry_receipt_values: Vec<preserves::IOValue>,
}

fn finish_send(input: FinishSendInput<'_>) -> crate::error::Result<super::transits::ControlLiveSend> {
    let send_receipt_value = super::sends::live_send_receipt_value(&super::sends::LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.input.from_peer,
        ticket: input.ticket,
        envelope: &input.envelope,
        transport_receipt_ref: Some(&input.published.transport_receipt_ref),
        topology_profile_ref: super::tuning::selected_topology_profile_ref(input.input),
        transport_profile_ref: super::tuning::selected_transport_profile_ref(input.input),
        effective_max_attempts: super::tuning::effective_live_send_max_attempts(input.input),
        effective_join_timeout_ms: super::tuning::effective_live_send_join_timeout_ms(input.input),
        diagnostics: &[],
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = input.state_root {
        super::storage::import_artifact(state_root, input.input.receiver_ticket_value)?;
        super::writers::write_ingress_envelope_and_verify(state_root, &input.ticket.topic, &input.envelope)?;
        super::storage::import_artifact(state_root, &input.envelope.value)?;
        super::storage::write_preserves(
            state_root,
            &super::paths::control_live_transport_receipt_path(&input.envelope.envelope_ref, "send")?,
            &input.published.transport_receipt_value,
        )?;
        super::storage::import_artifact(state_root, &input.published.transport_receipt_value)?;
        super::storage::write_preserves(
            state_root,
            &super::paths::control_live_send_receipt_path(&send_receipt_ref)?,
            &send_receipt_value,
        )?;
        super::storage::import_artifact(state_root, &send_receipt_value)?;
    }
    Ok(super::transits::ControlLiveSend {
        envelope_ref: input.envelope.envelope_ref,
        envelope_value: input.envelope.value,
        operation_ref: input.envelope.operation_ref,
        receiver_ticket_ref: input.ticket.ticket_ref.clone(),
        receiver_endpoint_id: input.ticket.live_endpoint_id.clone(),
        transport_receipt_ref: Some(input.published.transport_receipt_ref),
        transport_receipt_value: Some(input.published.transport_receipt_value),
        retry_receipt_refs: input.retry_receipt_refs,
        retry_receipt_values: input.retry_receipt_values,
        duplicate_receipt_ref: None,
        duplicate_receipt_value: None,
        send_receipt_ref,
        send_receipt_value,
    })
}
