pub(crate) fn init(input: super::command::base::Init) -> molten::error::Result<()> {
    let super::command::base::Init {
        state_root,
        node_id,
        config_out,
        identity_receipt_out,
    } = input;
    let init = molten::node_daemon::init_local_node(&molten::node_daemon::InitInput {
        state_root: &state_root,
        node_id: &node_id,
    })?;
    if let Some(path) = config_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&init.config_value)?)?;
    }
    if let Some(path) = identity_receipt_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&init.identity_receipt_value)?)?;
    }
    println!(
        "node init config={} identity={} identity_receipt={} state_root={}",
        init.config_ref,
        init.identity_ref,
        init.identity_receipt_ref,
        state_root.display()
    );
    Ok(())
}

pub(crate) fn run(input: super::command::base::Run) -> molten::error::Result<()> {
    let super::command::base::Run {
        state_root,
        startup_out,
    } = input;
    let run = molten::node_daemon::run_local_node(&molten::node_daemon::RunInput {
        state_root: &state_root,
    })?;
    if let Some(path) = startup_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&run.startup_value)?)?;
    }
    println!(
        "node run startup={} adapters={} state_root={}",
        run.startup_ref,
        run.adapter_receipt_refs.len(),
        state_root.display()
    );
    Ok(())
}

pub(crate) fn run_loop(input: super::command::base::RunLoop) -> molten::error::Result<()> {
    let super::command::base::RunLoop {
        state_root,
        max_requests,
        receipt_out,
        heartbeat_out,
    } = input;
    let loop_run = molten::node_daemon::run_control_loop(&molten::node_daemon::ControlLoopInput {
        state_root: &state_root,
        max_requests,
    })?;
    if let Some(path) = heartbeat_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&loop_run.heartbeat_receipt_value)?)?;
    }
    super::core::emit_named_receipt(receipt_out.as_ref(), "node control loop receipt", &loop_run.loop_receipt_value)?;
    println!(
        "node run-loop loop_receipt={} heartbeat={} processed={} stopped={}",
        loop_run.loop_receipt_ref,
        loop_run.heartbeat_receipt_ref,
        loop_run.processed_request_refs.len(),
        if loop_run.has_stopped { "yes" } else { "no" }
    );
    Ok(())
}

pub(crate) fn serve(input: super::command::base::Serve) -> molten::error::Result<()> {
    let super::command::base::Serve {
        state_root,
        topic,
        max_ticks,
        max_requests_per_tick,
        live_iroh,
        live_max_events,
        live_event_timeout_ms,
        service_receipt_out,
        live_ticket_out,
        supervisor_policy,
        receipt_out,
    } = input;
    let supervisor_policy_value =
        supervisor_policy.as_ref().map(|path| super::core::read_preserves_file(path)).transpose()?;
    if live_iroh {
        serve_live(
            super::command::base::Serve {
                state_root,
                topic,
                max_ticks,
                max_requests_per_tick,
                live_iroh,
                live_max_events,
                live_event_timeout_ms,
                service_receipt_out,
                live_ticket_out,
                supervisor_policy,
                receipt_out,
            },
            supervisor_policy_value.as_ref(),
        )
    } else {
        let served = molten::node_daemon::serve_node_control(&molten::node_daemon::ControlServeInput {
            state_root: &state_root,
            topic: &topic,
            max_ticks,
            max_requests_per_tick,
            supervisor_policy_value: supervisor_policy_value.as_ref(),
        })?;
        super::core::emit_named_receipt(
            receipt_out.as_ref(),
            "node control service run receipt",
            &served.service_receipt_value,
        )?;
        println!(
            "node serve decision={} receipt={} ticks={} heartbeats={} ingress={} loops={} processed={} stopped={}",
            served.decision,
            served.service_receipt_ref,
            served.ticks,
            served.heartbeat_receipt_refs.len(),
            served.ingress_receipt_refs.len(),
            served.loop_receipt_refs.len(),
            served.processed_request_refs.len(),
            if served.has_stopped { "yes" } else { "no" }
        );
        Ok(())
    }
}

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
        .map_err(molten::error::MoltenError::from)?;
    let served = runtime.block_on(molten::node_daemon::serve_node_control_live_listener(
        &molten::node_daemon::ControlLiveServeInput {
            state_root: &state_root,
            topic: &topic,
            max_events: live_max_events,
            event_timeout_ms: live_event_timeout_ms,
            max_requests_per_tick,
            supervisor_policy_value,
        },
    ))?;
    if let Some(path) = service_receipt_out.as_ref() {
        super::core::write_file(path, &molten::preserves_rail::to_text(&served.service.service_receipt_value)?)?;
    }
    if let Some(path) = live_ticket_out.as_ref()
        && let Some(ticket_value) = served.live_ticket_value.as_ref()
    {
        super::core::write_file(path, &molten::preserves_rail::to_text(ticket_value)?)?;
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
    let status = molten::node_daemon::status_local_node(&molten::node_daemon::StatusInput {
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

pub(crate) fn stop(input: super::command::base::Stop) -> molten::error::Result<()> {
    let super::command::base::Stop {
        state_root,
        shutdown_out,
        receipt_out,
    } = input;
    let stop = molten::node_daemon::stop_local_node(&molten::node_daemon::StopInput {
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
    println!("{}", molten::node_daemon::node_daemon_summary(&value)?);
    Ok(())
}
