
pub fn run_local(input: &RunInput<'_>) -> Result<Run> {
    ensure_state_layout(input.state_root)?;
    verify_restart_state(input.state_root)?;
    let config_value = read_preserves(&input.state_root.join(CONFIG_FILE))?;
    let identity_receipt = read_preserves(&input.state_root.join(IDENTITY_RECEIPT_FILE))?;
    let identity_receipt_ref = crate::preserves_rail::canonical_hash(&identity_receipt)?;
    let index_receipt_refs = index_receipt_refs(input.state_root)?;
    let resource_receipt_refs = resource_receipt_refs(input.state_root)?;
    let capability_receipt_refs = capability_receipt_refs(input.state_root)?;
    let version_refs = vec![local_ref("molten-binary-version", env!("CARGO_PKG_VERSION"))?];
    let source_gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let source_gate_ref = crate::preserves_rail::canonical_hash(&source_gate_value)?;
    let run = crate::node_runtime::start_node_runtime(&crate::node_runtime::NodeRuntimeStartInput {
        config_value,
        identity_receipt_ref,
        index_receipt_refs,
        source_gate_receipt_refs: vec![source_gate_ref],
        source_gate_receipt_values: vec![source_gate_value],
        capability_receipt_refs,
        resource_receipt_refs,
        version_refs,
    })?;
    for (adapter, value) in run.adapter_receipts.iter().zip(run.adapter_receipt_values.iter()) {
        write_preserves(
            &input.state_root.join("receipts").join(format!("adapter-start-{}.preserves", adapter.name)),
            value,
        )?;
    }
    write_preserves(&input.state_root.join(STARTUP_FILE), &run.startup_receipt.value)?;
    if run.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "node daemon startup denied receipt={}",
            run.startup_receipt.receipt_ref
        )));
    }
    let startup_ref = run.startup_receipt.receipt_ref.clone();
    write_active_lock(input.state_root, &startup_ref)?;
    import_artifact(input.state_root, &run.startup_receipt.value)?;
    Ok(Run {
        startup_ref,
        startup_value: run.startup_receipt.value,
        adapter_receipt_refs: run.adapter_receipts,
    })
}

pub fn status_local(input: &StatusInput<'_>) -> Result<Status> {
    let request = status_request()?;
    status_local_node_with_request(input, &request)
}

fn status_local_node_with_request(
    input: &StatusInput<'_>,
    request: &crate::node_runtime::ControlRequest,
) -> Result<Status> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let shutdown_ref = if input.state_root.join(SHUTDOWN_FILE).exists() {
        Some(crate::preserves_rail::canonical_hash(&read_preserves(&input.state_root.join(SHUTDOWN_FILE))?)?)
    } else {
        None
    };
    let status = if shutdown_ref.is_some() { "stopped" } else { "running" }.to_string();
    let health_value = crate::node_runtime::node_health_receipt_value(&crate::node_runtime::HealthReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        shutdown_receipt_ref: shutdown_ref.as_deref(),
        adapter_receipts: &startup.adapters,
        index_receipt_refs: &index_receipt_refs(input.state_root)?,
        head_refs: std::slice::from_ref(&startup.receipt_ref),
        open_job_refs: &[],
        replay_is_eligible: shutdown_ref.is_some(),
        diagnostics: &[],
    })?;
    let health_ref = crate::preserves_rail::canonical_hash(&health_value)?;
    write_preserves(&input.state_root.join(HEALTH_FILE), &health_value)?;
    import_artifact(input.state_root, &health_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&health_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STATUS_FILE), &control_receipt_value)?;
    import_artifact(input.state_root, &control_receipt_value)?;
    Ok(Status {
        health_ref,
        control_receipt_ref,
        health_value,
        control_receipt_value,
        status,
    })
}

pub fn stop_local(input: &StopInput<'_>) -> Result<Stop> {
    let request = shutdown_request()?;
    stop_local_node_with_request(input, &request)
}

