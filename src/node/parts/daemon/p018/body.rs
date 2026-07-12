pub fn run_local(input: &RunInput<'_>) -> Result<Run> {
    validate_state_root(input.state_root)?;
    let root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    run_local_with_root(&root)
}

pub fn run_local_with_root(root: &crate::node_state::NodeStateRoot) -> Result<Run> {
    ensure_state_layout(root)?;
    verify_restart_state(root)?;
    let config_value = read_preserves(root, &fixed_node_path(CONFIG_FILE)?)?;
    let identity_receipt = read_preserves(root, &fixed_node_path(IDENTITY_RECEIPT_FILE)?)?;
    let identity_receipt_ref = crate::preserves_rail::canonical_hash(&identity_receipt)?;
    let index_receipt_refs = index_receipt_refs(root)?;
    let resource_receipt_refs = resource_receipt_refs(root)?;
    let capability_receipt_refs = capability_receipt_refs(root)?;
    let version_refs = vec![local_ref("molten-binary-version", env!("CARGO_PKG_VERSION"))?];
    let profile_metadata_refs = profile_metadata_refs(root)?;
    let source_gate_value = crate::octet_gate::synthetic_clean_octet_gate_receipt_for_tests()?;
    let source_gate_ref = crate::preserves_rail::canonical_hash(&source_gate_value)?;
    let run = crate::node_runtime::start_node_runtime(&crate::node_runtime::NodeRuntimeStartInput {
        config_value,
        identity_receipt_ref,
        index_receipt_refs,
        source_gate_receipt_refs: vec![source_gate_ref],
        source_gate_receipt_values: vec![source_gate_value],
        profile_metadata_refs,
        capability_receipt_refs,
        resource_receipt_refs,
        version_refs,
    })?;
    for (adapter, value) in run.adapter_receipts.iter().zip(run.adapter_receipt_values.iter()) {
        let path = node_leaf_path("receipts", &format!("adapter-start-{}.preserves", adapter.name))?;
        write_preserves(root, &path, value)?;
    }
    write_preserves(root, &fixed_node_path(STARTUP_FILE)?, &run.startup_receipt.value)?;
    if run.decision != "pass" {
        return Err(MoltenError::invalid_harness(format!(
            "node daemon startup denied receipt={}",
            run.startup_receipt.receipt_ref
        )));
    }
    let startup_ref = run.startup_receipt.receipt_ref.clone();
    write_active_lock(root, &startup_ref)?;
    import_artifact(root, &run.startup_receipt.value)?;
    Ok(Run {
        startup_ref,
        startup_value: run.startup_receipt.value,
        adapter_receipt_refs: run.adapter_receipts,
    })
}

pub fn status_local(input: &StatusInput<'_>) -> Result<Status> {
    validate_state_root(input.state_root)?;
    let root = crate::node_state::NodeStateRoot::open_existing(input.state_root)?;
    status_local_with_root(&root)
}

pub fn status_local_with_root(root: &crate::node_state::NodeStateRoot) -> Result<Status> {
    let request = status_request()?;
    status_local_node_with_request(root, &request)
}

