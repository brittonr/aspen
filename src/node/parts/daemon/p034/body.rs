
fn supervisor_summary(value: &IoValue) -> Result<Option<String>> {
    if let Ok(policy) = parse_control_supervisor_policy(value) {
        return Ok(Some(format!(
            "node control supervisor policy ref={} restarts={} stale_lock_recovery={}",
            policy.policy_ref, policy.max_restarts, policy.stale_lock_recovery
        )));
    }
    if let Ok(receipt) = parse_control_supervisor_receipt(value) {
        return Ok(Some(format!(
            "node control supervisor decision={} operation={} policy={}",
            receipt.decision,
            receipt.operation,
            receipt.supervisor_policy_ref.unwrap_or_else(|| "none".to_string())
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(17)) {
        return Ok(Some(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[7], "ticks")?,
            record_sequence_len(&fields[8], "heartbeats")?,
            record_string(&fields[12], "stopped")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-service-run-receipt-v1", Some(15)) {
        return Ok(Some(format!(
            "node control service run decision={} ticks={} heartbeats={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[7], "ticks")?,
            record_sequence_len(&fields[8], "heartbeats")?,
            record_string(&fields[12], "stopped")?
        )));
    }
    Ok(None)
}

fn control_summary(value: &IoValue) -> Result<Option<String>> {
    if let Some(fields) = value.collect_simple_record("node-control-queue-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control queue decision={} phase={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "phase")?,
            record_string(&fields[4], "request")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-operation-receipt-v1", Some(8)) {
        return Ok(Some(format!(
            "node control operation decision={} operation={} request={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "operation")?,
            record_string(&fields[3], "request")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-heartbeat-receipt-v1", Some(9)) {
        return Ok(Some(format!(
            "node control heartbeat decision={} startup={} processed={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_string(&fields[5], "processed-count")?
        )));
    }
    if let Some(fields) = value.collect_simple_record("node-control-loop-receipt-v1", Some(11)) {
        return Ok(Some(format!(
            "node control loop decision={} startup={} processed={} stopped={}",
            record_string(&fields[1], "decision")?,
            record_string(&fields[2], "startup")?,
            record_sequence_len(&fields[5], "processed-requests")?,
            record_string(&fields[7], "stopped")?
        )));
    }
    Ok(None)
}

fn current_startup_receipt<Root: NodeStateAuthority + ?Sized>(
    source: &Root,
) -> Result<crate::node_runtime::NodeStartupReceipt> {
    let root = source.acquire_node_state_root()?;
    let startup_value = read_preserves(&root, &fixed_node_path(STARTUP_FILE)?)?;
    crate::node_runtime::parse_node_startup_receipt(&startup_value)
}

fn write_active_lock(root: &crate::node_state::NodeStateRoot, startup_receipt_ref: &str) -> Result<()> {
    let lock_value = active_lock_value(root, startup_receipt_ref)?;
    write_preserves(root, &fixed_node_path(CONTROL_LOCK_FILE)?, &lock_value)?;
    import_artifact(root, &lock_value)?;
    Ok(())
}

fn require_active_lock(root: &crate::node_state::NodeStateRoot) -> Result<()> {
    let lock_path = fixed_node_path(CONTROL_LOCK_FILE)?;
    if !root.try_exists(&lock_path)? {
        return Err(MoltenError::invalid_harness("node control dispatch requires active node lock"));
    }
    let lock_value = read_preserves(root, &lock_path)?;
    let fields = lock_value
        .collect_simple_record("node-control-lock-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-lock-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA, "node control lock")?;
    let locked_startup = record_string(&fields[2], "startup")?;
    let startup = current_startup_receipt(root)?;
    if locked_startup != startup.receipt_ref {
        return Err(MoltenError::invalid_harness("node control lock is stale for current startup receipt"));
    }
    Ok(())
}

fn remove_active_lock(root: &crate::node_state::NodeStateRoot) -> Result<()> {
    let path = fixed_node_path(CONTROL_LOCK_FILE)?;
    if root.try_exists(&path)? {
        root.remove_regular_file(&path)?;
    }
    Ok(())
}

fn active_lock_value(root: &crate::node_state::NodeStateRoot, startup_receipt_ref: &str) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-lock-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(root)?)]),
        crate::preserves_rail::record("startup", vec![crate::preserves_rail::string(startup_receipt_ref)]),
        crate::preserves_rail::record("owner", vec![crate::preserves_rail::string(&local_ref(
            "node-control-owner",
            startup_receipt_ref,
        )?)]),
        crate::preserves_rail::record("profile", vec![crate::preserves_rail::string(
            "local-preserves-control-file-v1",
        )]),
        crate::preserves_rail::record("checks", vec![crate::preserves_rail::sequence(vec![
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("startup-bound"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("not-authority-token"),
                crate::preserves_rail::string("pass"),
            ]),
            crate::preserves_rail::record("check", vec![
                crate::preserves_rail::string("explicit-state-root"),
                crate::preserves_rail::string("pass"),
            ]),
        ])]),
    ]))
}

