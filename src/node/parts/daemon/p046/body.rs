
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
