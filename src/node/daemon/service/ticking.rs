//! Bounded service ticks: heartbeat, ingress delivery, and loop processing.

pub(super) fn run_service_ticks(
    input: super::serving::ServiceTickInput<'_>,
) -> crate::error::Result<super::serving::ServiceRunParts> {
    let mut run = super::serving::ServiceRunParts {
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

fn run_service_tick(
    input: &super::serving::ServiceTickInput<'_>,
    run: &mut super::serving::ServiceRunParts,
    tick: u64,
) -> crate::error::Result<bool> {
    write_service_heartbeat(input, run, tick)?;
    if deliver_service_ingress(input, run)? {
        return Ok(true);
    }
    process_service_loop(input, run)
}

fn write_service_heartbeat(
    input: &super::serving::ServiceTickInput<'_>,
    run: &mut super::serving::ServiceRunParts,
    tick: u64,
) -> crate::error::Result<()> {
    let heartbeat_value =
        super::services::service_heartbeat_receipt_value(&super::services::ServiceHeartbeatValueInput {
            startup_receipt_ref: input.startup_receipt_ref,
            service_lock_ref: input.service_lock_ref,
            tick,
            delivered_count: run.ingress_receipt_refs.len() as u64,
            processed_count: run.processed_request_refs.len() as u64,
            diagnostics: &run.diagnostics,
        })?;
    let heartbeat_ref = crate::preserves_rail::canonical_hash(&heartbeat_value)?;
    super::storage::write_preserves(
        input.state_root,
        &super::paths::control_service_heartbeat_path(&heartbeat_ref)?,
        &heartbeat_value,
    )?;
    super::storage::import_artifact(input.state_root, &heartbeat_value)?;
    run.heartbeat_receipt_refs.push(heartbeat_ref);
    Ok(())
}

fn deliver_service_ingress(
    input: &super::serving::ServiceTickInput<'_>,
    run: &mut super::serving::ServiceRunParts,
) -> crate::error::Result<bool> {
    let envelope_refs = match pending_ingress_envelope_refs(input.state_root, input.topic) {
        Ok(envelope_refs) => envelope_refs,
        Err(error) => {
            run.diagnostics.push(format!("node control service ingress scan failed: {error}"));
            return Ok(true);
        }
    };
    for envelope_ref in envelope_refs {
        let delivered =
            match super::delivery::deliver_control_ingress_with_root(input.state_root, input.topic, &envelope_ref) {
                Ok(delivered) => delivered,
                Err(error) => {
                    run.diagnostics
                        .push(format!("node control service ingress delivery {envelope_ref} failed: {error}"));
                    continue;
                }
            };
        let receipt = super::draining::ingress_receipt_decision(&delivered.ingress_receipt_value)?;
        if receipt != "pass" {
            run.diagnostics
                .push(format!("node control service ingress {} decision {}", delivered.envelope_ref, receipt));
        }
        run.ingress_receipt_refs.push(delivered.ingress_receipt_ref);
    }
    Ok(false)
}

fn process_service_loop(
    input: &super::serving::ServiceTickInput<'_>,
    run: &mut super::serving::ServiceRunParts,
) -> crate::error::Result<bool> {
    let lock_path = crate::node_state::RelativePath::parse(super::settings::CONTROL_LOCK_FILE)?;
    if !input.state_root.try_exists(&lock_path)? {
        super::tracking::note_service_shutdown_observation(input, run);
        return Ok(true);
    }
    let loop_run = match super::submission::run_control_loop_with_root(input.state_root, input.max_requests_per_tick) {
        Ok(loop_run) => loop_run,
        Err(error) => {
            run.diagnostics.push(format!("node control service loop failed: {error}"));
            return Ok(true);
        }
    };
    run.processed_request_refs.extend(loop_run.processed_request_refs.iter().cloned());
    if loop_run.is_denied {
        run.diagnostics
            .push(format!("node control service loop denied receipt={}", loop_run.loop_receipt_ref));
    }
    run.loop_receipt_refs.push(loop_run.loop_receipt_ref);
    if loop_run.has_stopped {
        run.has_stopped = true;
        return Ok(true);
    }
    if loop_run.is_denied {
        return Ok(true);
    }
    if !input.state_root.try_exists(&lock_path)? {
        super::tracking::note_service_shutdown_observation(input, run);
        return Ok(true);
    }
    Ok(false)
}

fn pending_ingress_envelope_refs(
    state_root: &crate::node_state::DirectoryRoot,
    topic: &str,
) -> crate::error::Result<Vec<String>> {
    let ingress = state_root.control_ingress()?;
    let topic = ingress.open_subdir(&crate::node_state::RelativePath::parse(topic)?)?;
    let entries = topic.list_entries()?;
    if entries.len() > super::settings::MAX_PENDING_CONTROL_REQUESTS {
        return Err(crate::error::Failure::invalid_harness("node control ingress pending envelope bound exceeded"));
    }
    let mut envelope_refs = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.kind != crate::node_state::EntryKind::RegularFile {
            continue;
        }
        let bytes = topic.read_entry(&entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
        let text = String::from_utf8(bytes).map_err(|error| {
            crate::error::Failure::invalid_harness(format!("node control ingress envelope is not UTF-8: {error}"))
        })?;
        let value = crate::preserves_rail::parse_text(&text)?;
        let envelope = super::delivery::parse_control_ingress_envelope(&value)?;
        if !state_root.try_exists(&super::paths::control_ingress_receipt_path(&envelope.envelope_ref, "deliver")?)? {
            envelope_refs.push(envelope.envelope_ref);
        }
    }
    Ok(envelope_refs)
}

fn has_pending_service_work(state_root: &crate::node_state::DirectoryRoot, topic: &str) -> crate::error::Result<bool> {
    if !pending_ingress_envelope_refs(state_root, topic)?.is_empty() {
        return Ok(true);
    }
    super::leases::next_pending_control_request(state_root).map(|pending| pending.is_some())
}