fn import_artifact<Root: NodeStateAuthority + ?Sized>(source: &Root, value: &IoValue) -> Result<String> {
    let root = source.acquire_node_state_root()?;
    let ledger = root.ledger_store()?;
    let imported = crate::ledger::import_artifact_with_root(&ledger, value)?;
    let receipt_path = node_leaf_path(
        "receipts",
        &format!("ledger-import-{}.preserves", ref_file_stem(&imported.artifact_ref)),
    )?;
    write_preserves(&root, &receipt_path, &imported.receipt_value)?;
    Ok(imported.artifact_ref)
}

struct PendingControlRequest {
    entry: crate::node_state::NodeStateEntry,
    content_ref: String,
}

fn first_pending_control_request(root: &crate::node_state::NodeStateRoot) -> Result<PendingControlRequest> {
    next_pending_control_request(root)?
        .ok_or_else(|| MoltenError::invalid_harness("node control inbox has no pending requests"))
}

fn next_pending_control_request<Root: NodeStateAuthority + ?Sized>(
    source: &Root,
) -> Result<Option<PendingControlRequest>> {
    let root = source.acquire_node_state_root()?;
    Ok(pending_control_requests(&root)?.into_iter().next())
}

fn pending_control_request_by_name(
    root: &crate::node_state::NodeStateRoot,
    name: &str,
) -> Result<PendingControlRequest> {
    crate::node_state::NodeStatePath::parse("control/inbox")?.join_segment(name)?;
    pending_control_requests(root)?
        .into_iter()
        .find(|request| request.entry.name == name)
        .ok_or_else(|| MoltenError::invalid_harness(format!("node control inbox entry {name} is not pending")))
}

fn pending_control_requests(root: &crate::node_state::NodeStateRoot) -> Result<Vec<PendingControlRequest>> {
    let inbox = root.control_inbox()?;
    let mut requests = Vec::with_capacity(MAX_PENDING_CONTROL_REQUESTS);
    for entry in inbox.list_entries()? {
        if entry.kind != crate::node_state::NodeStateEntryKind::RegularFile
            || !entry.name.ends_with(".preserves")
            || entry.name.contains("receipt")
        {
            continue;
        }
        if requests.len() >= MAX_PENDING_CONTROL_REQUESTS {
            return Err(MoltenError::invalid_harness("too many pending node control requests"));
        }
        let bytes = inbox.read_entry(&entry, crate::node_state::MAX_NODE_STATE_FILE_BYTES)?;
        requests.push(PendingControlRequest {
            entry,
            content_ref: crate::preserves_rail::content_ref_from_bytes(&bytes),
        });
    }
    Ok(requests)
}

fn archive_dispatched_request(
    root: &crate::node_state::NodeStateRoot,
    request_entry: &crate::node_state::NodeStateEntry,
    request_value: &IoValue,
) -> Result<()> {
    let request_ref = crate::preserves_rail::canonical_hash(request_value)?;
    let archived = control_outbox_request_path(&request_ref)?;
    write_preserves(root, &archived, request_value)?;
    root.control_inbox()?.remove_entry(request_entry)
}