fn stop_local_node_with_request(input: &StopInput<'_>, request: &crate::node_runtime::ControlRequest) -> Result<Stop> {
    let startup_value = read_preserves(&input.state_root.join(STARTUP_FILE))?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let mut shutdown_adapters = Vec::with_capacity(startup.adapters.len());
    for adapter in startup.adapters.iter().rev() {
        let binding = crate::node_runtime::node_adapter_binding(&adapter.name, &adapter.receipt_ref)?;
        let value = crate::node_runtime::node_adapter_lifecycle_receipt_value(
            &crate::node_runtime::AdapterLifecycleReceiptInput {
                operation: "shutdown",
                decision: "pass",
                adapter: &binding,
                index_receipt_refs: &index_receipt_refs(input.state_root)?,
                resource_receipt_refs: &resource_receipt_refs(input.state_root)?,
                diagnostics: &[],
            },
        )?;
        let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
        write_preserves(
            &input.state_root.join("receipts").join(format!("adapter-shutdown-{}.preserves", adapter.name)),
            &value,
        )?;
        import_artifact(input.state_root, &value)?;
        shutdown_adapters.push(crate::node_runtime::NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref,
        });
    }
    let index_refs = index_receipt_refs(input.state_root)?;
    let shutdown_value =
        crate::node_runtime::node_shutdown_receipt_value(&crate::node_runtime::ShutdownReceiptValueInput {
            decision: "pass",
            startup_receipt_ref: &startup.receipt_ref,
            adapter_receipts: &shutdown_adapters,
            drained_job_refs: &[],
            index_receipt_refs: &index_refs,
            diagnostics: &[],
        })?;
    let shutdown_ref = crate::preserves_rail::canonical_hash(&shutdown_value)?;
    write_preserves(&input.state_root.join(SHUTDOWN_FILE), &shutdown_value)?;
    import_artifact(input.state_root, &shutdown_value)?;
    let control_receipt_value = control_receipt_for_request(
        input.state_root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&shutdown_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    write_preserves(&input.state_root.join(CONTROL_STOP_FILE), &control_receipt_value)?;
    import_artifact(input.state_root, &control_receipt_value)?;
    remove_active_lock(input.state_root)?;
    Ok(Stop {
        shutdown_ref,
        control_receipt_ref,
        shutdown_value,
        control_receipt_value,
    })
}

pub fn submit_control_request(input: &ControlSubmitInput<'_>) -> Result<ControlSubmit> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let request = crate::node_runtime::parse_control_request(input.request_value)?;
    import_artifact(input.state_root, input.request_value)?;
    let inbox_path = control_inbox_path(input.state_root, &request.request_ref);
    write_preserves(&inbox_path, input.request_value)?;
    let location_ref = local_ref("node-control-inbox-path", &inbox_path.display().to_string())?;
    let receipt_value = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase: "enqueue",
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &[],
    })?;
    let queue_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(&queue_receipt_path(input.state_root, &request.request_ref), &receipt_value)?;
    import_artifact(input.state_root, &receipt_value)?;
    Ok(ControlSubmit {
        request_ref: request.request_ref,
        inbox_path,
        queue_receipt_ref,
        queue_receipt_value: receipt_value,
    })
}

