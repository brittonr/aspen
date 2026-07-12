
fn write_service_heartbeat(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts, tick: u64) -> Result<()> {
    let heartbeat_value = service_heartbeat_receipt_value(&ServiceHeartbeatValueInput {
        startup_receipt_ref: input.startup_receipt_ref,
        service_lock_ref: input.service_lock_ref,
        tick,
        delivered_count: run.ingress_receipt_refs.len() as u64,
        processed_count: run.processed_request_refs.len() as u64,
        diagnostics: &run.diagnostics,
    })?;
    let heartbeat_ref = crate::preserves_rail::canonical_hash(&heartbeat_value)?;
    write_preserves(
        input.state_root,
        &control_service_heartbeat_path(&heartbeat_ref)?,
        &heartbeat_value,
    )?;
    import_artifact(input.state_root, &heartbeat_value)?;
    run.heartbeat_receipt_refs.push(heartbeat_ref);
    Ok(())
}

fn deliver_service_ingress(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts) -> Result<bool> {
    let envelope_refs = match pending_ingress_envelope_refs(input.state_root, input.topic) {
        Ok(envelope_refs) => envelope_refs,
        Err(error) => {
            run.diagnostics.push(format!("node control service ingress scan failed: {error}"));
            return Ok(true);
        }
    };
    for envelope_ref in envelope_refs {
        let delivered = match deliver_control_ingress_with_root(input.state_root, input.topic, &envelope_ref) {
            Ok(delivered) => delivered,
            Err(error) => {
                run.diagnostics
                    .push(format!("node control service ingress delivery {envelope_ref} failed: {error}"));
                continue;
            }
        };
        let receipt = ingress_receipt_decision(&delivered.ingress_receipt_value)?;
        if receipt != "pass" {
            run.diagnostics
                .push(format!("node control service ingress {} decision {}", delivered.envelope_ref, receipt));
        }
        run.ingress_receipt_refs.push(delivered.ingress_receipt_ref);
    }
    Ok(false)
}

fn process_service_loop(input: &ServiceTickInput<'_>, run: &mut ServiceRunParts) -> Result<bool> {
    let lock_path = crate::node_state::NodeStatePath::parse(CONTROL_LOCK_FILE)?;
    if !input.state_root.try_exists(&lock_path)? {
        run.has_stopped = true;
        return Ok(true);
    }
    let loop_run = match run_control_loop_with_root(input.state_root, input.max_requests_per_tick) {
        Ok(loop_run) => loop_run,
        Err(error) => {
            run.diagnostics.push(format!("node control service loop failed: {error}"));
            return Ok(true);
        }
    };
    run.processed_request_refs.extend(loop_run.processed_request_refs.iter().cloned());
    run.loop_receipt_refs.push(loop_run.loop_receipt_ref);
    if loop_run.has_stopped || !input.state_root.try_exists(&lock_path)? {
        run.has_stopped = true;
        return Ok(true);
    }
    Ok(false)
}

