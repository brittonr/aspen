
fn run_service_ticks(input: ServiceTickInput<'_>) -> Result<ServiceRunParts> {
    let mut run = ServiceRunParts {
        heartbeat_receipt_refs: Vec::with_capacity(input.tick_capacity),
        ingress_receipt_refs: Vec::with_capacity(input.event_capacity),
        loop_receipt_refs: Vec::with_capacity(input.tick_capacity),
        processed_request_refs: Vec::with_capacity(input.event_capacity),
        diagnostics: Vec::with_capacity(input.tick_capacity.saturating_mul(2)),
        ticks: 0,
        has_stopped: false,
    };

    for tick in 0..input.max_ticks {
        run.ticks = tick + 1;
        if run_service_tick(&input, &mut run, tick)? {
            break;
        }
    }
    if !run.has_stopped {
        match has_pending_service_work(input.state_root, input.topic) {
            Ok(true) => run.diagnostics.push("node control service reached max ticks with pending work".to_string()),
            Ok(false) => {}
            Err(error) => run.diagnostics.push(format!("node control service pending-work scan failed: {error}")),
        }
    }
    Ok(run)
}

fn run_service_tick(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts, tick: u64) -> Result<bool> {
    write_service_heartbeat(input, run, tick)?;
    if deliver_service_ingress(input, run)? {
        return Ok(true);
    }
    process_service_loop(input, run)
}
