
fn duplicate_control_live_send(
    input: &ControlLiveSendInput<'_>,
    state_root: &Path,
    ticket: &ControlLiveTicket,
    envelope: &ControlIngressEnvelope,
) -> Result<Option<ControlLiveSend>> {
    let transport_receipt_value = live_transport_receipt_value(&LiveTransportReceiptValueInput {
        operation: "publish",
        decision: "pass",
        node_id: input.from_peer,
        delivered_from: None,
        envelope,
        ingress_receipt_ref: None,
        topology_profile_ref: selected_topology_profile_ref(input),
        transport_profile_ref: selected_transport_profile_ref(input),
        effective_max_attempts: Some(effective_live_send_max_attempts(input)),
        effective_join_timeout_ms: Some(effective_live_send_join_timeout_ms(input)),
        diagnostics: &[],
    })?;
    let transport_receipt_ref = crate::preserves_rail::canonical_hash(&transport_receipt_value)?;
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.from_peer,
        ticket,
        envelope,
        transport_receipt_ref: Some(&transport_receipt_ref),
        topology_profile_ref: selected_topology_profile_ref(input),
        transport_profile_ref: selected_transport_profile_ref(input),
        effective_max_attempts: effective_live_send_max_attempts(input),
        effective_join_timeout_ms: effective_live_send_join_timeout_ms(input),
        diagnostics: &[],
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    let send_path = control_live_send_receipt_path(state_root, &send_receipt_ref);
    if !send_path.exists() {
        return Ok(None);
    }
    let prior_send_value = read_preserves(&send_path)?;
    let prior_send = parse_control_live_send_receipt(&prior_send_value)?;
    if prior_send.receipt_ref != send_receipt_ref {
        return Err(MoltenError::invalid_harness("node control live send prior receipt path is stale"));
    }
    if prior_send.decision != "pass" || prior_send.envelope_ref != envelope.envelope_ref {
        return Ok(None);
    }
    let diagnostics = vec![format!(
        "node control live send duplicate operation {} reused prior send receipt {send_receipt_ref}",
        envelope.operation_ref
    )];
    let duplicate_receipt_value = live_send_duplicate_receipt_value(&LiveSendDuplicateReceiptValueInput {
        from_peer: input.from_peer,
        ticket,
        envelope,
        prior_send_receipt_ref: &send_receipt_ref,
        diagnostics: &diagnostics,
    })?;
    let duplicate_receipt_ref = crate::preserves_rail::canonical_hash(&duplicate_receipt_value)?;
    write_preserves(
        &control_live_send_duplicate_receipt_path(state_root, &duplicate_receipt_ref),
        &duplicate_receipt_value,
    )?;
    import_artifact(state_root, &duplicate_receipt_value)?;
    Ok(Some(ControlLiveSend {
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

fn denied_control_live_send_with_diagnostics(denied: DeniedLiveSendInput<'_>) -> Result<ControlLiveSend> {
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "deny",
        from_peer: denied.input.from_peer,
        ticket: denied.ticket,
        envelope: &denied.envelope,
        transport_receipt_ref: None,
        topology_profile_ref: selected_topology_profile_ref(denied.input),
        transport_profile_ref: selected_transport_profile_ref(denied.input),
        effective_max_attempts: effective_live_send_max_attempts(denied.input),
        effective_join_timeout_ms: effective_live_send_join_timeout_ms(denied.input),
        diagnostics: &denied.diagnostics,
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = denied.input.state_root {
        import_artifact(state_root, denied.input.receiver_ticket_value)?;
        write_ingress_envelope_and_verify(state_root, &denied.ticket.topic, &denied.envelope)?;
        import_artifact(state_root, &denied.envelope.value)?;
        write_preserves(&control_live_send_receipt_path(state_root, &send_receipt_ref), &send_receipt_value)?;
        import_artifact(state_root, &send_receipt_value)?;
    }
    Ok(ControlLiveSend {
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

pub fn parse_control_live_send_receipt(value: &IoValue) -> Result<ControlLiveSendReceipt> {
    let fields = value
        .collect_simple_record("node-control-live-send-receipt-v1", Some(15))
        .or_else(|| value.collect_simple_record("node-control-live-send-receipt-v1", Some(13)))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-live-send-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_LIVE_SEND_RECEIPT_SCHEMA,
        "node control live send receipt",
    )?;
    let transport_receipt_ref = record_optional_string(&fields[10], "transport-receipt")?;
    if let Some(reference) = transport_receipt_ref.as_ref() {
        validate_ingress_ref(reference, "node control live send transport receipt ref")?;
    }
    Ok(ControlLiveSendReceipt {
        receipt_ref: crate::preserves_rail::canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        topic: record_string(&fields[3], "topic")?,
        from_peer: record_string(&fields[4], "from-peer")?,
        to_node: record_string(&fields[5], "to-node")?,
        receiver_ticket_ref: record_ref_string(&fields[6], "receiver-ticket")?,
        receiver_endpoint_id: record_string(&fields[7], "receiver-endpoint")?,
        receiver_address_refs: record_strings(&fields[8], "receiver-addresses")?,
        envelope_ref: record_ref_string(&fields[9], "envelope")?,
        transport_receipt_ref,
        diagnostics: record_strings(&fields[11], "diagnostics")?,
        value: value.clone(),
    })
}

struct FlowChecks<'a> {
    ticket: &'a ControlLiveTicket,
    admission: &'a ControlLivePeerAdmission,
    authority: &'a ControlAuthorityGrant,
    send: &'a ControlLiveSendReceipt,
    service_receipt_ref: &'a str,
}

struct FlowRefs {
    receive_receipt_refs: Vec<String>,
    listener_receipt_ref: Option<String>,
}

impl FlowChecks<'_> {
    fn note_bindings(&self, diagnostics: &mut impl VecSink<String>) {
        if self.admission.ticket_ref != self.ticket.ticket_ref {
            diagnostics.push_item("node control live workflow admission does not bind receiver ticket".to_string());
        }
        if self.admission.decision != "pass" {
            diagnostics.push_item(format!("node control live workflow admission decision {}", self.admission.decision));
        }
        if self.authority.peer_id != self.admission.peer_id {
            diagnostics
                .push_item("node control live workflow authority grant peer does not match admission".to_string());
        }
        if self.authority.node_id != self.ticket.node_id {
            diagnostics.push_item("node control live workflow authority grant node does not match ticket".to_string());
        }
        if self.send.receiver_ticket_ref != self.ticket.ticket_ref {
            diagnostics.push_item("node control live workflow send receipt does not bind receiver ticket".to_string());
        }
        if self.send.from_peer != self.admission.peer_id {
            diagnostics.push_item("node control live workflow send peer does not match admission".to_string());
        }
        if self.send.to_node != self.ticket.node_id || self.send.topic != self.ticket.topic {
            diagnostics.push_item("node control live workflow send destination does not match ticket".to_string());
        }
        if self.send.decision != "pass" {
            diagnostics.push_item(format!("node control live workflow send decision {}", self.send.decision));
        }
    }

    fn collect_refs(
        &self,
        input: &ControlLiveWorkflowInput<'_>,
        diagnostics: &mut impl VecSink<String>,
    ) -> Result<FlowRefs> {
        let mut receive_receipt_refs = Vec::with_capacity(input.receive_receipt_values.len());
        for receive_value in input.receive_receipt_values {
            let (receipt_ref, operation, envelope_ref) = live_transport_receipt_ref(receive_value)?;
            if operation != "receive" {
                diagnostics.push_item(format!(
                    "node control live workflow transport receipt operation {operation} is not receive"
                ));
            }
            if envelope_ref != self.send.envelope_ref {
                diagnostics
                    .push_item("node control live workflow receive envelope does not match send envelope".to_string());
            }
            receive_receipt_refs.push(receipt_ref);
        }
        if receive_receipt_refs.is_empty() {
            diagnostics.push_item("node control live workflow missing receive receipt".to_string());
        }
        let listener_receipt_ref = if let Some(listener_value) = input.listener_receipt_value {
            let (listener_ref, listener_transport_refs, listener_service_ref) =
                live_listener_receipt_refs(listener_value)?;
            for receive_ref in &receive_receipt_refs {
                if !listener_transport_refs.iter().any(|reference| reference == receive_ref) {
                    diagnostics
                        .push_item("node control live workflow listener does not bind receive receipt".to_string());
                }
            }
            if listener_service_ref != self.service_receipt_ref {
                diagnostics.push_item(
                    "node control live workflow listener service run does not match service receipt".to_string(),
                );
            }
            Some(listener_ref)
        } else {
            None
        };
        Ok(FlowRefs {
            receive_receipt_refs,
            listener_receipt_ref,
        })
    }
}

fn import_flow_values(
    state_root: &Path,
    input: &ControlLiveWorkflowInput<'_>,
    receipt_ref: &str,
    receipt_value: &IoValue,
) -> Result<()> {
    import_artifact(state_root, input.receiver_ticket_value)?;
    import_artifact(state_root, input.peer_admission_value)?;
    import_artifact(state_root, input.authority_grant_value)?;
    import_artifact(state_root, input.send_receipt_value)?;
    for receive_value in input.receive_receipt_values {
        import_artifact(state_root, receive_value)?;
    }
    if let Some(listener_value) = input.listener_receipt_value {
        import_artifact(state_root, listener_value)?;
    }
    import_artifact(state_root, input.service_receipt_value)?;
    write_preserves(&control_live_workflow_receipt_path(state_root, receipt_ref), receipt_value)?;
    import_artifact(state_root, receipt_value)?;
    Ok(())
}

pub fn control_live_workflow_receipt(input: &ControlLiveWorkflowInput<'_>) -> Result<ControlLiveWorkflowReceipt> {
    if let Some(state_root) = input.state_root {
        validate_state_root(state_root)?;
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
    if let Some(state_root) = input.state_root {
        import_flow_values(state_root, input, &receipt_ref, &receipt_value)?;
    }
    Ok(ControlLiveWorkflowReceipt {
        receipt_ref,
        receipt_value,
        decision: decision.to_string(),
        diagnostics,
    })
}