fn node_leaf_path(base: &str, leaf: &str) -> Result<crate::node_state::NodeStatePath> {
    fixed_node_path(base)?.join_segment(leaf)
}

fn control_inbox_entry_name(request_ref: &str) -> String {
    format!("{}.preserves", ref_file_stem(request_ref))
}

fn control_inbox_path(request_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(CONTROL_INBOX_DIR, &control_inbox_entry_name(request_ref))
}

fn queue_receipt_path(request_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_INBOX_DIR,
        &format!("{}.queue-receipt.preserves", ref_file_stem(request_ref)),
    )
}

fn dispatch_receipt_path(request_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_OUTBOX_DIR,
        &format!("{}.dispatch-receipt.preserves", ref_file_stem(request_ref)),
    )
}

fn control_outbox_request_path(request_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_OUTBOX_DIR,
        &format!("{}.request.preserves", ref_file_stem(request_ref)),
    )
}

fn control_outbox_receipt_path(request_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_OUTBOX_DIR,
        &format!("{}.control-receipt.preserves", ref_file_stem(request_ref)),
    )
}

fn control_operation_receipt_path(request_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_OUTBOX_DIR,
        &format!("{}.operation-receipt.preserves", ref_file_stem(request_ref)),
    )
}

fn control_operation_subreceipt_path(request_ref: &str, label: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_OUTBOX_DIR,
        &format!("{}.{}.preserves", ref_file_stem(request_ref), label),
    )
}

fn control_heartbeat_receipt_path(heartbeat_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_OUTBOX_DIR,
        &format!("{}.heartbeat-receipt.preserves", ref_file_stem(heartbeat_ref)),
    )
}

fn control_loop_receipt_path(loop_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_OUTBOX_DIR,
        &format!("{}.loop-receipt.preserves", ref_file_stem(loop_ref)),
    )
}

fn control_service_heartbeat_path(heartbeat_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_SERVICE_DIR,
        &format!("{}.service-heartbeat.preserves", ref_file_stem(heartbeat_ref)),
    )
}

fn control_service_run_receipt_path(service_run_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_SERVICE_DIR,
        &format!("{}.service-run-receipt.preserves", ref_file_stem(service_run_ref)),
    )
}

fn control_supervisor_receipt_path(receipt_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    node_leaf_path(
        CONTROL_SERVICE_DIR,
        &format!("{}.supervisor-receipt.preserves", ref_file_stem(receipt_ref)),
    )
}

fn write_supervisor_receipt(
    root: &crate::node_state::NodeStateRoot,
    input: &SupervisorReceiptValueInput<'_>,
) -> Result<String> {
    let value = supervisor_receipt_value(input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    write_preserves(root, &control_supervisor_receipt_path(&receipt_ref)?, &value)?;
    import_artifact(root, &value)?;
    Ok(receipt_ref)
}

fn control_ingress_envelope_path(topic: &str, envelope_ref: &str) -> Result<crate::node_state::NodeStatePath> {
    fixed_node_path(CONTROL_INGRESS_DIR)?
        .join_segment(topic)?
        .join_segment(&format!("{}.envelope.preserves", ref_file_stem(envelope_ref)))
}

fn write_ingress_envelope_and_verify(
    root: &crate::node_state::NodeStateRoot,
    topic: &str,
    envelope: &ControlIngressEnvelope,
) -> Result<()> {
    let path = control_ingress_envelope_path(topic, &envelope.envelope_ref)?;
    write_preserves(root, &path, &envelope.value)?;
    let read_value = read_preserves(root, &path)?;
    let read_envelope = parse_control_ingress_envelope(&read_value)?;
    if read_envelope.envelope_ref != envelope.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "node control ingress materialized envelope ref {} does not match written {}",
            read_envelope.envelope_ref, envelope.envelope_ref
        )));
    }
    Ok(())
}

fn control_ingress_receipt_path(envelope_ref: &str, phase: &str) -> Result<crate::node_state::NodeStatePath> {
    fixed_node_path(CONTROL_INGRESS_DIR)?
        .join("receipts")?
        .join_segment(&format!("{}.{}.receipt.preserves", ref_file_stem(envelope_ref), phase))
}