fn note_shutdown_drain(input: ShutdownDrainInput<'_>) -> Result<ShutdownDrain> {
    let mut run = input.run;
    let mut supervisor_receipt_refs = input.supervisor_receipt_refs;
    if let Some(policy) = input.policy
        && run.has_stopped
    {
        let mut shutdown_diagnostics = Vec::new();
        if run.ticks > policy.shutdown_drain_ticks {
            let diagnostic = format!(
                "node control shutdown drain ticks {} exceeded supervisor bound {}",
                run.ticks, policy.shutdown_drain_ticks
            );
            run.diagnostics.push(diagnostic.clone());
            shutdown_diagnostics.push(diagnostic);
        }
        let receipt_ref = write_supervisor_receipt(input.state_root, &SupervisorReceiptValueInput {
            decision: if shutdown_diagnostics.is_empty() {
                "pass"
            } else {
                "deny"
            },
            operation: "shutdown-drain",
            startup_receipt_ref: input.startup_receipt_ref,
            service_lock_ref: Some(input.service_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &shutdown_diagnostics,
        })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    Ok(ShutdownDrain {
        run,
        supervisor_receipt_refs,
    })
}

fn finish_service_run(input: FinishServiceInput<'_>) -> Result<ControlServe> {
    let decision = if input.run.diagnostics.is_empty() {
        "pass"
    } else {
        "deny"
    };
    let receipt_value = service_run_receipt_value(&ServiceRunReceiptValueInput {
        decision,
        startup_receipt_ref: input.startup_receipt_ref,
        service_lock_ref: Some(input.service_lock_ref),
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: input.run.ticks,
        heartbeat_receipt_refs: &input.run.heartbeat_receipt_refs,
        ingress_receipt_refs: &input.run.ingress_receipt_refs,
        loop_receipt_refs: &input.run.loop_receipt_refs,
        processed_request_refs: &input.run.processed_request_refs,
        has_stopped: input.run.has_stopped,
        supervisor_policy_ref: input.supervisor_policy_ref,
        supervisor_receipt_refs: &input.supervisor_receipt_refs,
        diagnostics: &input.run.diagnostics,
    })?;
    let service_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(
        input.state_root,
        &control_service_run_receipt_path(&service_receipt_ref)?,
        &receipt_value,
    )?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: Some(input.service_lock_ref.to_string()),
        heartbeat_receipt_refs: input.run.heartbeat_receipt_refs,
        ingress_receipt_refs: input.run.ingress_receipt_refs,
        loop_receipt_refs: input.run.loop_receipt_refs,
        processed_request_refs: input.run.processed_request_refs,
        supervisor_policy_ref: input.supervisor_policy_ref.map(|value| value.to_string()),
        supervisor_receipt_refs: input.supervisor_receipt_refs,
        ticks: input.run.ticks,
        has_stopped: input.run.has_stopped,
        decision: decision.to_string(),
    })
}

fn denied_duplicate_service_run(
    state_root: &crate::node_state::NodeStateRoot,
    input: &ControlServeInput<'_>,
    startup: &crate::node_runtime::NodeStartupReceipt,
    supervisor_policy: Option<&ControlSupervisorPolicy>,
    inherited_supervisor_receipt_refs: &[String],
) -> Result<ControlServe> {
    let lock_value = read_preserves(
        state_root,
        &crate::node_state::NodeStatePath::parse(CONTROL_SERVICE_LOCK_FILE)?,
    )?;
    let service_lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    let diagnostics = vec!["node control service runner already active".to_string()];
    let mut supervisor_receipt_refs = inherited_supervisor_receipt_refs.to_vec();
    if let Some(policy) = supervisor_policy {
        let receipt_ref = write_supervisor_receipt(state_root, &SupervisorReceiptValueInput {
            decision: "deny",
            operation: "duplicate-runner-deny",
            startup_receipt_ref: &startup.receipt_ref,
            service_lock_ref: Some(&service_lock_ref),
            supervisor_policy_ref: Some(&policy.policy_ref),
            topic: input.topic,
            diagnostics: &diagnostics,
        })?;
        supervisor_receipt_refs.push(receipt_ref);
    }
    let receipt_value = service_run_receipt_value(&ServiceRunReceiptValueInput {
        decision: "deny",
        startup_receipt_ref: &startup.receipt_ref,
        service_lock_ref: Some(&service_lock_ref),
        topic: input.topic,
        max_ticks: input.max_ticks,
        max_requests_per_tick: input.max_requests_per_tick,
        ticks: 0,
        heartbeat_receipt_refs: &[],
        ingress_receipt_refs: &[],
        loop_receipt_refs: &[],
        processed_request_refs: &[],
        has_stopped: false,
        supervisor_policy_ref: supervisor_policy.map(|policy| policy.policy_ref.as_str()),
        supervisor_receipt_refs: &supervisor_receipt_refs,
        diagnostics: &diagnostics,
    })?;
    let service_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(state_root, &control_service_run_receipt_path(&service_receipt_ref)?, &receipt_value)?;
    import_artifact(state_root, &receipt_value)?;
    Ok(ControlServe {
        service_receipt_ref,
        service_receipt_value: receipt_value,
        service_lock_ref: Some(service_lock_ref),
        heartbeat_receipt_refs: Vec::new(),
        ingress_receipt_refs: Vec::new(),
        loop_receipt_refs: Vec::new(),
        processed_request_refs: Vec::new(),
        supervisor_policy_ref: supervisor_policy.map(|policy| policy.policy_ref.clone()),
        supervisor_receipt_refs,
        ticks: 0,
        has_stopped: false,
        decision: "deny".to_string(),
    })
}