fn status_local_node_with_request(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<Status> {
    let startup_value = read_preserves(root, &fixed_node_path(STARTUP_FILE)?)?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let shutdown_path = fixed_node_path(SHUTDOWN_FILE)?;
    let shutdown_ref = if root.try_exists(&shutdown_path)? {
        Some(crate::preserves_rail::canonical_hash(&read_preserves(root, &shutdown_path)?)?)
    } else {
        None
    };
    let status = if shutdown_ref.is_some() { "stopped" } else { "running" }.to_string();
    let health_value = crate::node_runtime::node_health_receipt_value(&crate::node_runtime::HealthReceiptValueInput {
        decision: "pass",
        startup_receipt_ref: &startup.receipt_ref,
        shutdown_receipt_ref: shutdown_ref.as_deref(),
        adapter_receipts: &startup.adapters,
        index_receipt_refs: &index_receipt_refs(root)?,
        head_refs: std::slice::from_ref(&startup.receipt_ref),
        open_job_refs: &[],
        replay_is_eligible: shutdown_ref.is_some(),
        diagnostics: &[],
    })?;
    let health_ref = crate::preserves_rail::canonical_hash(&health_value)?;
    write_preserves(root, &fixed_node_path(HEALTH_FILE)?, &health_value)?;
    import_artifact(root, &health_value)?;
    let control_receipt_value = control_receipt_for_request(
        root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&health_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    write_preserves(root, &fixed_node_path(CONTROL_STATUS_FILE)?, &control_receipt_value)?;
    import_artifact(root, &control_receipt_value)?;
    Ok(Status {
        health_ref,
        control_receipt_ref,
        health_value,
        control_receipt_value,
        status,
    })
}

pub fn stop_local(input: &StopInput<'_>) -> Result<Stop> {
    validate_state_root(input.state_root)?;
    let root = crate::node_state::NodeStateRoot::open_existing(input.state_root)?;
    stop_local_with_root(&root)
}

pub fn stop_local_with_root(root: &crate::node_state::NodeStateRoot) -> Result<Stop> {
    let request = shutdown_request()?;
    stop_local_node_with_request(root, &request)
}

fn stop_local_node_with_request(
    root: &crate::node_state::NodeStateRoot,
    request: &crate::node_runtime::ControlRequest,
) -> Result<Stop> {
    let startup_value = read_preserves(root, &fixed_node_path(STARTUP_FILE)?)?;
    let startup = crate::node_runtime::parse_node_startup_receipt(&startup_value)?;
    let mut shutdown_adapters = Vec::with_capacity(startup.adapters.len());
    for adapter in startup.adapters.iter().rev() {
        let binding = crate::node_runtime::node_adapter_binding(&adapter.name, &adapter.receipt_ref)?;
        let value = crate::node_runtime::node_adapter_lifecycle_receipt_value(
            &crate::node_runtime::AdapterLifecycleReceiptInput {
                operation: "shutdown",
                decision: "pass",
                adapter: &binding,
                index_receipt_refs: &index_receipt_refs(root)?,
                resource_receipt_refs: &resource_receipt_refs(root)?,
                diagnostics: &[],
            },
        )?;
        let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
        let path = node_leaf_path("receipts", &format!("adapter-shutdown-{}.preserves", adapter.name))?;
        write_preserves(root, &path, &value)?;
        import_artifact(root, &value)?;
        shutdown_adapters.push(crate::node_runtime::NodeAdapterReceiptRef {
            name: adapter.name.clone(),
            receipt_ref,
        });
    }
    let index_refs = index_receipt_refs(root)?;
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
    write_preserves(root, &fixed_node_path(SHUTDOWN_FILE)?, &shutdown_value)?;
    import_artifact(root, &shutdown_value)?;
    let control_receipt_value = control_receipt_for_request(
        root,
        request,
        &startup.receipt_ref,
        std::slice::from_ref(&shutdown_ref),
        &[],
    )?;
    let control_receipt_ref = crate::preserves_rail::canonical_hash(&control_receipt_value)?;
    write_preserves(root, &fixed_node_path(CONTROL_STOP_FILE)?, &control_receipt_value)?;
    import_artifact(root, &control_receipt_value)?;
    remove_active_lock(root)?;
    Ok(Stop {
        shutdown_ref,
        control_receipt_ref,
        shutdown_value,
        control_receipt_value,
    })
}

pub fn submit_control_request(input: &ControlSubmitInput<'_>) -> Result<ControlSubmit> {
    validate_state_root(input.state_root)?;
    let root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    submit_control_request_with_root(&root, input.request_value)
}

pub fn submit_control_request_with_root(
    root: &crate::node_state::NodeStateRoot,
    request_value: &IoValue,
) -> Result<ControlSubmit> {
    ensure_state_layout(root)?;
    let request = crate::node_runtime::parse_control_request(request_value)?;
    import_artifact(root, request_value)?;
    let inbox_path = control_inbox_path(&request.request_ref)?;
    write_preserves(root, &inbox_path, request_value)?;
    let location_ref = local_ref("node-control-inbox-path", &inbox_path.display())?;
    let receipt_value = queue_receipt_value(&QueueReceiptValueInput {
        decision: "pass",
        phase: "enqueue",
        operation: &request.operation,
        request_ref: &request.request_ref,
        location_ref: &location_ref,
        diagnostics: &[],
    })?;
    let queue_receipt_ref = crate::preserves_rail::canonical_hash(&receipt_value)?;
    write_preserves(root, &queue_receipt_path(&request.request_ref)?, &receipt_value)?;
    import_artifact(root, &receipt_value)?;
    Ok(ControlSubmit {
        request_ref: request.request_ref.clone(),
        inbox_entry: control_inbox_entry_name(&request.request_ref),
        queue_receipt_ref,
        queue_receipt_value: receipt_value,
    })
}

pub fn dispatch_control_request(input: &ControlDispatchInput<'_>) -> Result<ControlDispatch> {
    validate_state_root(input.state_root)?;
    let request_entry = compatibility_request_entry(input.state_root, input.request_path)?;
    let root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    dispatch_control_request_with_root(&root, request_entry)
}

pub fn dispatch_control_request_entry(input: &ControlDispatchEntryInput<'_>) -> Result<ControlDispatch> {
    validate_state_root(input.state_root)?;
    let root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    dispatch_control_request_with_root(&root, input.request_entry)
}

fn compatibility_request_entry<'a>(state_root: &Path, request_path: Option<&'a Path>) -> Result<Option<&'a str>> {
    let Some(request_path) = request_path else {
        return Ok(None);
    };
    let mut components = request_path.components();
    if let (Some(std::path::Component::Normal(entry)), None) = (components.next(), components.next()) {
        return entry
            .to_str()
            .map(Some)
            .ok_or_else(|| MoltenError::invalid_harness("node control compatibility request entry must be UTF-8"));
    }

    let relative = request_path.strip_prefix(state_root).map_err(|_| {
        MoltenError::invalid_harness(
            "node control compatibility request path must name the selected state root inbox",
        )
    })?;
    if relative.parent() != Some(Path::new(CONTROL_INBOX_DIR)) {
        return Err(MoltenError::invalid_harness(
            "node control compatibility request path must name the selected state root inbox",
        ));
    }
    let entry = relative
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| MoltenError::invalid_harness("node control compatibility request entry must be UTF-8"))?;
    crate::node_state::NodeStatePath::parse(CONTROL_INBOX_DIR)?.join_segment(entry)?;
    Ok(Some(entry))
}

