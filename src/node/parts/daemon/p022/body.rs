
pub async fn send_control_live_ingress(input: &ControlLiveSendInput<'_>) -> Result<ControlLiveSend> {
    let state_root = input.state_root.map(crate::node_state::NodeStateRoot::open).transpose()?;
    send_control_live_ingress_with_root(input, state_root.as_ref()).await
}

async fn send_control_live_ingress_with_root(
    input: &ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::NodeStateRoot>,
) -> Result<ControlLiveSend> {
    validate_send_input(input, state_root)?;
    let ticket = parse_control_live_ticket(input.receiver_ticket_value)?;
    let envelope = send_envelope(input, &ticket)?;
    if let Some(operation_ref) = input.expected_operation_ref
        && operation_ref != envelope.operation_ref
    {
        let diagnostics = vec![format!(
            "node control live send operation-id {operation_ref} does not match derived {}",
            envelope.operation_ref
        )];
        return denied_control_live_send_with_diagnostics(DeniedLiveSendInput {
            input,
            state_root,
            ticket: &ticket,
            envelope,
            diagnostics,
            retry_receipt_refs: Vec::new(),
            retry_receipt_values: Vec::new(),
        });
    }
    let receiver_addr = match send_receiver_addr(input, state_root, &ticket, &envelope)? {
        Ok(addr) => addr,
        Err(diagnostics) => {
            return denied_control_live_send_with_diagnostics(DeniedLiveSendInput {
                input,
                state_root,
                ticket: &ticket,
                envelope,
                diagnostics,
                retry_receipt_refs: Vec::new(),
                retry_receipt_values: Vec::new(),
            });
        }
    };
    if let Some(state_root) = state_root
        && let Some(duplicate) = duplicate_control_live_send(input, state_root, &ticket, &envelope)?
    {
        return Ok(duplicate);
    }
    let retries = publish_with_retries(input, state_root, &receiver_addr, &ticket, &envelope).await?;
    let Some(published) = retries.published else {
        return denied_control_live_send_with_diagnostics(DeniedLiveSendInput {
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
struct SendRetryOutcome {
    published: Option<ControlLiveIngressPublish>,
    retry_receipt_refs: Vec<String>,
    retry_receipt_values: Vec<IoValue>,
    diagnostics: Vec<String>,
}

#[derive(Debug)]
struct FinishSendInput<'a> {
    input: &'a ControlLiveSendInput<'a>,
    state_root: Option<&'a crate::node_state::NodeStateRoot>,
    ticket: &'a ControlLiveTicket,
    envelope: ControlIngressEnvelope,
    published: ControlLiveIngressPublish,
    retry_receipt_refs: Vec<String>,
    retry_receipt_values: Vec<IoValue>,
}

fn validate_send_input(
    input: &ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::NodeStateRoot>,
) -> Result<()> {
    if let Some(path) = input.state_root {
        validate_state_root(path)?;
    }
    if let Some(state_root) = state_root {
        ensure_state_layout(state_root)?;
    }
    validate_node_id(input.from_peer)?;
    validate_live_send_timeout(input.join_timeout_ms)?;
    validate_live_send_attempts(input.max_attempts)?;
    if let Some(operation_ref) = input.expected_operation_ref {
        validate_ingress_ref(operation_ref, "node control live send operation id")?;
    }
    if let Some(node) = input.expected_receiver_node {
        validate_node_id(node)?;
    }
    if let Some(topic) = input.expected_topic {
        validate_node_id(topic)?;
    }
    if let Some(endpoint) = input.expected_endpoint {
        validate_node_id(endpoint)?;
    }
    if let Some(profile) = input.topology_profile {
        validate_live_topology_profile(profile)?;
    }
    if let Some(profile) = input.transport_profile {
        validate_live_transport_profile_shape(profile)?;
    }
    Ok(())
}

fn send_envelope(input: &ControlLiveSendInput<'_>, ticket: &ControlLiveTicket) -> Result<ControlIngressEnvelope> {
    control_live_ingress_envelope(&ControlIngressEnvelopeInput {
        request_value: input.request_value,
        from_peer: input.from_peer,
        to_node: &ticket.node_id,
        topic: &ticket.topic,
        sequence: input.sequence,
        peer_bootstrap_refs: input.peer_bootstrap_refs,
        authority_refs: input.authority_refs,
        policy_refs: input.policy_refs,
        resource_refs: input.resource_refs,
        evidence_refs: input.evidence_refs,
    })
}

fn send_receiver_addr(
    input: &ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::NodeStateRoot>,
    ticket: &ControlLiveTicket,
    envelope: &ControlIngressEnvelope,
) -> Result<std::result::Result<iroh::EndpointAddr, Vec<String>>> {
    let mut diagnostics = live_send_ticket_diagnostics(input, ticket);
    let profile = live_send_profile_preflight(LiveProfilePreflightInput { send: input, ticket, envelope })?;
    diagnostics.extend(profile.diagnostics);
    if let Some(state_root) = state_root {
        diagnostics.extend(live_send_state_root_evidence_diagnostics(state_root, input, envelope)?);
    }
    if ticket.address_refs.is_empty() {
        diagnostics.push(
            "node control live send ticket has no endpoint addresses; import a bound live ticket with live-ticket-import or use serve --live-ticket-out"
                .to_string(),
        );
        return Ok(Err(diagnostics));
    }
    match live_ticket_endpoint_addr(ticket) {
        Ok(addr) if diagnostics.is_empty() => Ok(Ok(addr)),
        Ok(_) => Ok(Err(diagnostics)),
        Err(error) => {
            diagnostics.push(format!(
                "node control live send ticket address unsupported or malformed: {error}; import a fresh live ticket with live-ticket-import"
            ));
            Ok(Err(diagnostics))
        }
    }
}

async fn publish_with_retries(
    input: &ControlLiveSendInput<'_>,
    state_root: Option<&crate::node_state::NodeStateRoot>,
    receiver_addr: &iroh::EndpointAddr,
    ticket: &ControlLiveTicket,
    envelope: &ControlIngressEnvelope,
) -> Result<SendRetryOutcome> {
    let effective_max_attempts = effective_live_send_max_attempts(input);
    let attempt_capacity = usize::try_from(effective_max_attempts)
        .map_err(|_| MoltenError::invalid_harness("node control live send attempts exceed usize capacity"))?;
    let mut retry_receipt_refs = Vec::with_capacity(attempt_capacity);
    let mut retry_receipt_values = Vec::with_capacity(attempt_capacity);
    let mut diagnostics = Vec::with_capacity(attempt_capacity);
    let mut published = None;
    for attempt in 1..=effective_max_attempts {
        match attempt_control_live_send(input, receiver_addr, envelope).await? {
            Ok(receipt) => {
                published = Some(receipt);
                break;
            }
            Err(diagnostic) => {
                let attempt_diagnostics = vec![format!(
                    "node control live send attempt {attempt}/{} failed: {diagnostic}",
                    effective_max_attempts
                )];
                diagnostics.extend(attempt_diagnostics.iter().cloned());
                let retry_value = live_send_retry_receipt_value(&LiveSendRetryReceiptValueInput {
                    decision: if attempt == effective_max_attempts { "deny" } else { "fail" },
                    attempt,
                    max_attempts: effective_max_attempts,
                    from_peer: input.from_peer,
                    ticket,
                    envelope,
                    diagnostics: &attempt_diagnostics,
                })?;
                let retry_ref = crate::preserves_rail::canonical_hash(&retry_value)?;
                if let Some(state_root) = state_root {
                    write_preserves(state_root, &control_live_send_retry_receipt_path(&retry_ref)?, &retry_value)?;
                    import_artifact(state_root, &retry_value)?;
                }
                retry_receipt_refs.push(retry_ref);
                retry_receipt_values.push(retry_value);
            }
        }
    }
    Ok(SendRetryOutcome {
        published,
        retry_receipt_refs,
        retry_receipt_values,
        diagnostics,
    })
}

fn finish_send(input: FinishSendInput<'_>) -> Result<ControlLiveSend> {
    let send_receipt_value = live_send_receipt_value(&LiveSendReceiptValueInput {
        decision: "pass",
        from_peer: input.input.from_peer,
        ticket: input.ticket,
        envelope: &input.envelope,
        transport_receipt_ref: Some(&input.published.transport_receipt_ref),
        topology_profile_ref: selected_topology_profile_ref(input.input),
        transport_profile_ref: selected_transport_profile_ref(input.input),
        effective_max_attempts: effective_live_send_max_attempts(input.input),
        effective_join_timeout_ms: effective_live_send_join_timeout_ms(input.input),
        diagnostics: &[],
    })?;
    let send_receipt_ref = crate::preserves_rail::canonical_hash(&send_receipt_value)?;
    if let Some(state_root) = input.state_root {
        import_artifact(state_root, input.input.receiver_ticket_value)?;
        write_ingress_envelope_and_verify(state_root, &input.ticket.topic, &input.envelope)?;
        import_artifact(state_root, &input.envelope.value)?;
        write_preserves(
            state_root,
            &control_live_transport_receipt_path(&input.envelope.envelope_ref, "send")?,
            &input.published.transport_receipt_value,
        )?;
        import_artifact(state_root, &input.published.transport_receipt_value)?;
        write_preserves(state_root, &control_live_send_receipt_path(&send_receipt_ref)?, &send_receipt_value)?;
        import_artifact(state_root, &send_receipt_value)?;
    }
    Ok(ControlLiveSend {
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

async fn attempt_control_live_send(
    input: &ControlLiveSendInput<'_>,
    receiver_addr: &iroh::EndpointAddr,
    envelope: &ControlIngressEnvelope,
) -> Result<std::result::Result<ControlLiveIngressPublish, String>> {
    let lookup = iroh::address_lookup::memory::MemoryLookup::new();
    lookup.add_endpoint_info(receiver_addr.clone());
    let sender_endpoint = match live_gossip_endpoint(&lookup, None).await {
        Ok(endpoint) => endpoint,
        Err(error) => return Ok(Err(format!("live Iroh sender endpoint failed: {error}"))),
    };
    lookup.add_endpoint_info(sender_endpoint.addr());
    let sender_gossip = iroh_gossip::Gossip::builder().spawn(sender_endpoint.clone());
    let sender_router = iroh::protocol::Router::builder(sender_endpoint)
        .accept(iroh_gossip::ALPN, sender_gossip.clone())
        .spawn();
    let topic_id = control_live_topic_id(&envelope.topic);
    let join_timeout = std::time::Duration::from_millis(effective_live_send_join_timeout_ms(input));
    let join_result =
        tokio::time::timeout(join_timeout, sender_gossip.subscribe_and_join(topic_id, vec![receiver_addr.id])).await;
    let mut result = match join_result {
        Err(_) => Err(format!(
            "live Iroh node control send timed out joining topic {} at endpoint {}",
            envelope.topic, receiver_addr.id
        )),
        Ok(Err(error)) => Err(format!(
            "live Iroh node control send join failed for topic {} endpoint {}: {error}",
            envelope.topic, receiver_addr.id
        )),
        Ok(Ok(sender_topic)) => {
            let (sender, _receiver_unused) = sender_topic.split();
            let published = publish_control_live_ingress(&ControlLiveIngressPublishInput {
                sender: &sender,
                envelope_value: &envelope.value,
                node_id: input.from_peer,
                topology_profile_ref: selected_topology_profile_ref(input),
                transport_profile_ref: selected_transport_profile_ref(input),
                effective_max_attempts: Some(effective_live_send_max_attempts(input)),
                effective_join_timeout_ms: Some(effective_live_send_join_timeout_ms(input)),
            })
            .await;
            if published.is_ok() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            published.map_err(|error| format!("live Iroh node control send publish failed: {error}"))
        }
    };
    if let Err(error) = sender_router.shutdown().await {
        let diagnostic = format!("live Iroh sender router shutdown failed: {error}");
        if result.is_ok() {
            return Ok(Err(diagnostic));
        }
        result = result.map_err(|existing| format!("{existing}; {diagnostic}"));
    }
    Ok(result)
}
