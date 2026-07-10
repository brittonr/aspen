pub(crate) fn build(input: super::super::command::control::IngressBuild) -> molten::error::Result<()> {
    let super::super::command::control::IngressBuild {
        request,
        out,
        from_peer,
        to_node,
        topic,
        sequence,
        peer_bootstrap_refs,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs,
    } = input;
    let request_value = super::super::core::read_preserves_file(&request)?;
    let envelope = molten::node_daemon::control_ingress_envelope(&molten::node_daemon::ControlIngressEnvelopeInput {
        request_value: &request_value,
        from_peer: &from_peer,
        to_node: &to_node,
        topic: &topic,
        sequence,
        peer_bootstrap_refs: &peer_bootstrap_refs,
        authority_refs: &authority_refs,
        policy_refs: &policy_refs,
        resource_refs: &resource_refs,
        evidence_refs: &evidence_refs,
    })?;
    super::super::core::write_file(&out, &molten::preserves_rail::to_text(&envelope.value)?)?;
    println!(
        "node control ingress envelope={} request={} written to {}",
        envelope.envelope_ref,
        envelope.request.request_ref,
        out.display()
    );
    Ok(())
}

pub(crate) fn live_build(input: super::super::command::control::IngressLiveBuild) -> molten::error::Result<()> {
    let super::super::command::control::IngressLiveBuild {
        request,
        out,
        from_peer,
        to_node,
        topic,
        sequence,
        peer_bootstrap_refs,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs,
    } = input;
    let request_value = super::super::core::read_preserves_file(&request)?;
    let envelope =
        molten::node_daemon::control_live_ingress_envelope(&molten::node_daemon::ControlIngressEnvelopeInput {
            request_value: &request_value,
            from_peer: &from_peer,
            to_node: &to_node,
            topic: &topic,
            sequence,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &evidence_refs,
        })?;
    super::super::core::write_file(&out, &molten::preserves_rail::to_text(&envelope.value)?)?;
    println!(
        "node control live ingress envelope={} request={} written to {}",
        envelope.envelope_ref,
        envelope.request.request_ref,
        out.display()
    );
    Ok(())
}

pub(crate) fn live_loopback(input: super::super::command::control::IngressLiveLoopback) -> molten::error::Result<()> {
    let super::super::command::control::IngressLiveLoopback {
        state_root,
        request,
        from_peer,
        to_node,
        topic,
        sequence,
        peer_bootstrap_refs,
        authority_refs,
        policy_refs,
        resource_refs,
        evidence_refs,
        publish_receipt_out,
        receive_receipt_out,
    } = input;
    let request_value = super::super::core::read_preserves_file(&request)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(molten::error::MoltenError::from)?;
    let loopback = runtime.block_on(molten::node_daemon::control_live_iroh_loopback(
        &molten::node_daemon::ControlLiveLoopbackInput {
            state_root: &state_root,
            request_value: &request_value,
            from_peer: &from_peer,
            to_node: &to_node,
            topic: &topic,
            sequence,
            peer_bootstrap_refs: &peer_bootstrap_refs,
            authority_refs: &authority_refs,
            policy_refs: &policy_refs,
            resource_refs: &resource_refs,
            evidence_refs: &evidence_refs,
        },
    ))?;
    if let Some(path) = publish_receipt_out.as_ref() {
        super::super::core::write_file(path, &molten::preserves_rail::to_text(&loopback.publish_receipt_value)?)?;
    }
    super::super::core::emit_named_receipt(
        receive_receipt_out.as_ref(),
        "node control live transport receipt",
        &loopback.receive_receipt_value,
    )?;
    println!(
        "node control live ingress loopback envelope={} publish_receipt={} receive_receipt={} ingress_receipt={} enqueued={}",
        loopback.envelope_ref,
        loopback.publish_receipt_ref,
        loopback.receive_receipt_ref,
        loopback.ingress_receipt_ref,
        if loopback.has_enqueued { "yes" } else { "no" }
    );
    Ok(())
}

pub(crate) fn live_send(input: super::super::command::control::IngressLiveSend) -> molten::error::Result<()> {
    let values = LiveSendValues::read(&input)?;
    let sent = send_live(&input, &values)?;
    write_live_send_outputs(&input, &sent)?;
    Ok(())
}

struct LiveSendValues {
    request_value: preserves::IOValue,
    ticket_value: preserves::IOValue,
}

impl LiveSendValues {
    fn read(input: &super::super::command::control::IngressLiveSend) -> molten::error::Result<Self> {
        let request_value = super::super::core::read_preserves_file(&input.request)?;
        let ticket_value = super::super::core::read_preserves_file(&input.ticket)?;
        Ok(Self {
            request_value,
            ticket_value,
        })
    }
}

