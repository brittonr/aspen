
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

fn current_startup_receipt(state_root: &Path) -> Result<crate::node_runtime::NodeStartupReceipt> {
    let startup_value = read_preserves(&state_root.join(STARTUP_FILE))?;
    crate::node_runtime::parse_node_startup_receipt(&startup_value)
}

fn write_active_lock(state_root: &Path, startup_receipt_ref: &str) -> Result<()> {
    let lock_value = active_lock_value(state_root, startup_receipt_ref)?;
    write_preserves(&state_root.join(CONTROL_LOCK_FILE), &lock_value)?;
    import_artifact(state_root, &lock_value)?;
    Ok(())
}

fn require_active_lock(state_root: &Path) -> Result<()> {
    let lock_path = state_root.join(CONTROL_LOCK_FILE);
    if !lock_path.exists() {
        return Err(MoltenError::invalid_harness("node control dispatch requires active node lock"));
    }
    let lock_value = read_preserves(&lock_path)?;
    let fields = lock_value
        .collect_simple_record("node-control-lock-v1", Some(6))
        .ok_or_else(|| MoltenError::invalid_harness("expected <node-control-lock-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA, "node control lock")?;
    let locked_startup = record_string(&fields[2], "startup")?;
    let startup = current_startup_receipt(state_root)?;
    if locked_startup != startup.receipt_ref {
        return Err(MoltenError::invalid_harness("node control lock is stale for current startup receipt"));
    }
    Ok(())
}

fn remove_active_lock(state_root: &Path) -> Result<()> {
    let path = state_root.join(CONTROL_LOCK_FILE);
    if path.exists() {
        fs::remove_file(path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn active_lock_value(state_root: &Path, startup_receipt_ref: &str) -> Result<IoValue> {
    Ok(crate::preserves_rail::record("node-control-lock-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::NODE_CONTROL_LOCK_SCHEMA),
        crate::preserves_rail::record("state-root", vec![crate::preserves_rail::string(&state_root_profile_ref(
            state_root,
        )?)]),
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

fn import_artifact(state_root: &Path, value: &IoValue) -> Result<String> {
    let imported = crate::ledger::import_artifact(&state_root.join("ledger"), value)?;
    let receipt_path = state_root
        .join("receipts")
        .join(format!("ledger-import-{}.preserves", ref_file_stem(&imported.artifact_ref)));
    write_preserves(&receipt_path, &imported.receipt_value)?;
    Ok(imported.artifact_ref)
}

fn first_pending_control_request(state_root: &Path) -> Result<PathBuf> {
    next_pending_control_request(state_root)?
        .ok_or_else(|| MoltenError::invalid_harness("node control inbox has no pending requests"))
}

fn next_pending_control_request(state_root: &Path) -> Result<Option<PathBuf>> {
    let mut paths = pending_control_request_paths(state_root)?;
    Ok(paths.pop())
}

fn pending_control_request_paths(state_root: &Path) -> Result<Vec<PathBuf>> {
    let inbox = state_root.join(CONTROL_INBOX_DIR);
    let mut paths = Vec::with_capacity(MAX_PENDING_CONTROL_REQUESTS);
    for entry_result in fs::read_dir(&inbox).map_err(MoltenError::from)? {
        if paths.len() >= MAX_PENDING_CONTROL_REQUESTS {
            return Err(MoltenError::invalid_harness("too many pending node control requests"));
        }
        let entry = entry_result.map_err(MoltenError::from)?;
        let path = entry.path();
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
        if path.is_file() && name.ends_with(".preserves") && !name.contains("receipt") {
            paths.push(path);
        }
    }
    paths.sort_by(|left, right| right.cmp(left));
    Ok(paths)
}

fn archive_dispatched_request(state_root: &Path, request_path: &Path, request_value: &IoValue) -> Result<()> {
    let request_ref = crate::preserves_rail::canonical_hash(request_value)?;
    let archived = control_outbox_request_path(state_root, &request_ref);
    write_preserves(&archived, request_value)?;
    if request_path.starts_with(state_root.join(CONTROL_INBOX_DIR)) && request_path.exists() {
        fs::remove_file(request_path).map_err(MoltenError::from)?;
    }
    Ok(())
}

fn control_inbox_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root.join(CONTROL_INBOX_DIR).join(format!("{}.preserves", ref_file_stem(request_ref)))
}

fn queue_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INBOX_DIR)
        .join(format!("{}.queue-receipt.preserves", ref_file_stem(request_ref)))
}

fn dispatch_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.dispatch-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_outbox_request_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.request.preserves", ref_file_stem(request_ref)))
}

fn control_outbox_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.control-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_operation_receipt_path(state_root: &Path, request_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.operation-receipt.preserves", ref_file_stem(request_ref)))
}

fn control_operation_subreceipt_path(state_root: &Path, request_ref: &str, label: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.{}.preserves", ref_file_stem(request_ref), label))
}

fn control_heartbeat_receipt_path(state_root: &Path, heartbeat_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.heartbeat-receipt.preserves", ref_file_stem(heartbeat_ref)))
}

fn control_loop_receipt_path(state_root: &Path, loop_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_OUTBOX_DIR)
        .join(format!("{}.loop-receipt.preserves", ref_file_stem(loop_ref)))
}

fn control_service_heartbeat_path(state_root: &Path, heartbeat_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_SERVICE_DIR)
        .join(format!("{}.service-heartbeat.preserves", ref_file_stem(heartbeat_ref)))
}

fn control_service_run_receipt_path(state_root: &Path, service_run_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_SERVICE_DIR)
        .join(format!("{}.service-run-receipt.preserves", ref_file_stem(service_run_ref)))
}

fn control_supervisor_receipt_path(state_root: &Path, receipt_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_SERVICE_DIR)
        .join(format!("{}.supervisor-receipt.preserves", ref_file_stem(receipt_ref)))
}

fn write_supervisor_receipt(state_root: &Path, input: &SupervisorReceiptValueInput<'_>) -> Result<String> {
    let value = supervisor_receipt_value(input)?;
    let receipt_ref = crate::preserves_rail::canonical_hash(&value)?;
    write_preserves(&control_supervisor_receipt_path(state_root, &receipt_ref), &value)?;
    import_artifact(state_root, &value)?;
    Ok(receipt_ref)
}

fn control_ingress_envelope_path(state_root: &Path, topic: &str, envelope_ref: &str) -> PathBuf {
    state_root
        .join(CONTROL_INGRESS_DIR)
        .join(topic)
        .join(format!("{}.envelope.preserves", ref_file_stem(envelope_ref)))
}

fn write_ingress_envelope_and_verify(state_root: &Path, topic: &str, envelope: &ControlIngressEnvelope) -> Result<()> {
    let path = control_ingress_envelope_path(state_root, topic, &envelope.envelope_ref);
    write_preserves(&path, &envelope.value)?;
    let read_value = read_preserves(&path)?;
    let read_envelope = parse_control_ingress_envelope(&read_value)?;
    if read_envelope.envelope_ref != envelope.envelope_ref {
        return Err(MoltenError::invalid_harness(format!(
            "node control ingress materialized envelope ref {} does not match written {}",
            read_envelope.envelope_ref, envelope.envelope_ref
        )));
    }
    Ok(())
}

fn control_ingress_receipt_path(state_root: &Path, envelope_ref: &str, phase: &str) -> PathBuf {
    state_root.join(CONTROL_INGRESS_DIR).join("receipts").join(format!(
        "{}.{}.receipt.preserves",
        ref_file_stem(envelope_ref),
        phase
    ))
}
