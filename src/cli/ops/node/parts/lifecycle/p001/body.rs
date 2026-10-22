
fn serve_live(
    input: super::command::base::Serve,
    supervisor_policy_value: Option<&preserves::IOValue>,
) -> molten::error::Result<()> {
    let super::command::base::Serve {
        state_root,
        topic,
        max_requests_per_tick,
        live_max_events,
        live_event_timeout_ms,
        service_receipt_out,
        live_ticket_out,
        receipt_out,
        ..
    } = input;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(molten::error::Failure::from)?;
    let input = molten::node_daemon::ControlLiveServeInput {
        state_root: &state_root,
        topic: &topic,
        max_events: live_max_events,
        event_timeout_ms: live_event_timeout_ms,
        max_requests_per_tick,
        supervisor_policy_value,
    };
    let served = runtime.block_on(async {
        let bound = molten::node_daemon::bind_control_live_listener(&input).await?;
        // The ticket is published once the endpoint is bound, so a peer can
        // send while the listener waits. The rename makes it appear whole.
        if let Some(path) = live_ticket_out.as_ref() {
            super::core::write_file_replace(path, &molten::preserves_rail::to_text(bound.live_ticket_value())?)?;
        }
        println!("node serve live-iroh bound endpoint={}", bound.bound_endpoint_id());
        bound.serve().await
    })?;
    if let Some(path) = service_receipt_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&served.service.service_receipt_value)?)?;
    }
    super::core::emit_named_receipt(
        receipt_out.as_ref(),
        "node control live listener receipt",
        &served.listener_receipt_value,
    )?;
    println!(
        "node serve live-iroh listener={} service={} endpoint={} events={} transports={} processed={} stopped={}",
        served.listener_receipt_ref,
        served.service.service_receipt_ref,
        served.bound_endpoint_id,
        served.observed_events,
        served.transport_receipt_refs.len(),
        served.service.processed_request_refs.len(),
        if served.service.has_stopped { "yes" } else { "no" }
    );
    Ok(())
}

pub(crate) fn status(input: super::command::base::Status) -> molten::error::Result<()> {
    let super::command::base::Status {
        state_root,
        health_out,
        receipt_out,
    } = input;
    let status = molten::node_daemon::status_local(&molten::node_daemon::StatusInput {
        state_root: &state_root,
    })?;
    if let Some(path) = health_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&status.health_value)?)?;
    }
    super::core::emit_named_receipt(receipt_out.as_ref(), "node control receipt", &status.control_receipt_value)?;
    println!(
        "node status {} health={} control_receipt={}",
        status.status, status.health_ref, status.control_receipt_ref
    );
    Ok(())
}

pub(crate) fn control_history(input: super::command::base::History) -> molten::error::Result<()> {
    let history = molten::node_daemon::control_history_local(&molten::node_daemon::ControlHistoryInput {
        state_root: &input.state_root,
    })?;
    println!(
        "node control-history deliveries={} pending={} processed={}",
        history.deliveries.len(),
        history.pending_request_refs.len(),
        history.processed.len()
    );
    for delivery in &history.deliveries {
        println!(
            "delivery envelope={} request={} peer={} sequence={} decision={} phase={}",
            delivery.envelope_ref,
            delivery.request_ref,
            delivery.from_peer,
            delivery.sequence,
            delivery.decision,
            delivery.phase
        );
    }
    for request_ref in &history.pending_request_refs {
        println!("pending request={request_ref}");
    }
    for processed in &history.processed {
        println!(
            "processed request={} decision={} receipt={}",
            processed.request_ref, processed.decision, processed.receipt_ref
        );
    }
    Ok(())
}

pub(crate) fn stop(input: super::command::base::Stop) -> molten::error::Result<()> {
    let super::command::base::Stop {
        state_root,
        shutdown_out,
        receipt_out,
    } = input;
    let stop = molten::node_daemon::stop_local(&molten::node_daemon::StopInput {
        state_root: &state_root,
    })?;
    if let Some(path) = shutdown_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&stop.shutdown_value)?)?;
    }
    super::core::emit_named_receipt(receipt_out.as_ref(), "node control receipt", &stop.control_receipt_value)?;
    println!("node stop shutdown={} control_receipt={}", stop.shutdown_ref, stop.control_receipt_ref);
    Ok(())
}

pub(crate) fn show(input: super::command::base::Show) -> molten::error::Result<()> {
    let value = super::core::read_preserves_file(&input.artifact)?;
    println!("{}", molten::node_daemon::summary(&value)?);
    Ok(())
}