fn send_live(
    input: &super::super::command::control::IngressLiveSend,
    values: &LiveSendValues,
) -> molten::error::Result<molten::node_daemon::ControlLiveSend> {
    let ticket = molten::node_daemon::parse_control_live_ticket(&values.ticket_value)?;
    let default_profile_alpns = [molten::node_daemon::LIVE_CONTROL_INGRESS_TRANSPORT.to_string()];
    let profile_alpn_values = if input.topology_profile_alpns.is_empty() {
        default_profile_alpns.as_slice()
    } else {
        input.topology_profile_alpns.as_slice()
    };
    let profile_alpn_refs: Vec<&str> = profile_alpn_values.iter().map(String::as_str).collect();
    let topology_ticket_refs = vec![ticket.ticket_ref.clone()];
    let topology_peer_admission_refs = input.peer_bootstrap_refs.clone();
    let topology_profile =
        input.topology_profile_ref.as_deref().map(|profile_ref| molten::node_daemon::LiveTopologyProfile {
            profile_ref,
            expected_node: input.expected_node.as_deref().unwrap_or(&ticket.node_id),
            expected_peer: &input.from_peer,
            expected_topic: input.expected_topic.as_deref().unwrap_or(&ticket.topic),
            expected_endpoint: input.expected_endpoint.as_deref().or(Some(&ticket.live_endpoint_id)),
            allowed_alpns: &profile_alpn_refs,
            ticket_refs: &topology_ticket_refs,
            peer_admission_refs: &topology_peer_admission_refs,
            role: input.topology_profile_role.as_deref(),
        });
    let transport_profile =
        input.transport_profile_ref.as_deref().map(|profile_ref| molten::node_daemon::LiveTransportProfile {
            profile_ref,
            max_attempts: input.max_attempts,
            join_timeout_ms: input.join_timeout_ms,
            publish_timeout_ms: input.transport_profile_publish_timeout_ms.unwrap_or(input.join_timeout_ms),
            relay_preference: &input.transport_profile_relay,
        });
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(molten::error::MoltenError::from)?;
    runtime.block_on(molten::node_daemon::send_control_live_ingress(&molten::node_daemon::ControlLiveSendInput {
        state_root: input.state_root.as_deref(),
        request_value: &values.request_value,
        receiver_ticket_value: &values.ticket_value,
        from_peer: &input.from_peer,
        sequence: input.sequence,
        expected_operation_ref: input.operation_id.as_deref(),
        expected_receiver_node: input.expected_node.as_deref(),
        expected_topic: input.expected_topic.as_deref(),
        expected_endpoint: input.expected_endpoint.as_deref(),
        topology_profile: topology_profile.as_ref(),
        transport_profile: transport_profile.as_ref(),
        max_attempts: input.max_attempts,
        peer_bootstrap_refs: &input.peer_bootstrap_refs,
        authority_refs: &input.authority_refs,
        policy_refs: &input.policy_refs,
        resource_refs: &input.resource_refs,
        evidence_refs: &input.evidence_refs,
        join_timeout_ms: input.join_timeout_ms,
    }))
}

fn write_live_send_outputs(
    input: &super::super::command::control::IngressLiveSend,
    sent: &molten::node_daemon::ControlLiveSend,
) -> molten::error::Result<()> {
    write_optional_receipt(input.transport_receipt_out.as_ref(), sent.transport_receipt_value.as_ref())?;
    if let Some(dir) = input.retry_receipts_dir.as_ref() {
        std::fs::create_dir_all(dir).map_err(molten::error::MoltenError::from)?;
        for (reference, value) in sent.retry_receipt_refs.iter().zip(sent.retry_receipt_values.iter()) {
            let path = dir.join(format!("{}.preserves", reference.replace(':', "-")));
            super::super::core::write_file(&path, &molten::preserves_rail::to_text(value)?)?;
        }
    }
    write_optional_receipt(input.duplicate_receipt_out.as_ref(), sent.duplicate_receipt_value.as_ref())?;
    super::super::core::emit_named_receipt(
        input.receipt_out.as_ref(),
        "node control live send receipt",
        &sent.send_receipt_value,
    )?;
    println!(
        "node control live ingress send envelope={} operation={} ticket={} endpoint={} transport_receipt={} send_receipt={} retries={} duplicate_receipt={}",
        sent.envelope_ref,
        sent.operation_ref,
        sent.receiver_ticket_ref,
        sent.receiver_endpoint_id,
        sent.transport_receipt_ref.as_deref().unwrap_or("none"),
        sent.send_receipt_ref,
        sent.retry_receipt_refs.len(),
        sent.duplicate_receipt_ref.as_deref().unwrap_or("none")
    );
    Ok(())
}

fn write_optional_receipt(
    path: Option<&std::path::PathBuf>,
    value: Option<&preserves::IOValue>,
) -> molten::error::Result<()> {
    if let (Some(path), Some(value)) = (path, value) {
        super::super::core::write_file(path, &molten::preserves_rail::to_text(value)?)?;
    }
    Ok(())
}

pub(crate) fn publish(input: super::super::command::control::IngressPublish) -> molten::error::Result<()> {
    let super::super::command::control::IngressPublish {
        state_root,
        envelope,
        receipt_out,
    } = input;
    let envelope_value = super::super::core::read_preserves_file(&envelope)?;
    let published = molten::node_daemon::publish_control_ingress(&molten::node_daemon::ControlIngressPublishInput {
        state_root: &state_root,
        envelope_value: &envelope_value,
    })?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control ingress receipt",
        &published.receipt_value,
    )?;
    println!(
        "node control ingress publish envelope={} receipt={} path={}",
        published.envelope_ref,
        published.receipt_ref,
        published.envelope_path.display()
    );
    Ok(())
}

pub(crate) fn deliver(input: super::super::command::control::IngressDeliver) -> molten::error::Result<()> {
    let super::super::command::control::IngressDeliver {
        state_root,
        topic,
        envelope_ref,
        receipt_out,
    } = input;
    let delivered = molten::node_daemon::deliver_control_ingress(&molten::node_daemon::ControlIngressDeliverInput {
        state_root: &state_root,
        topic: &topic,
        envelope_ref: &envelope_ref,
    })?;
    super::super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control ingress receipt",
        &delivered.ingress_receipt_value,
    )?;
    println!(
        "node control ingress deliver envelope={} request={} receipt={} enqueued={}",
        delivered.envelope_ref,
        delivered.request_ref,
        delivered.ingress_receipt_ref,
        if delivered.has_enqueued { "yes" } else { "no" }
    );
    Ok(())
}