fn pending_ingress_envelope_refs(
    state_root: &crate::node_state::NodeStateRoot,
    topic: &str,
) -> Result<Vec<String>> {
    let ingress = state_root.control_ingress()?;
    let topic = ingress.open_subdir(&crate::node_state::NodeStatePath::parse(topic)?)?;
    let entries = topic.list_entries()?;
    if entries.len() > MAX_PENDING_CONTROL_REQUESTS {
        return Err(MoltenError::invalid_harness("node control ingress pending envelope bound exceeded"));
    }
    let mut envelope_refs = Vec::with_capacity(entries.len());
    for entry in entries {
        if entry.kind != crate::node_state::NodeStateEntryKind::RegularFile {
            continue;
        }
        let bytes = topic.read_entry(&entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
        let text = String::from_utf8(bytes)
            .map_err(|error| MoltenError::invalid_harness(format!("node control ingress envelope is not UTF-8: {error}")))?;
        let value = crate::preserves_rail::parse_text(&text)?;
        let envelope = parse_control_ingress_envelope(&value)?;
        if !state_root.try_exists(&control_ingress_receipt_path(&envelope.envelope_ref, "deliver")?)? {
            envelope_refs.push(envelope.envelope_ref);
        }
    }
    Ok(envelope_refs)
}

fn has_pending_service_work(state_root: &crate::node_state::NodeStateRoot, topic: &str) -> Result<bool> {
    if !pending_ingress_envelope_refs(state_root, topic)?.is_empty() {
        return Ok(true);
    }
    next_pending_control_request(state_root).map(|pending| pending.is_some())
}

fn remove_service_lock(state_root: &crate::node_state::NodeStateRoot, service_lock_ref: &str) -> Result<()> {
    let path = crate::node_state::NodeStatePath::parse(CONTROL_SERVICE_LOCK_FILE)?;
    if !state_root.try_exists(&path)? {
        return Ok(());
    }
    let current_ref = crate::preserves_rail::canonical_hash(&read_preserves(state_root, &path)?)?;
    if current_ref != service_lock_ref {
        return Err(MoltenError::invalid_harness("node control service lock changed during serve"));
    }
    state_root.remove_regular_file(&path)
}

fn ingress_receipt_decision(value: &IoValue) -> Result<String> {
    let fields = value
        .collect_simple_record("node-control-ingress-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-ingress-receipt-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::NODE_CONTROL_INGRESS_RECEIPT_SCHEMA,
        "node control ingress receipt",
    )?;
    record_string(&fields[1], "decision")
}

pub fn control_ingress_envelope(input: &ControlIngressEnvelopeInput<'_>) -> Result<ControlIngressEnvelope> {
    control_ingress_envelope_for_transport(input, LOCAL_CONTROL_INGRESS_TRANSPORT, "iroh-local-ingress")
}

pub fn control_live_ingress_envelope(input: &ControlIngressEnvelopeInput<'_>) -> Result<ControlIngressEnvelope> {
    control_ingress_envelope_for_transport(input, LIVE_CONTROL_INGRESS_TRANSPORT, "live-iroh-gossip")
}