pub fn dispatch_control_request_with_root(
    root: &crate::node_state::NodeStateRoot,
    request_entry: Option<&str>,
) -> Result<ControlDispatch> {
    ensure_state_layout(root)?;
    require_active_lock(root)?;
    let pending = match request_entry {
        Some(name) => pending_control_request_by_name(root, name)?,
        None => first_pending_control_request(root)?,
    };
    dispatch_pending_control_request(root, pending)
}

fn dispatch_pending_control_request(
    root: &crate::node_state::NodeStateRoot,
    pending: PendingControlRequest,
) -> Result<ControlDispatch> {
    let inbox = root.control_inbox()?;
    let bytes = inbox.read_entry(&pending.entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
    let observed_ref = crate::preserves_rail::content_ref_from_bytes(&bytes);
    if observed_ref != pending.content_ref {
        return Err(MoltenError::invalid_harness(format!(
            "node control inbox entry {} changed between discovery and dispatch",
            pending.entry.name
        )));
    }
    let text = String::from_utf8(bytes)
        .map_err(|error| MoltenError::invalid_harness(format!("node control request is not UTF-8: {error}")))?;
    let request_value = crate::preserves_rail::parse_text(&text)?;
    let request = crate::node_runtime::parse_control_request(&request_value)?;
    import_artifact(root, &request_value)?;
    if let Some(prior) = prior_dispatch_for_request(root, &request)? {
        archive_dispatched_request(root, &pending.entry, &request.value)?;
        write_dispatch_queue_receipt(root, &request, "duplicate-dispatch")?;
        return Ok(prior);
    }
    let dispatch = match request.operation.as_str() {
        "status" => dispatch_status_request(root, &request)?,
        "shutdown" => dispatch_shutdown_request(root, &request)?,
        "install" => dispatch_install_request(root, &request)?,
        "run" => dispatch_run_request(root, &request)?,
        "gate" => dispatch_gate_request(root, &request)?,
        other => {
            return Err(MoltenError::invalid_harness(format!("node control request operation unsupported: {other}")));
        }
    };
    archive_dispatched_request(root, &pending.entry, &request.value)?;
    write_dispatch_queue_receipt(root, &request, "dispatch")?;
    Ok(dispatch)
}

pub fn run_control_loop(input: &ControlLoopInput<'_>) -> Result<ControlLoop> {
    validate_state_root(input.state_root)?;
    let root = crate::node_state::NodeStateRoot::open(input.state_root)?;
    run_control_loop_with_root(&root, input.max_requests)
}

pub fn run_control_loop_with_root(
    root: &crate::node_state::NodeStateRoot,
    maximum_requests: u64,
) -> Result<ControlLoop> {
    ensure_state_layout(root)?;
    let max_requests = validate_loop_request_limit(maximum_requests)?;
    require_active_lock(root)?;
    let startup = current_startup_receipt(root)?;
    let lock_value = read_preserves(root, &fixed_node_path(CONTROL_LOCK_FILE)?)?;
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
    write_preserves(
        root,
        &control_heartbeat_receipt_path(&heartbeat_receipt_ref)?,
        &heartbeat_value,
    )?;
    import_artifact(root, &heartbeat_value)?;

    let mut processed_request_refs = Vec::with_capacity(max_requests);
    let mut dispatch_receipt_refs = Vec::with_capacity(max_requests);
    let mut diagnostics = Vec::new();
    let mut has_stopped = false;
    for _ in 0..max_requests {
        let Some(pending) = next_pending_control_request(root)? else {
            break;
        };
        let dispatched = dispatch_pending_control_request(root, pending)?;
        let control = crate::node_runtime::parse_control_receipt(&dispatched.control_receipt_value)?;
        processed_request_refs.push(dispatched.request_ref.clone());
        dispatch_receipt_refs.push(dispatched.control_receipt_ref.clone());
        if dispatched.operation == "shutdown" && control.decision == "pass" {
            has_stopped = true;
            break;
        }
    }
    if processed_request_refs.len() == max_requests && next_pending_control_request(root)?.is_some() {
        diagnostics.push("node control loop reached max requests with pending inbox entries".to_string());
    }
    let decision = if diagnostics.is_empty() { "pass" } else { "deny" };
    let loop_value = loop_receipt_value(&LoopReceiptValueInput {
        decision,
        startup_receipt_ref: &startup.receipt_ref,
        heartbeat_receipt_ref: &heartbeat_receipt_ref,
        max_requests: maximum_requests,
        processed_request_refs: &processed_request_refs,
        dispatch_receipt_refs: &dispatch_receipt_refs,
        has_stopped,
        diagnostics: &diagnostics,
    })?;
    let loop_receipt_ref = crate::preserves_rail::canonical_hash(&loop_value)?;
    write_preserves(root, &control_loop_receipt_path(&loop_receipt_ref)?, &loop_value)?;
    import_artifact(root, &loop_value)?;
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