pub fn dispatch_control_request(input: &ControlDispatchInput<'_>) -> Result<ControlDispatch> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    require_active_lock(input.state_root)?;
    let request_path = match input.request_path {
        Some(path) => path.to_path_buf(),
        None => first_pending_control_request(input.state_root)?,
    };
    let request_value = read_preserves(&request_path)?;
    let request = crate::node_runtime::parse_control_request(&request_value)?;
    import_artifact(input.state_root, &request_value)?;
    if let Some(prior) = prior_dispatch_for_request(input.state_root, &request)? {
        archive_dispatched_request(input.state_root, &request_path, &request.value)?;
        write_dispatch_queue_receipt(input.state_root, &request, "duplicate-dispatch")?;
        return Ok(prior);
    }
    let dispatch = match request.operation.as_str() {
        "status" => dispatch_status_request(input.state_root, &request)?,
        "shutdown" => dispatch_shutdown_request(input.state_root, &request)?,
        "install" => dispatch_install_request(input.state_root, &request)?,
        "run" => dispatch_run_request(input.state_root, &request)?,
        "gate" => dispatch_gate_request(input.state_root, &request)?,
        other => {
            return Err(MoltenError::invalid_harness(format!("node control request operation unsupported: {other}")));
        }
    };
    archive_dispatched_request(input.state_root, &request_path, &request.value)?;
    write_dispatch_queue_receipt(input.state_root, &request, "dispatch")?;
    Ok(dispatch)
}

pub fn run_control_loop(input: &ControlLoopInput<'_>) -> Result<ControlLoop> {
    validate_state_root(input.state_root)?;
    ensure_state_layout(input.state_root)?;
    let max_requests = validate_loop_request_limit(input.max_requests)?;
    require_active_lock(input.state_root)?;
    let startup = current_startup_receipt(input.state_root)?;
    let lock_value = read_preserves(&input.state_root.join(CONTROL_LOCK_FILE))?;
    let lock_ref = crate::preserves_rail::canonical_hash(&lock_value)?;
    let initial_diagnostics = Vec::new();
    let heartbeat_value = heartbeat_receipt_value(&HeartbeatReceiptValueInput {
        startup_receipt_ref: &startup.receipt_ref,
        lock_ref: &lock_ref,
        loop_sequence: 0,
        processed_count: 0,
        diagnostics: &initial_diagnostics,
    })?;
    let heartbeat_receipt_ref = crate::preserves_rail::canonical_hash(&heartbeat_value)?;
    write_preserves(&control_heartbeat_receipt_path(input.state_root, &heartbeat_receipt_ref), &heartbeat_value)?;
    import_artifact(input.state_root, &heartbeat_value)?;

    let mut processed_request_refs = Vec::with_capacity(max_requests);
    let mut dispatch_receipt_refs = Vec::with_capacity(max_requests);
    let mut diagnostics = Vec::new();
    let mut has_stopped = false;
    for _ in 0..max_requests {
        let Some(request_path) = next_pending_control_request(input.state_root)? else {
            break;
        };
        let dispatched = dispatch_control_request(&ControlDispatchInput {
            state_root: input.state_root,
            request_path: Some(&request_path),
        })?;
        let control = crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value)?;
        processed_request_refs.push(dispatched.request_ref.clone());
        dispatch_receipt_refs.push(dispatched.control_receipt_ref.clone());
        if dispatched.operation == "shutdown" && control.decision == "pass" {
            has_stopped = true;
            break;
        }
    }
    if processed_request_refs.len() == max_requests && next_pending_control_request(input.state_root)?.is_some() {
        diagnostics.push("node control loop reached max requests with pending inbox entries".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let loop_value = loop_receipt_value(&LoopReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        heartbeat_receipt_ref: &heartbeat_receipt_ref,
        max_requests: input.max_requests,
        processed_request_refs: &processed_request_refs,
        dispatch_receipt_refs: &dispatch_receipt_refs,
        has_stopped,
        diagnostics: &diagnostics,
    })?;
    let loop_receipt_ref = crate::preserves_rail::canonical_hash(&loop_value)?;
    write_preserves(&control_loop_receipt_path(input.state_root, &loop_receipt_ref), &loop_value)?;
    import_artifact(input.state_root, &loop_value)?;
    Ok(ControlLoop {
        loop_receipt_ref,
        loop_receipt_value: loop_value,
        heartbeat_receipt_ref,
        heartbeat_receipt_value: heartbeat_value,
        processed_request_refs,
        dispatch_receipt_refs,
        has_stopped,
    })
}
